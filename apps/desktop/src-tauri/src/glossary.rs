//! Glossário do projeto (`glossario.json`, formato de `producao.md` da skill):
//! termos que não se traduzem, traduções fixas, personagens e equivalências de
//! linguagem crua. Inclui a sugestão automática de termos a partir dos capítulos.

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};

pub const FILE_NAME: &str = "glossario.json";

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Character {
    pub apelidos: Vec<String>,
    pub tratamentos: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Glossary {
    pub nao_traduzir: Vec<String>,
    pub traducoes_fixas: BTreeMap<String, String>,
    pub personagens: BTreeMap<String, Character>,
    /// Termo cru do original → possíveis equivalências em PT-BR.
    pub palavroes: BTreeMap<String, Vec<String>>,
    /// Campos que o autor tenha acrescentado ao arquivo são preservados.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl Glossary {
    /// Remove entradas vazias e duplicadas (preservando a ordem).
    pub fn cleaned(mut self) -> Glossary {
        let mut seen = HashSet::new();
        self.nao_traduzir = self.nao_traduzir.into_iter().map(|t| t.trim().to_string()).filter(|t| !t.is_empty() && seen.insert(t.to_lowercase())).collect();
        self.traducoes_fixas = self.traducoes_fixas.into_iter().map(|(k, v)| (k.trim().to_string(), v.trim().to_string())).filter(|(k, v)| !k.is_empty() && !v.is_empty()).collect();
        self.personagens = self.personagens.into_iter().filter(|(name, _)| !name.trim().is_empty()).map(|(name, mut c)| {
            c.apelidos = c.apelidos.into_iter().map(|a| a.trim().to_string()).filter(|a| !a.is_empty()).collect();
            c.tratamentos = c.tratamentos.into_iter().map(|a| a.trim().to_string()).filter(|a| !a.is_empty()).collect();
            (name.trim().to_string(), c)
        }).collect();
        self.palavroes = self.palavroes.into_iter().map(|(k, v)| (k.trim().to_lowercase(), v.into_iter().map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).collect::<Vec<_>>())).filter(|(k, v)| !k.is_empty() && !v.is_empty()).collect();
        self
    }
}

