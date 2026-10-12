//! Registros por ID dentro de tabelas Markdown (`CF-20`, `FIO-14`, `LAC-03`, `PT-L1-01`…).
//!
//! O vault do autor usa tabelas cuja primeira coluna é um ID estável. Aqui elas são
//! lidas como registros estruturados SEM alterar os arquivos: o mesmo ID pode aparecer
//! em mais de uma linha (aberto e, depois, resolvido); o estado vigente é o da última.

use regex::Regex;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::OnceLock;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    pub id: String,
    /// Chave única do registro (o próprio ID, ou `nota#chave` nos registros por nome).
    pub uid: String,
    /// Registro por nome (primeira coluna sem ID, ex.: "Relógio de Jorge" no Mapa de Âncoras).
    pub keyed: bool,
    pub prefix: String,
    /// Caminho da nota (relativo ao caderno) e linha (1-based) onde o registro está.
    pub note: String,
    pub line: usize,
    pub headers: Vec<String>,
    pub cells: Vec<String>,
    /// "aberto", "resolvido" ou "supersedido" quando o texto permite inferir.
    pub status: Option<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordSummary {
    pub id: String,
    pub uid: String,
    pub keyed: bool,
    pub prefix: String,
    /// Texto curto do registro (segunda coluna da primeira ocorrência).
    pub title: String,
    pub note: String,
    pub line: usize,
    /// Estado da última linha que declara um estado.
    pub status: Option<&'static str>,
    pub occurrences: usize,
}

fn id_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^[A-Z]{2,5}(?:-[A-Z0-9]{1,4}){0,2}-\d{1,3}$").expect("regex"))
}

fn clean_cell(cell: &str) -> String {
    cell.trim().trim_matches('*').trim().trim_matches('`').trim().to_string()
}

fn split_row(line: &str) -> Option<Vec<String>> {
    let t = line.trim();
    if !t.starts_with('|') { return None; }
    let inner = t.trim_start_matches('|');
    let inner = inner.strip_suffix('|').unwrap_or(inner);
    Some(inner.split('|').map(|c| c.trim().to_string()).collect())
}

fn is_separator(cells: &[String]) -> bool {
    !cells.is_empty() && cells.iter().all(|c| { let t = c.trim(); t.len() >= 3 && t.chars().all(|ch| ch == '-' || ch == ':') })
}

/// Infere o estado a partir do texto da última coluna (Tratamento/Estado/Resolução).
/// É uma sugestão: a interface mostra o estado como "inferido" e o autor pode corrigi-lo.
pub fn infer_status(cells: &[String]) -> Option<&'static str> {
    let last = cells.last()?.to_lowercase().replace("**", "");
    let text = last.trim();
    if text.starts_with("resolvid") { return Some("resolvido"); }
    if text.starts_with("aberto") || text.starts_with("em aberto") { return Some("aberto"); }
    let head: String = text.chars().take(240).collect();
    if head.contains("supersedid") && !text.contains("resolvid") { return Some("supersedido"); }
    // "Resolvido" em qualquer ponto vence (ex.: "…; CF-40 resolvido editorialmente; mistérios continuam abertos").
    if text.contains("resolvid") || head.starts_with("corrigid") || head.contains("resolvido") || text.starts_with("adotada") || head.contains("corrigido ") { return Some("resolvido"); }
    if head.contains("aberto") || head.contains("lacuna") || text.contains("continua sem") || text.contains("nenhuma cena esclarece") { return Some("aberto"); }
    None
}

/// Extrai os registros de uma nota. Linhas sem cabeçalho próprio herdam o último da nota.
pub fn parse(note: &str, body: &str) -> Vec<Record> {
    let mut out = Vec::new();
    let mut headers: Vec<String> = Vec::new();
    let mut previous: Option<Vec<String>> = None;
    for (index, line) in body.lines().enumerate() {
        let Some(cells) = split_row(line) else { previous = None; continue };
        if is_separator(&cells) {
            if let Some(header) = previous.take() { headers = header.iter().map(|c| clean_cell(c)).collect(); }
            continue;
        }
        let first = cells.first().map(|c| clean_cell(c)).unwrap_or_default();
        if id_pattern().is_match(&first) {
            let status = infer_status(&cells);
            out.push(Record { prefix: first.split('-').next().unwrap_or("").to_string(), uid: first.clone(), keyed: false, id: first, note: note.to_string(), line: index + 1, headers: headers.clone(), cells: cells.clone(), status });
        }
        previous = Some(cells);
    }
    out
}