/// Equivalências obrigatórias da skill (`SKILL.md`, seção 3, e `producao.md`).
pub fn default_equivalences() -> BTreeMap<String, Vec<String>> {
    let table: [(&str, &[&str]); 8] = [
        ("damn", &["porra"]),
        ("fuck", &["foder", "porra", "caralho"]),
        ("hell", &["porra", "caralho"]),
        ("shit", &["merda"]),
        ("bitch", &["vadia"]),
        ("pussy", &["buceta"]),
        ("cock", &["pau"]),
        ("badass", &["fodona", "fodão"]),
    ];
    table.iter().map(|(k, v)| (k.to_string(), v.iter().map(|s| s.to_string()).collect())).collect()
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TermCandidate {
    pub term: String,
    pub count: usize,
    /// Em quantos capítulos o termo aparece.
    pub chapters: usize,
    /// "invented" (apóstrofo, ex.: rwy'n) ou "name" (inicial maiúscula no meio da frase).
    pub kind: &'static str,
}

const STOPWORDS: [&str; 19] = ["janeiro", "fevereiro", "março", "abril", "maio", "junho", "julho", "agosto", "setembro", "outubro", "novembro", "dezembro", "segunda", "terça", "quarta", "quinta", "sexta", "sábado", "domingo"];

fn sentence_start(before: &str) -> bool {
    match before.trim_end().chars().last() {
        None => true,
        Some(c) => matches!(c, '.' | '!' | '?' | '…' | ':' | '—' | '–' | '-' | '"' | '“' | '«' | '(' | '[' | '*'),
    }
}

/// Procura nomes próprios e termos inventados que aparecem em `chapters` (um item por capítulo,
/// com um parágrafo por linha). Ignora `known` e termos com menos de `min_count` ocorrências.
pub fn suggest_terms(chapters: &[Vec<String>], known: &[String], min_count: usize) -> Vec<TermCandidate> {
    static TOKEN: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    let token = TOKEN.get_or_init(|| Regex::new(r"\p{L}[\p{L}'’\-]*").expect("regex"));
    let known: HashSet<String> = known.iter().map(|k| k.to_lowercase()).collect();
    let mut counts: HashMap<String, (usize, HashSet<usize>, &'static str)> = HashMap::new();
    for (chapter_index, lines) in chapters.iter().enumerate() {
        for line in lines {
            for found in token.find_iter(line) {
                let word = found.as_str().trim_matches(|c| c == '-' || c == '\'' || c == '’');
                let chars: Vec<char> = word.chars().collect();
                if chars.len() < 3 { continue; }
                let invented = word.contains('\'') || word.contains('’');
                let capitalized = chars[0].is_uppercase();
                if !invented && !(capitalized && !sentence_start(&line[..found.start()])) { continue; }
                // “DEPOIS” em caixa-alta, títulos: ignora palavras todas em maiúsculas.
                if !invented && chars.iter().all(|c| !c.is_lowercase()) { continue; }
                let key = word.to_string();
                if STOPWORDS.contains(&key.to_lowercase().as_str()) || known.contains(&key.to_lowercase()) { continue; }
                let entry = counts.entry(key).or_insert((0, HashSet::new(), if invented { "invented" } else { "name" }));
                entry.0 += 1;
                entry.1.insert(chapter_index);
            }
        }
    }
    let mut out: Vec<TermCandidate> = counts.into_iter().filter(|(_, (count, _, _))| *count >= min_count).map(|(term, (count, chapters, kind))| TermCandidate { term, count, chapters: chapters.len(), kind }).collect();
    out.sort_by(|a, b| b.count.cmp(&a.count).then(a.term.cmp(&b.term)));
    out.truncate(120);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn book() -> Vec<Vec<String>> {
        vec![
            vec!["Aria olhou para Kix. Depois Aria sorriu.".to_string(), "— rwy'n la, disse Kix.".to_string()],
            vec!["Quando Aria chegou, Kix já esperava. Zhrovni brilhava.".to_string(), "Segunda ela voltou a rwy'n la.".to_string()],
        ]
    }

    #[test]
    fn suggests_mid_sentence_names_and_invented_terms() {
        let found = suggest_terms(&book(), &[], 2);
        let terms: Vec<&str> = found.iter().map(|c| c.term.as_str()).collect();
        assert!(terms.contains(&"Aria") && terms.contains(&"Kix"), "{terms:?}");
        let invented = found.iter().find(|c| c.term == "rwy'n").unwrap();
        assert_eq!((invented.count, invented.chapters, invented.kind), (2, 2, "invented"));
        assert!(!terms.contains(&"Quando") && !terms.contains(&"Depois") && !terms.contains(&"Segunda"), "início de frase e dias não contam: {terms:?}");
    }

    #[test]
    fn respects_known_terms_and_minimum_count() {
        let found = suggest_terms(&book(), &["kix".to_string()], 2);
        assert!(found.iter().all(|c| c.term != "Kix"));
        assert!(found.iter().all(|c| c.term != "Zhrovni"), "só 1 ocorrência");
        assert!(suggest_terms(&[], &[], 1).is_empty());
    }

    #[test]
    fn cleaning_trims_dedupes_and_keeps_unknown_fields() {
        let raw = r#"{"nao_traduzir":[" Kix ","kix",""],"traducoes_fixas":{"my heart":" meu coração ","":"x"},"personagens":{"Aria":{"apelidos":[" Ari ",""]}},"palavroes":{"FUCK":["foder"," "]},"notas":"manter"}"#;
        let glossary: Glossary = serde_json::from_str(raw).unwrap();
        let cleaned = glossary.cleaned();
        assert_eq!(cleaned.nao_traduzir, vec!["Kix"]);
        assert_eq!(cleaned.traducoes_fixas.get("my heart").map(String::as_str), Some("meu coração"));
        assert_eq!(cleaned.personagens["Aria"].apelidos, vec!["Ari"]);
        assert_eq!(cleaned.palavroes["fuck"], vec!["foder"]);
        assert_eq!(cleaned.extra["notas"], "manter");
        assert!(serde_json::to_string(&cleaned).unwrap().contains("\"notas\":\"manter\""));
    }

    #[test]
    fn default_equivalences_come_from_the_skill_table() {
        let table = default_equivalences();
        assert_eq!(table["fuck"], vec!["foder", "porra", "caralho"]);
        assert_eq!(table["damn"], vec!["porra"]);
    }
}