/// Registros por NOME: tabelas de uma nota do tipo Registro cuja primeira coluna não é um ID
/// (Mapa de Âncoras, Verificação de Pontas Soltas, Matriz de Relações…). `label` agrupa na interface.
pub fn parse_keyed(note: &str, label: &str, body: &str) -> Vec<Record> {
    let rows: Vec<Option<Vec<String>>> = body.lines().map(split_row).collect();
    let mut out = Vec::new();
    let mut headers: Vec<String> = Vec::new();
    for (i, row) in rows.iter().enumerate() {
        let Some(cells) = row else { continue };
        if is_separator(cells) { continue; }
        let next_is_separator = rows.get(i + 1).and_then(|r| r.as_ref()).map(|c| is_separator(c)).unwrap_or(false);
        if next_is_separator { headers = cells.iter().map(|c| clean_cell(c)).collect(); continue; }
        if i > 0 && rows[i - 1].is_none() { headers.clear(); } // linha solta depois de texto: sem tabela
        let key = cells.first().map(|c| clean_cell(c)).unwrap_or_default();
        if headers.is_empty() || cells.len() < 2 || key.is_empty() || id_pattern().is_match(&key) { continue; }
        let status = infer_status(cells);
        out.push(Record { id: key.clone(), uid: format!("{note}#{key}"), keyed: true, prefix: label.chars().take(40).collect(), note: note.to_string(), line: i + 1, headers: headers.clone(), cells: cells.clone(), status });
    }
    out
}

/// Um item por ID, na ordem da primeira aparição.
pub fn summarize(records: &[Record]) -> Vec<RecordSummary> {
    let mut order: Vec<String> = Vec::new();
    let mut by_uid: HashMap<&str, Vec<&Record>> = HashMap::new();
    for record in records {
        if !by_uid.contains_key(record.uid.as_str()) { order.push(record.uid.clone()); }
        by_uid.entry(&record.uid).or_default().push(record);
    }
    order.into_iter().map(|uid| {
        let rows = &by_uid[uid.as_str()];
        let first = rows[0];
        let last = rows[rows.len() - 1];
        let title: String = first.cells.get(1).map(|c| clean_cell(c)).unwrap_or_default().chars().take(200).collect();
        RecordSummary { id: first.id.clone(), keyed: first.keyed, prefix: first.prefix.clone(), title, note: last.note.clone(), line: last.line, status: rows.iter().rev().find_map(|r| r.status), occurrences: rows.len(), uid }
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "# Registro\n\n| ID | Conflito | Fontes | Tratamento atual |\n|---|---|---|---|\n| CF-01 | L3 aparece como planejado | sessões | manter “declarado” |\n| CF-08 | Saída Kindle | HTML v5 | **Resolvido por decisão autoral em 14 set. 2026:** restaurada |\n\n| CF-30 | C31 descreve Lucas | C31 × C33 | aberto: vínculos podem ser cumulativos |\n\nTexto solto.\n\n| ID | Questão | Fontes | Estado |\n|---|---|---|---|\n| CF-34 | C35 manda Caio correr | C35 e C36 | aberto para revisão autoral |\n| CF-35 | Verônica com 45 anos | C35 | conflito de duração/idade aberto. Preservar |\n\n| ID | Decisão e aplicação |\n|---|---|\n| CF-34 | Resolvido por continuidade local em C36 |\n| CF-35 | Resolvido pela cronologia/fichas |\n| CF-49 | |\n";

    #[test]
    fn parses_id_rows_and_infers_status() {
        let records = parse("07-Revisao/Registro.md", SAMPLE);
        let ids: Vec<&str> = records.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, vec!["CF-01", "CF-08", "CF-30", "CF-34", "CF-35", "CF-34", "CF-35", "CF-49"]);
        assert_eq!(records[0].status, None, "sem palavra de estado");
        assert_eq!(records[1].status, Some("resolvido"));
        assert_eq!(records[1].headers, vec!["ID", "Conflito", "Fontes", "Tratamento atual"]);
        assert_eq!(records[2].status, Some("aberto"));
        assert_eq!(records[2].headers.len(), 4, "linha sem cabeçalho herda o anterior da nota");
        assert_eq!(records[2].line, 8);
    }

    #[test]
    fn latest_row_decides_the_current_status() {
        let summaries = summarize(&parse("n.md", SAMPLE));
        let cf34 = summaries.iter().find(|s| s.id == "CF-34").unwrap();
        assert_eq!((cf34.status, cf34.occurrences), (Some("resolvido"), 2));
        assert_eq!(cf34.title, "C35 manda Caio correr");
        assert_eq!(summaries.iter().find(|s| s.id == "CF-30").unwrap().status, Some("aberto"));
        assert_eq!(summaries.len(), 6, "CF-01, 08, 30, 34, 35 e 49");
    }

    #[test]
    fn named_tables_become_keyed_records_without_header_rows_or_id_rows() {
        let body = "# Mapa de Âncoras\n\n| Âncora | L1 | L2 | O que deve verificar |\n|---|---|---|---|\n| Relógio de Jorge | mesinha | gaveta | não misturar |\n| Anel de Rafael | clareira | “um dia” | cena de resolução em aberto |\n\n| ID | Lacuna |\n|---|---|\n| LAC-01 | x |\n| Caso 38 | y |\n\nlinha solta\n| Solta | z |\n";
        let keyed = parse_keyed("06/Mapa.md", "Mapa de Âncoras", body);
        let keys: Vec<&str> = keyed.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(keys, vec!["Relógio de Jorge", "Anel de Rafael", "Caso 38"], "sem cabeçalho, sem ID e sem linha solta");
        assert!(keyed.iter().all(|r| r.keyed && r.prefix == "Mapa de Âncoras"));
        assert_eq!(keyed[0].headers, vec!["Âncora", "L1", "L2", "O que deve verificar"]);
        assert_eq!(keyed[1].status, Some("aberto"));
        assert_eq!(keyed[0].uid, "06/Mapa.md#Relógio de Jorge");
        // O mesmo nome em outra nota é outro registro.
        let other = parse_keyed("06/Verificacao.md", "Verificação", "| Fio | Resultado |\n|---|---|\n| Relógio de Jorge | forte |\n");
        let all: Vec<Record> = keyed.into_iter().chain(other).collect();
        assert_eq!(summarize(&all).len(), 4);
    }

    #[test]
    fn recognises_the_id_families_of_the_vault() {
        let body = "|ID|x|\n|---|---|\n| FIO-14 | caderno laranja |\n| LAC-03 | lacuna |\n| PT-L1-01 | tumor | \n| **CF-20** | negrito |\n| Caso 38 | não é ID |\n| 12 | número |\n";
        let ids: Vec<String> = parse("m.md", body).into_iter().map(|r| r.id).collect();
        assert_eq!(ids, vec!["FIO-14", "LAC-03", "PT-L1-01", "CF-20"]);
    }

    #[test]
    fn status_words_cover_the_authors_phrasing() {
        let s = |t: &str| infer_status(&[ "x".to_string(), t.to_string()]);
        assert_eq!(s("**Resolvido editorialmente em 14 set.**"), Some("resolvido"));
        assert_eq!(s("Resolvida a estrutura (decisão autoral)"), Some("resolvido"));
        assert_eq!(s("Em aberto, preservado e transferido"), Some("aberto"));
        assert_eq!(s("As leituras anteriores ficam supersedidas."), Some("supersedido"));
        assert_eq!(s("manter “declarado; conferir narrativa final”"), None);
        assert_eq!(s("corrigido no Kindle e `ch38.xhtml` para “dezessete anos”"), Some("resolvido"));
        assert_eq!(s("adotada a sequência do Kindle"), Some("resolvido"));
        assert_eq!(s("manter como lacuna crítica"), Some("aberto"));
        assert_eq!(s("Corrigidos em 21 parágrafos; CF-40 resolvido editorialmente; mistérios continuam abertos."), Some("resolvido"));
        assert_eq!(s("Referente do GoPro continua sem explicação narrativa"), Some("aberto"));
    }

    /// Diagnóstico opt-in com arquivos reais do autor: LIVRO_FIXTURE_DIR=<pasta com os .md>.
    #[test]
    #[ignore]
    fn real_vault_files_report() {
        let dir = std::env::var("LIVRO_FIXTURE_DIR").expect("defina LIVRO_FIXTURE_DIR");
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") { continue; }
            let body = std::fs::read_to_string(&path).unwrap();
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            let records = parse(&name, &body);
            let summaries = summarize(&records);
            let count = |s: &str| summaries.iter().filter(|x| x.status == Some(s)).count();
            println!("{name}: {} linhas, {} IDs, abertos={}, resolvidos={}, supersedidos={}, sem estado={}", records.len(), summaries.len(), count("aberto"), count("resolvido"), count("supersedido"), summaries.iter().filter(|x| x.status.is_none()).count());
            let prefixes: std::collections::BTreeMap<&str, usize> = summaries.iter().fold(Default::default(), |mut m, x| { *m.entry(x.prefix.as_str()).or_default() += 1; m });
            println!("  prefixos: {prefixes:?}");
            let dup: Vec<&str> = summaries.iter().filter(|x| x.occurrences > 1).map(|x| x.id.as_str()).collect();
            println!("  IDs com mais de uma linha: {dup:?}");
        }
    }
}
