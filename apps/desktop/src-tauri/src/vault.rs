//! Caderno do livro: notas Markdown compatíveis com o Obsidian
//! (`[[wikilinks]]`, frontmatter YAML opcional). O caderno pode ser uma pasta do
//! projeto (padrão `.livro-notas/`) OU um vault Obsidian existente, inclusive
//! compartilhado entre os livros de uma série. Notas já existentes NUNCA são
//! reescritas só por serem lidas: o app não exige frontmatter nem uma estrutura
//! de pastas própria.

use crate::project_files;
use crate::records::{self, Record, RecordSummary};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::sync::OnceLock;
use std::time::UNIX_EPOCH;

/// Pasta padrão do caderno dentro do projeto.
pub const DEFAULT_DIR: &str = ".livro-notas";
const MAX_NOTES: usize = 5000;
const MAX_NOTE_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum Kind { Personagem, Lugar, Fio, Evento, Mundo, Capitulo, Biblia, Registro, Nota }

impl Kind {
    pub const ALL: [Kind; 9] = [Kind::Personagem, Kind::Lugar, Kind::Fio, Kind::Evento, Kind::Mundo, Kind::Capitulo, Kind::Biblia, Kind::Registro, Kind::Nota];
    pub fn folder(self) -> &'static str {
        match self { Kind::Personagem => "Personagens", Kind::Lugar => "Lugares", Kind::Fio => "Fios", Kind::Evento => "Eventos", Kind::Mundo => "Mundo", Kind::Capitulo => "Capitulos", Kind::Biblia => "Biblias", Kind::Registro => "Registros", Kind::Nota => "Notas" }
    }
    pub fn id(self) -> &'static str {
        match self { Kind::Personagem => "personagem", Kind::Lugar => "lugar", Kind::Fio => "fio", Kind::Evento => "evento", Kind::Mundo => "mundo", Kind::Capitulo => "capitulo", Kind::Biblia => "biblia", Kind::Registro => "registro", Kind::Nota => "nota" }
    }
    fn from_id(id: &str) -> Option<Kind> { Kind::ALL.into_iter().find(|k| k.id() == id.trim().to_lowercase()) }

    /// Modelo de nota nova (baseado em `criacao.md`, `SKILL.md` e `revisao-sistemica.md`).
    pub fn template(self, title: &str) -> String {
        let head = format!("---\ntipo: {}\naliases: []\n{}---\n# {title}\n\n", self.id(), if self == Kind::Fio { "estado: aberto\n" } else { "" });
        let body = match self {
            Kind::Personagem => "## Ficha técnica\n\n| Campo | Registro |\n|---|---|\n| Nome | |\n| Idade | |\n| Data de nascimento | |\n| Características | |\n| Personalidade | |\n\n## Vida própria\n\n**Desejo:** \n**Medo:** \n**Segredo:** \n**Contradição:** \n**Relação com a protagonista:** \n**Arco:** \n**Ferida central:** \n**Necessidade inconsciente:** \n**Distorção cognitiva:** \n**Vício de linguagem:** \n**O que evita dizer:** \n**Sob pressão máxima:** \n\n## Aparições e momentos\n\n## Arco e conexões\n",
            Kind::Lugar => "**Descrição sensorial:** \n**Regras / o que acontece aqui:** \n\n## Aparições\n",
            Kind::Fio => "**Descrição do fio:** \n**Aberto em:** \n**Fechado em:** \n\n## Desenvolvimento\n",
            Kind::Evento => "**Quando (ordem cronológica):** \n**Onde:** \n**Quem sabe disso:** \n**Consequências:** \n",
            Kind::Mundo => "**Regra:** \n**Custo físico:** \n**Custo emocional:** \n**Custo narrativo:** \n**Limites (o que NÃO pode fazer):** \n",
            Kind::Capitulo => "## Resumo\n",
            Kind::Biblia => "## Linha do tempo\n\n## Personagens\n\n## Mundo e regras\n\n## Fios e âncoras\n",
            Kind::Registro => "| ID | Descrição | Fontes | Estado |\n|---|---|---|---|\n",
            Kind::Nota => "",
        };
        format!("{head}{body}")
    }
}

/// Associa pastas do caderno a tipos de nota (prefixo de pasta → tipo).
#[derive(Debug, Clone, Default)]
pub struct KindMap { entries: Vec<(String, Kind)> }

fn normalize_folder(folder: &str) -> String { folder.replace('\\', "/").trim_matches('/').to_lowercase() }

impl KindMap {
    pub fn from_entries(entries: &BTreeMap<String, String>) -> KindMap {
        let mut list: Vec<(String, Kind)> = entries.iter().filter_map(|(folder, kind)| Some((normalize_folder(folder), Kind::from_id(kind)?))).filter(|(f, _)| !f.is_empty()).collect();
        // Prefixos mais específicos primeiro.
        list.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
        KindMap { entries: list }
    }

    /// Tipo pela pasta da nota (mapeamento do autor; depois os nomes padrão do app).
    pub fn kind_for(&self, rel: &str) -> Option<Kind> {
        let lower = normalize_folder(rel);
        if let Some((_, kind)) = self.entries.iter().find(|(folder, _)| lower == *folder || lower.starts_with(&format!("{folder}/"))) { return Some(*kind); }
        let first = rel.split('/').next()?;
        Kind::ALL.into_iter().find(|k| k.folder() == first)
    }

    /// Pasta onde criar notas novas de um tipo: a mais específica mapeada, ou o nome padrão.
    pub fn folder_for(&self, kind: Kind) -> String {
        self.entries.iter().find(|(_, k)| *k == kind).map(|(folder, _)| folder.clone()).unwrap_or_else(|| kind.folder().to_string())
    }
}

/// Sugere um mapeamento olhando os nomes das pastas do vault (ex.: `04-Personagens`).
pub fn detect_mapping(dir: &str) -> BTreeMap<String, String> {
    fn guess(name: &str) -> Option<Kind> {
        // Compara por palavra ("trilogia" não é "log"); prefixos só para palavras longas.
        let folded = fold(name);
        let tokens: Vec<&str> = folded.split(|c: char| !c.is_alphanumeric()).filter(|t| !t.is_empty()).collect();
        let has = |words: &[&str]| words.iter().any(|w| tokens.iter().any(|t| if w.len() <= 3 { t == w || *t == format!("{w}s") } else { t.starts_with(w) }));
        if has(&["personag", "ficha", "elenco"]) { Some(Kind::Personagem) }
        else if has(&["biblia"]) { Some(Kind::Biblia) }
        else if has(&["confli", "revisao", "plot", "twist", "mapa", "registro", "fios", "ancora"]) { Some(Kind::Registro) }
        else if has(&["log", "capitulo", "recap"]) { Some(Kind::Capitulo) }
        else if has(&["lugar", "local"]) { Some(Kind::Lugar) }
        else if has(&["mundo", "regra", "cronologia", "universo"]) { Some(Kind::Mundo) }
        else if has(&["evento", "timeline"]) { Some(Kind::Evento) }
        else { None }
    }
    let mut out = BTreeMap::new();
    fn walk(base: &std::path::Path, rel: &str, depth: usize, out: &mut BTreeMap<String, String>) {
        let Ok(entries) = fs::read_dir(base) else { return };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') || !entry.path().is_dir() { continue; }
            let relative = if rel.is_empty() { name.clone() } else { format!("{rel}/{name}") };
            if let Some(kind) = guess(&name) { out.insert(relative.clone(), kind.id().to_string()); }
            else if depth < 2 { walk(&entry.path(), &relative, depth + 1, out); }
            // "Fichas" dentro de uma pasta de personagens vira o destino de notas novas.
            if depth < 2 && guess(&name) == Some(Kind::Personagem) {
                if let Ok(inner) = fs::read_dir(entry.path()) { for sub in inner.flatten() { let s = sub.file_name().to_string_lossy().into_owned(); if sub.path().is_dir() && fold(&s).contains("ficha") { out.insert(format!("{relative}/{s}"), "personagem".to_string()); } } }
            }
        }
    }
    walk(std::path::Path::new(dir), "", 0, &mut out);
    out
}

/// Minúsculas sem acento, para comparar “Íris” com “Iris-Barbosa”.
pub fn fold(text: &str) -> String {
    text.to_lowercase().chars().map(|c| match c {
        'á' | 'à' | 'â' | 'ã' | 'ä' => 'a', 'é' | 'è' | 'ê' | 'ë' => 'e', 'í' | 'ì' | 'î' | 'ï' => 'i',
        'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o', 'ú' | 'ù' | 'û' | 'ü' => 'u', 'ç' => 'c', 'ñ' => 'n', other => other,
    }).collect()
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteMeta {
    /// Primeiro título `# …` da nota (ex.: "Eugênio Castelo" num arquivo `Eugenio-Castelo.md`), para exibir.
    pub heading: Option<String>,
    /// Caminho relativo ao caderno (ex.: `04-Personagens/Fichas/Iris-Barbosa.md`).
    pub path: String,
    pub title: String,
    pub kind: Kind,
    pub aliases: Vec<String>,
    pub words: usize,
    pub modified: u64,
    /// Para fios: "aberto" ou "fechado".
    pub state: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Edge { pub from: String, pub to: String }

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Unresolved { pub from: String, pub target: String }

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultIndex {
    pub exists: bool,
    pub notes: Vec<NoteMeta>,
    pub edges: Vec<Edge>,
    pub unresolved: Vec<Unresolved>,
    /// Um item por ID de registro (`CF-20`…), com o estado vigente.
    pub records: Vec<RecordSummary>,
    /// Todas as linhas de registro (um mesmo ID pode ter várias).
    pub record_rows: Vec<Record>,
}

fn wikilink() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\[\[([^\[\]\n|#]+)(#[^\[\]\n|]*)?(\|[^\[\]\n]*)?\]\]").expect("regex"))
}

pub fn split_frontmatter(text: &str) -> (Option<&str>, &str) {
    let Some(rest) = text.strip_prefix("---\n").or_else(|| text.strip_prefix("---\r\n")) else { return (None, text) };
    match rest.find("\n---") {
        Some(end) => {
            let after = &rest[end + 4..];
            (Some(&rest[..end]), after.strip_prefix("\r\n").or_else(|| after.strip_prefix('\n')).unwrap_or(after))
        }
        None => (None, text),
    }
}

fn frontmatter_value<'a>(front: &'a str, key: &str) -> Option<&'a str> {
    front.lines().find_map(|line| line.strip_prefix(key).and_then(|rest| rest.strip_prefix(':')).map(str::trim))
}

fn parse_aliases(front: &str) -> Vec<String> {
    let mut aliases = Vec::new();
    let mut lines = front.lines().peekable();
    while let Some(line) = lines.next() {
        let Some(rest) = line.strip_prefix("aliases:") else { continue };
        let rest = rest.trim();
        if let Some(inline) = rest.strip_prefix('[').and_then(|r| r.strip_suffix(']')) {
            aliases.extend(inline.split(',').map(|a| a.trim().trim_matches(['"', '\'']).to_string()).filter(|a| !a.is_empty()));
        } else if rest.is_empty() {
            while let Some(next) = lines.peek() {
                match next.trim_start().strip_prefix("- ") { Some(item) => { aliases.push(item.trim().trim_matches(['"', '\'']).to_string()); lines.next(); } None => break }
            }
        }
    }
    aliases
}

/// Define `key: value` no frontmatter (cria o bloco se faltar).
pub fn set_frontmatter(text: &str, key: &str, value: &str) -> String {
    let (front, body) = split_frontmatter(text);
    let new_line = format!("{key}: {value}");
    match front {
        Some(front) => {
            let mut replaced = false;
            let mut lines: Vec<String> = front.lines().map(|line| if line.starts_with(&format!("{key}:")) { replaced = true; new_line.clone() } else { line.to_string() }).collect();
            if !replaced { lines.push(new_line); }
            format!("---\n{}\n---\n{body}", lines.join("\n"))
        }
        None => format!("---\n{new_line}\n---\n{body}"),
    }
}

/// Acrescenta apelidos (`aliases`) ao frontmatter sem duplicar; cria o bloco se a nota não tiver.
pub fn add_aliases(text: &str, new: &[String]) -> String {
    let (front, body) = split_frontmatter(text);
    let existing: Vec<String> = front.map(parse_aliases).unwrap_or_default();
    let mut all = existing.clone();
    for alias in new { let a = alias.trim(); if !a.is_empty() && !all.iter().any(|e| fold(e) == fold(a)) { all.push(a.to_string()); } }
    if all.len() == existing.len() { return text.to_string(); }
    let quote = |a: &String| if a.contains([',', ':', '[', ']', '#']) { format!("\"{}\"", a.replace('"', "'")) } else { a.clone() };
    let inline = format!("[{}]", all.iter().map(quote).collect::<Vec<_>>().join(", "));
    let Some(front) = front else { return format!("---\naliases: {inline}\n---\n{body}"); };
    // Lista em bloco ("aliases:" seguido de "- item"): acrescenta itens e mantém o estilo.
    let lines: Vec<&str> = front.lines().collect();
    if let Some(at) = lines.iter().position(|l| l.trim_end() == "aliases:") {
        let mut end = at + 1;
        while end < lines.len() && lines[end].trim_start().starts_with("- ") { end += 1; }
        if end > at + 1 {
            let mut out: Vec<String> = lines[..end].iter().map(|l| l.to_string()).collect();
            for alias in &all[existing.len()..] { out.push(format!("  - {}", quote(alias))); }
            out.extend(lines[end..].iter().map(|l| l.to_string()));
            return format!("---\n{}\n---\n{body}", out.join("\n"));
        }
    }
    set_frontmatter(text, "aliases", &inline)
}

pub fn sanitize_title(title: &str) -> Result<String, String> {
    let cleaned: String = title.chars().filter(|c| !matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '#' | '^' | '[' | ']') && !c.is_control()).collect();
    let cleaned = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    let cleaned = cleaned.trim_matches('.').trim().to_string();
    if cleaned.is_empty() { return Err("Informe um título para a nota.".to_string()); }
    Ok(cleaned.chars().take(100).collect())
}

/// Arquivos de navegação (índice, matriz, mapa) não são entidades mesmo dentro da pasta delas.
fn navigation_kind(stem: &str, from_folder: Kind) -> Kind {
    if !matches!(from_folder, Kind::Personagem | Kind::Lugar | Kind::Fio | Kind::Evento | Kind::Mundo) { return from_folder; }
    let folded = fold(&stem.replace(['-', '_'], " "));
    let first = folded.split_whitespace().next().unwrap_or("");
    match first { "indice" | "index" | "readme" | "moc" | "sumario" => Kind::Nota, "matriz" | "mapa" | "quadro" => Kind::Registro, _ => from_folder }
}

struct Loaded { meta: NoteMeta, body: String, links: Vec<String> }

/// Resolve alvos de `[[wikilinks]]`: por caminho (`../04-Personagens/Fichas/Iris`), título ou apelido.
struct Resolver { by_title: HashMap<String, String>, by_path: Vec<(String, String)> }

impl Resolver {
    fn new(notes: &[Loaded]) -> Resolver {
        let mut by_title: HashMap<String, String> = HashMap::new();
        for note in notes { by_title.entry(note.meta.title.to_lowercase()).or_insert_with(|| note.meta.path.clone()); }
        for note in notes { for alias in &note.meta.aliases { by_title.entry(alias.to_lowercase()).or_insert_with(|| note.meta.path.clone()); } }
        let by_path = notes.iter().map(|n| (n.meta.path.trim_end_matches(".md").to_lowercase(), n.meta.path.clone())).collect();
        Resolver { by_title, by_path }
    }

    fn resolve(&self, target: &str) -> Option<&String> {
        let mut t = target.trim().replace('\\', "/").to_lowercase();
        while let Some(rest) = t.strip_prefix("../").or_else(|| t.strip_prefix("./")) { t = rest.to_string(); }
        let t = t.trim_end_matches(".md").trim_matches('/').to_string();
        if t.contains('/') {
            if let Some((_, path)) = self.by_path.iter().find(|(p, _)| *p == t) { return Some(path); }
            if let Some((_, path)) = self.by_path.iter().find(|(p, _)| p.ends_with(&format!("/{t}"))) { return Some(path); }
        }
        let last = t.rsplit('/').next().unwrap_or(&t);
        self.by_title.get(&t).or_else(|| self.by_title.get(last))
    }
}

/// Um caderno de notas numa pasta (dentro ou fora do projeto).
pub struct Vault { dir: String, map: KindMap }

impl Vault {
    pub fn new(dir: &str, map: KindMap) -> Vault { Vault { dir: dir.to_string(), map } }
    #[cfg(test)]
    pub fn plain(dir: &str) -> Vault { Vault::new(dir, KindMap::default()) }
    pub fn dir(&self) -> &str { &self.dir }
    #[cfg(test)]
    pub fn map(&self) -> &KindMap { &self.map }

    fn exists(&self) -> bool { std::path::Path::new(&self.dir).is_dir() }

    fn note_path(&self, rel: &str) -> Result<std::path::PathBuf, String> {
        if !rel.ends_with(".md") { return Err("Só notas .md são permitidas.".to_string()); }
        fs::create_dir_all(&self.dir).map_err(|e| format!("Não foi possível criar a pasta do caderno: {e}"))?;
        project_files::resolve(&self.dir, rel)
    }

    fn load_all(&self) -> Result<Vec<Loaded>, String> {
        let base = std::path::PathBuf::from(&self.dir);
        if !base.is_dir() { return Ok(Vec::new()); }
        let mut out = Vec::new();
        let mut stack = vec![(base, String::new())];
        while let Some((dir, prefix)) = stack.pop() {
            for entry in fs::read_dir(&dir).map_err(|e| format!("Não foi possível ler o caderno: {e}"))? {
                let entry = entry.map_err(|e| e.to_string())?;
                let name = entry.file_name().to_string_lossy().into_owned();
                let meta = fs::symlink_metadata(entry.path()).map_err(|e| e.to_string())?;
                if meta.file_type().is_symlink() || name.starts_with('.') { continue; }
                let rel = if prefix.is_empty() { name.clone() } else { format!("{prefix}/{name}") };
                if meta.is_dir() { stack.push((entry.path(), rel)); continue; }
                if !name.ends_with(".md") || meta.len() > MAX_NOTE_BYTES || out.len() >= MAX_NOTES { continue; }
                let text = fs::read_to_string(entry.path()).map_err(|e| format!("Não foi possível ler {rel}: {e}"))?;
                let (front, body) = split_frontmatter(&text);
                let stem = name.trim_end_matches(".md");
                let declared = front.and_then(|f| frontmatter_value(f, "tipo")).and_then(Kind::from_id);
                // Índices e matrizes dentro da pasta de personagens/lugares não são personagens/lugares.
                let kind = declared.or_else(|| self.map.kind_for(&rel).map(|k| navigation_kind(stem, k))).unwrap_or(Kind::Nota);
                let links = wikilink().captures_iter(body).map(|c| c[1].trim().to_string()).collect();
                let modified = meta.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0);
                out.push(Loaded {
                    meta: NoteMeta { heading: body.lines().find_map(|l| l.strip_prefix("# ").map(|t| t.trim().to_string())).filter(|t| !t.is_empty()), title: rel.rsplit('/').next().unwrap_or(&rel).trim_end_matches(".md").to_string(), path: rel, kind, aliases: front.map(parse_aliases).unwrap_or_default(), words: body.split_whitespace().count(), modified, state: front.and_then(|f| frontmatter_value(f, "estado")).map(str::to_string) },
                    body: body.to_string(), links,
                });
            }
        }
        out.sort_by(|a, b| (a.meta.kind, a.meta.title.to_lowercase()).cmp(&(b.meta.kind, b.meta.title.to_lowercase())));
        Ok(out)
    }

    /// Todas as notas com o texto bruto (inclui frontmatter), para operações em lote.
    pub fn raw_notes(&self) -> Result<Vec<(NoteMeta, String)>, String> {
        let mut out = Vec::new();
        for note in self.load_all()? { let text = self.read_note(&note.meta.path)?; out.push((note.meta, text)); }
        Ok(out)
    }

    pub fn index(&self) -> Result<VaultIndex, String> {
        let notes = self.load_all()?;
        let resolver = Resolver::new(&notes);
        let (mut edges, mut unresolved, mut seen) = (Vec::new(), Vec::new(), HashSet::new());
        let mut rows: Vec<Record> = Vec::new();
        for note in &notes {
            for target in &note.links {
                match resolver.resolve(target) {
                    Some(to) if *to != note.meta.path => { if seen.insert((note.meta.path.clone(), to.clone())) { edges.push(Edge { from: note.meta.path.clone(), to: to.clone() }); } }
                    Some(_) => {}
                    None => unresolved.push(Unresolved { from: note.meta.path.clone(), target: target.clone() }),
                }
            }
            rows.extend(records::parse(&note.meta.path, &note.body));
        }
        let summaries = records::summarize(&rows);
        Ok(VaultIndex { exists: self.exists(), notes: notes.into_iter().map(|n| n.meta).collect(), edges, unresolved, records: summaries, record_rows: rows })
    }

    pub fn read_note(&self, rel: &str) -> Result<String, String> {
        let path = self.note_path(rel)?;
        if fs::metadata(&path).map(|m| m.len()).unwrap_or(0) > MAX_NOTE_BYTES { return Err("A nota é grande demais.".to_string()); }
        fs::read_to_string(&path).map_err(|e| format!("Não foi possível ler a nota: {e}"))
    }

    pub fn write_note(&self, rel: &str, content: &str) -> Result<(), String> {
        if content.len() as u64 > MAX_NOTE_BYTES { return Err("A nota passa de 1 MB.".to_string()); }
        project_files::write_atomic(&self.note_path(rel)?, content.as_bytes())
    }

    pub fn create_note(&self, kind: Kind, title: &str) -> Result<String, String> {
        let title = sanitize_title(title)?;
        let rel = format!("{}/{title}.md", self.map.folder_for(kind));
        let path = self.note_path(&rel)?;
        if path.exists() { return Err(format!("Já existe uma nota “{title}” em {}.", self.map.folder_for(kind))); }
        project_files::write_atomic(&path, kind.template(&title).as_bytes())?;
        Ok(rel)
    }

    /// Caminho de uma nota nova do tipo dado (falha se já existir).
    pub fn new_note_path(&self, kind: Kind, title: &str) -> Result<String, String> {
        let title = sanitize_title(title)?;
        let rel = format!("{}/{title}.md", self.map.folder_for(kind));
        if self.note_path(&rel)?.exists() { return Err(format!("Já existe uma nota “{title}” em {}.", self.map.folder_for(kind))); }
        Ok(rel)
    }

    /// Grava uma nota que ainda não existe (nunca sobrescreve).
    pub fn write_new(&self, rel: &str, content: &str) -> Result<(), String> {
        let path = self.note_path(rel)?;
        if path.exists() { return Err("A nota já existe.".to_string()); }
        project_files::write_atomic(&path, content.as_bytes())
    }

    pub fn delete_note(&self, rel: &str) -> Result<(), String> {
        let path = self.note_path(rel)?;
        fs::remove_file(&path).map_err(|e| format!("Não foi possível excluir a nota: {e}"))
    }

    /// Planeja a renomeação sem tocar em disco: devolve como reescrever os links de qualquer nota.
    pub fn plan_rename(&self, rel: &str, new_title: &str) -> Result<RenamePlan, String> {
        let new_title = sanitize_title(new_title)?;
        let folder = rel.rsplit_once('/').map(|(f, _)| f).unwrap_or("");
        let new_rel = if folder.is_empty() { format!("{new_title}.md") } else { format!("{folder}/{new_title}.md") };
        let (old, new) = (self.note_path(rel)?, self.note_path(&new_rel)?);
        if !old.is_file() { return Err("A nota não existe mais.".to_string()); }
        if new.exists() && new_rel.to_lowercase() != rel.to_lowercase() { return Err(format!("Já existe uma nota “{new_title}”.")); }
        let old_title = rel.rsplit('/').next().unwrap_or(rel).trim_end_matches(".md").to_string();
        let resolver = Resolver::new(&self.load_all()?);
        Ok(RenamePlan { old_rel: rel.to_string(), new_rel, old_title, new_title, resolver })
    }

    /// Renomeia a nota e reescreve os `[[links]]` que apontavam para ela pelo título (como o Obsidian).
    pub fn rename_note(&self, rel: &str, new_title: &str) -> Result<RenameResult, String> {
        let plan = self.plan_rename(rel, new_title)?;
        let before = self.load_all()?;
        fs::rename(self.note_path(rel)?, self.note_path(&plan.new_rel)?).map_err(|e| format!("Não foi possível renomear: {e}"))?;
        let mut updated = 0;
        for note in &before {
            let path = if note.meta.path == rel { plan.new_rel.clone() } else { note.meta.path.clone() };
            let current = self.read_note(&path)?;
            let rewritten = plan.rewrite(&current, path == plan.new_rel);
            if rewritten != current { self.write_note(&path, &rewritten)?; updated += 1; }
        }
        Ok(RenameResult { new_path: plan.new_rel, updated_notes: updated })
    }

    /// Notas-entidade (personagem, lugar, fio…) citadas em um texto, da mais para a menos citada.
    pub fn references(&self, lines: &[String]) -> Result<Vec<Reference>, String> {
        let text = fold(&lines.join("\n"));
        let mut out: Vec<Reference> = self.load_all()?.into_iter().filter(|n| is_entity(n.meta.kind)).filter_map(|n| {
            let count: usize = variants(&n.meta).iter().map(|name| count_word(&text, name)).sum();
            (count > 0).then(|| Reference { path: n.meta.path, title: n.meta.title, heading: n.meta.heading, kind: n.meta.kind, count })
        }).collect();
        out.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.title.cmp(&b.title)));
        Ok(out)
    }

    /// Dossiê de uma nota: conteúdo, ligações, registros por ID, seções da bíblia e outras notas que a citam.
    pub fn entity_context(&self, rel: &str) -> Result<EntityContext, String> {
        let content = self.read_note(rel)?;
        let index = self.index()?;
        let notes = self.load_all()?;
        let me = notes.iter().find(|n| n.meta.path == rel).ok_or_else(|| "A nota não existe mais.".to_string())?;
        let names = variants(&me.meta);
        let mentions = |text: &str| -> usize { let f = fold(text); names.iter().map(|n| count_word(&f, n)).sum() };
        let link_ref = |path: &str| notes.iter().find(|n| n.meta.path == path).map(|n| LinkRef { path: n.meta.path.clone(), title: n.meta.title.clone(), heading: n.meta.heading.clone(), kind: n.meta.kind });
        let outgoing = index.edges.iter().filter(|e| e.from == rel).filter_map(|e| link_ref(&e.to)).collect();
        let backlinks = index.edges.iter().filter(|e| e.to == rel).filter_map(|e| link_ref(&e.from)).collect();
        let snippet = |text: &str| -> String {
            let line = text.lines().find(|l| mentions(l) > 0).unwrap_or("");
            clip(line.trim_start_matches(['#', '-', '*', '|', ' ']), 180)
        };
        let records = index.record_rows.iter().filter(|r| r.note != rel).filter(|r| mentions(&r.cells.join(" ")) > 0).take(60).map(|r| RecordMention {
            id: r.id.clone(), note: r.note.clone(), line: r.line, status: r.status, title: r.cells.get(1).map(|c| clip(c, 160)).unwrap_or_default(), snippet: snippet(&r.cells.join(" | ")),
        }).collect();
        let mut sections = Vec::new();
        for note in notes.iter().filter(|n| n.meta.path != rel && n.meta.kind != Kind::Capitulo) {
            for (heading, body) in split_sections(&note.body) {
                if mentions(&heading) == 0 && mentions(&body) == 0 { continue; }
                if sections.len() >= 30 { break; }
                sections.push(SectionMention { note: note.meta.path.clone(), note_title: note.meta.heading.clone().unwrap_or_else(|| note.meta.title.clone()), heading: heading.trim().to_string(), snippet: snippet(&format!("{heading}\n{body}")), status: crate::bible::owning_status(&note.body, heading.trim()) });
            }
        }
        let mut cited: Vec<NoteMention> = notes.iter().filter(|n| n.meta.path != rel).filter_map(|n| { let count = mentions(&n.body); (count > 0).then(|| NoteMention { path: n.meta.path.clone(), title: n.meta.title.clone(), heading: n.meta.heading.clone(), kind: n.meta.kind, count }) }).collect();
        cited.sort_by(|a, b| b.count.cmp(&a.count));
        cited.truncate(25);
        Ok(EntityContext { path: rel.to_string(), title: me.meta.title.clone(), heading: me.meta.heading.clone(), kind: me.meta.kind, aliases: me.meta.aliases.clone(), content, outgoing, backlinks, records, sections, notes: cited })
    }

    /// Quantas vezes cada nota (título, apelido, nome sem hífen ou primeiro nome) é citada em cada capítulo.
    pub fn mentions(&self, chapters: &[Vec<String>]) -> Result<Vec<NoteMentions>, String> {
        let folded: Vec<String> = chapters.iter().map(|lines| fold(&lines.join("\n"))).collect();
        let mut out = Vec::new();
        for note in self.load_all()?.iter().filter(|n| is_entity(n.meta.kind)) {
            let names = variants(&note.meta);
            let per: Vec<ChapterCount> = folded.iter().enumerate().map(|(chapter, text)| ChapterCount { chapter, count: names.iter().map(|n| count_word(text, n)).sum() }).filter(|c| c.count > 0).collect();
            let total = per.iter().map(|c| c.count).sum();
            if total > 0 { out.push(NoteMentions { path: note.meta.path.clone(), chapters: per, total }); }
        }
        out.sort_by(|a, b| b.total.cmp(&a.total));
        Ok(out)
    }

    /// Memória para os prompts: recaps anteriores, notas e seções da bíblia sobre o que o capítulo
    /// cita, fios e registros em aberto. Conteúdo marcado como superado nunca entra.
    pub fn memory(&self, current: &[String], previous_chapters: &[String]) -> Result<MemoryContext, String> {
        const TOTAL_MAX: usize = 9000;
        let notes = self.load_all()?;
        let text = fold(&current.join("\n"));
        let mut sections: Vec<String> = Vec::new();
        let (mut recaps_used, mut notes_used) = (Vec::new(), Vec::new());

        let stems: Vec<String> = previous_chapters.iter().map(|n| n.rsplit_once('.').map(|(s, _)| s).unwrap_or(n).to_string()).collect();
        let mut recaps = Vec::new();
        for stem in stems.iter().rev() {
            if recaps.len() >= 3 { break; }
            if let Some(note) = notes.iter().find(|n| n.meta.kind == Kind::Capitulo && n.meta.title.eq_ignore_ascii_case(stem)) {
                recaps.push(format!("### {}\n{}", note.meta.title, clip(&strip_superseded(&note.body), 1500)));
                recaps_used.push(note.meta.path.clone());
            }
        }
        recaps.reverse();
        if !recaps.is_empty() { sections.push(format!("RECAPS DOS CAPÍTULOS ANTERIORES\n{}", recaps.join("\n\n"))); }

        let mut cited: Vec<(usize, &Loaded, Vec<String>)> = notes.iter().filter(|n| is_entity(n.meta.kind)).filter_map(|n| {
            let names = variants(&n.meta);
            let hits: usize = names.iter().map(|name| count_word(&text, name)).sum();
            (hits > 0).then_some((hits, n, names))
        }).collect();
        cited.sort_by(|a, b| b.0.cmp(&a.0));
        let mut cited_names: Vec<String> = cited.iter().flat_map(|(_, _, names)| names.iter().cloned()).filter(|n| count_word(&text, n) > 0).collect();
        let blocks: Vec<String> = cited.iter().take(8).map(|(_, n, _)| { notes_used.push(n.meta.path.clone()); format!("### {} ({})\n{}", n.meta.title, n.meta.kind.id(), clip(&strip_superseded(&n.body), 700)) }).collect();
        if !blocks.is_empty() { sections.push(format!("NOTAS DO AUTOR SOBRE ELEMENTOS CITADOS NESTE CAPÍTULO\n{}", blocks.join("\n\n"))); }

        // Seções da bíblia (ou de notas gerais) cujo título é um elemento citado no capítulo.
        let mut from_bible: Vec<String> = Vec::new();
        'notes: for note in notes.iter().filter(|n| matches!(n.meta.kind, Kind::Biblia | Kind::Nota)) {
            for (heading, body) in split_sections(&note.body) {
                if from_bible.len() >= 6 { break 'notes; }
                let h = fold(&heading);
                let heading_hits: Vec<String> = heading_variants(&heading).into_iter().filter(|v| count_word(&text, v) > 0).collect();
                let matches = !heading_hits.is_empty() || cited_names.iter().any(|name| h.contains(name.as_str()));
                if !matches || body.trim().is_empty() || notes_used.iter().any(|u| u.ends_with(&format!("{}.md", heading.trim()))) { continue; }
                let clean = crate::bible::strip_canon_tags(&strip_superseded(&body));
                if clean.trim().is_empty() { continue; }
                // Proposta da IA ainda não aprovada ou lacuna declarada: a IA de análise precisa saber.
                let label = match crate::bible::owning_status(&note.body, heading.trim()).as_deref() { Some("proposto") => "[PROPOSTO, NÃO CONFIRMADO PELO AUTOR] ", Some("aberto") => "[LACUNA EM ABERTO] ", _ => "" };
                cited_names.extend(heading_hits);
                notes_used.push(format!("{}#{}", note.meta.path, heading.trim()));
                from_bible.push(format!("### {label}{} (de {})\n{}", heading.trim(), note.meta.title, clip(&clean, 700)));
            }
        }
        if !from_bible.is_empty() { sections.push(format!("TRECHOS DA BÍBLIA SOBRE O QUE O CAPÍTULO CITA\n{}", from_bible.join("\n\n"))); }

        let open_threads: Vec<String> = notes.iter().filter(|n| n.meta.kind == Kind::Fio && n.meta.state.as_deref().map(|s| !s.eq_ignore_ascii_case("fechado")).unwrap_or(true)).take(10).map(|n| format!("- {}: {}", n.meta.title, clip(&n.body.lines().filter(|l| !l.trim().is_empty() && !l.starts_with('#')).collect::<Vec<_>>().join(" "), 160))).collect();
        if !open_threads.is_empty() { sections.push(format!("FIOS NARRATIVOS ABERTOS\n{}", open_threads.join("\n"))); }

        let rows: Vec<Record> = notes.iter().flat_map(|n| records::parse(&n.meta.path, &n.body)).collect();
        let open_records: Vec<String> = records::summarize(&rows).into_iter().filter(|r| r.status == Some("aberto")).filter(|r| { let t = fold(&r.title); cited_names.iter().any(|name| t.contains(name.as_str())) }).take(8).map(|r| format!("- {} (em aberto): {}", r.id, clip(&r.title, 180))).collect();
        if !open_records.is_empty() { sections.push(format!("REGISTROS EM ABERTO RELACIONADOS AO CAPÍTULO\n{}", open_records.join("\n"))); }

        let mut joined = sections.join("\n\n");
        if joined.chars().count() > TOTAL_MAX { joined = joined.chars().take(TOTAL_MAX).collect::<String>() + "…"; }
        Ok(MemoryContext { text: joined, notes_used, recaps_used })
    }

    /// Grava o LOG-RECAP do capítulo e atualiza as notas relacionadas (cria o que faltar).
    pub fn apply_recap(&self, chapter_stem: &str, order: u32, recap: &Recap) -> Result<Vec<Change>, String> {
        let stem = sanitize_title(chapter_stem)?;
        let mut changes = Vec::new();
        let mut notes = self.load_all()?;
        let mut resolver = Resolver::new(&notes);

        let ensure = |kind: Kind, name: &str, changes: &mut Vec<Change>, notes: &mut Vec<Loaded>, resolver: &mut Resolver| -> Result<Option<String>, String> {
            let name = name.trim();
            if name.is_empty() { return Ok(None); }
            if let Some(path) = resolver.resolve(name) { return Ok(Some(path.clone())); }
            let rel = self.create_note(kind, name)?;
            changes.push(Change { path: rel.clone(), action: "criada" });
            *notes = self.load_all()?;
            *resolver = Resolver::new(notes);
            Ok(Some(rel))
        };
        let link_to = |resolver: &Resolver, name: &str| -> String {
            match resolver.resolve(name.trim()) { Some(path) => format!("[[{}]]", path.rsplit('/').next().unwrap_or(path).trim_end_matches(".md")), None => format!("[[{}]]", name.trim()) }
        };

        let mut body = format!("---\ntipo: capitulo\nordem: {order}\naliases: []\n---\n# {stem}\n\n## Resumo\n{}\n\n## Linha do tempo\n", recap.summary.trim());
        for e in &recap.timeline { body.push_str(&format!("- {}{}\n", e.event.trim(), if e.scene.trim().is_empty() { String::new() } else { format!(" — {}", e.scene.trim()) })); }
        body.push_str("\n## Estado dos personagens\n");
        for c in &recap.characters {
            let Some(path) = ensure(Kind::Personagem, &c.name, &mut changes, &mut notes, &mut resolver)? else { continue };
            body.push_str(&format!("- {}: {}\n", link_to(&resolver, &c.name), c.state.trim()));
            if self.append_unique(&path, "## Aparições", &format!("- [[{stem}]]: {}", c.state.trim()))? { changes.push(Change { path, action: "atualizada" }); }
        }
        body.push_str("\n## Lugares\n");
        for p in &recap.places {
            if ensure(Kind::Lugar, &p.name, &mut changes, &mut notes, &mut resolver)?.is_none() { continue; }
            body.push_str(&format!("- {}{}\n", link_to(&resolver, &p.name), if p.note.trim().is_empty() { String::new() } else { format!(": {}", p.note.trim()) }));
        }
        body.push_str("\n## Fios abertos\n");
        for t in &recap.threads_open {
            let Some(path) = ensure(Kind::Fio, &t.title, &mut changes, &mut notes, &mut resolver)? else { continue };
            body.push_str(&format!("- {}: {}\n", link_to(&resolver, &t.title), t.text.trim()));
            if self.append_unique(&path, "## Desenvolvimento", &format!("- [[{stem}]]: {}", t.text.trim()))? { changes.push(Change { path, action: "atualizada" }); }
        }
        body.push_str("\n## Fios fechados\n");
        for t in &recap.threads_closed {
            let Some(path) = ensure(Kind::Fio, &t.title, &mut changes, &mut notes, &mut resolver)? else { continue };
            body.push_str(&format!("- ✓ {}: {}\n", link_to(&resolver, &t.title), t.text.trim()));
            let current = self.read_note(&path)?;
            self.write_note(&path, &set_frontmatter(&current, "estado", "fechado"))?;
            self.append_unique(&path, "## Desenvolvimento", &format!("- ✓ Fechado em [[{stem}]]: {}", t.text.trim()))?;
            changes.push(Change { path, action: "fio fechado" });
        }
        body.push_str("\n## Fatos estabelecidos\n");
        for f in &recap.facts { body.push_str(&format!("- {}\n", f.trim())); }
        body.push_str("\n## Alertas para os próximos capítulos\n");
        for a in &recap.alerts { body.push_str(&format!("- ⚠ {}\n", a.trim())); }

        let rel = format!("{}/{stem}.md", self.map.folder_for(Kind::Capitulo));
        let existed = self.note_path(&rel)?.exists();
        self.write_note(&rel, &body)?;
        changes.insert(0, Change { path: rel, action: if existed { "recap substituído" } else { "recap criado" } });
        Ok(changes)
    }

    fn append_unique(&self, rel: &str, section: &str, line: &str) -> Result<bool, String> {
        let current = self.read_note(rel)?;
        if current.lines().any(|l| l.trim() == line.trim()) { return Ok(false); }
        let mut updated = current.trim_end().to_string();
        if !updated.lines().any(|l| l.trim() == section) { updated.push_str(&format!("\n\n{section}")); }
        let section_pos = updated.rfind(section).expect("seção garantida");
        let after = &updated[section_pos + section.len()..];
        let end = after.find("\n## ").map(|i| section_pos + section.len() + i).unwrap_or(updated.len());
        updated.insert_str(end, &format!("\n{line}"));
        updated.push('\n');
        self.write_note(rel, &updated)?;
        Ok(true)
    }

    /// Eventos dos LOG-RECAPs, na ordem dos capítulos.
    pub fn timeline(&self) -> Result<Vec<TimelineEntry>, String> {
        let mut entries: Vec<TimelineEntry> = Vec::new();
        for note in self.load_all()?.iter().filter(|n| n.meta.kind == Kind::Capitulo) {
            let text = self.read_note(&note.meta.path)?;
            let (front, body) = split_frontmatter(&text);
            let order = front.and_then(|f| frontmatter_value(f, "ordem")).and_then(|v| v.parse().ok()).unwrap_or(0);
            let mut events = Vec::new();
            let mut inside = false;
            for line in body.lines() {
                if line.starts_with("## ") { inside = line.trim() == "## Linha do tempo"; continue; }
                if inside { if let Some(item) = line.trim().strip_prefix("- ") { events.push(item.to_string()); } }
            }
            if !events.is_empty() { entries.push(TimelineEntry { chapter: note.meta.title.clone(), order, events }); }
        }
        entries.sort_by(|a, b| a.order.cmp(&b.order).then(a.chapter.cmp(&b.chapter)));
        Ok(entries)
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Reference { pub path: String, pub title: String, pub heading: Option<String>, pub kind: Kind, pub count: usize }

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkRef { pub path: String, pub title: String, pub heading: Option<String>, pub kind: Kind }

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordMention { pub id: String, pub note: String, pub line: usize, pub status: Option<&'static str>, pub title: String, pub snippet: String }

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SectionMention { pub note: String, pub note_title: String, pub heading: String, pub snippet: String, pub status: Option<String> }

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteMention { pub path: String, pub title: String, pub heading: Option<String>, pub kind: Kind, pub count: usize }

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityContext {
    pub path: String, pub title: String, pub heading: Option<String>, pub kind: Kind, pub aliases: Vec<String>, pub content: String,
    pub outgoing: Vec<LinkRef>, pub backlinks: Vec<LinkRef>, pub records: Vec<RecordMention>, pub sections: Vec<SectionMention>, pub notes: Vec<NoteMention>,
}

pub struct RenamePlan { pub old_rel: String, pub new_rel: String, pub old_title: String, pub new_title: String, resolver: Resolver }

impl RenamePlan {
    /// Reescreve os links que apontam para a nota (por título ou caminho). `is_renamed_note`: também ajusta o `# título`.
    pub fn rewrite(&self, text: &str, is_renamed_note: bool) -> String {
        let rewritten = wikilink().replace_all(text, |caps: &regex::Captures| {
            let target = caps[1].trim();
            let last = target.rsplit('/').next().unwrap_or(target).trim_end_matches(".md");
            // Só reescreve links pelo título; links por apelido continuam válidos.
            if last.eq_ignore_ascii_case(&self.old_title) && self.resolver.resolve(target).map(|p| *p == self.old_rel).unwrap_or(false) {
                let prefix = target.rfind('/').map(|i| &target[..=i]).unwrap_or("");
                format!("[[{prefix}{}{}{}]]", self.new_title, caps.get(2).map(|m| m.as_str()).unwrap_or(""), caps.get(3).map(|m| m.as_str()).unwrap_or(""))
            } else { caps[0].to_string() }
        }).into_owned();
        if is_renamed_note { rewritten.replacen(&format!("# {}\n", self.old_title), &format!("# {}\n", self.new_title), 1) } else { rewritten }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameResult { pub new_path: String, pub updated_notes: usize }

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteMentions { pub path: String, pub chapters: Vec<ChapterCount>, pub total: usize }

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChapterCount { pub chapter: usize, pub count: usize }

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryContext { pub text: String, pub notes_used: Vec<String>, pub recaps_used: Vec<String> }

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct TimelineEvent { pub event: String, pub scene: String }
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct RecapCharacter { pub name: String, pub state: String }
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct RecapPlace { pub name: String, pub note: String }
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct RecapThread { pub title: String, pub text: String }

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Recap {
    pub summary: String,
    pub timeline: Vec<TimelineEvent>,
    pub characters: Vec<RecapCharacter>,
    pub places: Vec<RecapPlace>,
    pub threads_open: Vec<RecapThread>,
    pub threads_closed: Vec<RecapThread>,
    pub facts: Vec<String>,
    pub alerts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Change { pub path: String, pub action: &'static str }

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineEntry { pub chapter: String, pub order: u32, pub events: Vec<String> }

fn is_entity(kind: Kind) -> bool { matches!(kind, Kind::Personagem | Kind::Lugar | Kind::Fio | Kind::Evento | Kind::Mundo) }

/// Nomes pelos quais uma nota é citada no texto (já sem acento e em minúsculas).
fn variants(meta: &NoteMeta) -> Vec<String> {
    let spaced = fold(&meta.title.replace(['-', '_'], " "));
    let mut names = vec![spaced.clone()];
    names.extend(meta.aliases.iter().map(|a| fold(a)));
    if meta.kind == Kind::Personagem {
        if let Some(first) = spaced.split_whitespace().next() { if first.chars().count() >= 3 && first != spaced { names.push(first.to_string()); } }
    }
    names.sort();
    names.dedup();
    names.retain(|n| n.chars().count() >= 3);
    names
}

/// Nomes pelos quais o título de uma seção é citado: o título todo e, se tiver 2+ palavras
/// (ex.: "Lívia Valença"), o primeiro nome.
fn heading_variants(heading: &str) -> Vec<String> {
    let folded = fold(heading.trim());
    let words: Vec<&str> = folded.split_whitespace().collect();
    let mut out = vec![folded.clone()];
    if (2..=4).contains(&words.len()) && heading.trim().chars().next().map(char::is_uppercase).unwrap_or(false) { if let Some(first) = words.first() { if first.chars().count() >= 3 { out.push(first.to_string()); } } }
    out
}

fn count_word(haystack: &str, needle: &str) -> usize {
    if needle.chars().count() < 3 { return 0; }
    haystack.match_indices(needle).filter(|(start, _)| {
        haystack[..*start].chars().next_back().map(|c| !c.is_alphanumeric()).unwrap_or(true)
            && haystack[start + needle.len()..].chars().next().map(|c| !c.is_alphanumeric()).unwrap_or(true)
    }).count()
}

fn clip(text: &str, max: usize) -> String {
    let trimmed = text.trim();
    if trimmed.chars().count() <= max { trimmed.to_string() } else { format!("{}…", trimmed.chars().take(max).collect::<String>()) }
}

fn heading_of(line: &str) -> Option<(usize, &str)> {
    let t = line.trim_start();
    let level = t.chars().take_while(|c| *c == '#').count();
    (1..=6).contains(&level).then_some(()).and_then(|_| t[level..].strip_prefix(' ')).map(|rest| (level, rest.trim()))
}

/// (título, corpo) de cada seção de nível 2 a 4.
fn split_sections(body: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    let mut current: Option<(usize, String, String)> = None;
    for line in body.lines() {
        if let Some((level, title)) = heading_of(line) {
            if let Some((_, t, b)) = current.take() { out.push((t, b)); }
            if (2..=4).contains(&level) { current = Some((level, title.to_string(), String::new())); }
            continue;
        }
        if let Some((_, _, b)) = current.as_mut() { b.push_str(line); b.push('\n'); }
    }
    if let Some((_, t, b)) = current { out.push((t, b)); }
    out
}

/// Remove do texto o que o autor marcou como superado: linhas com `#canon/superado` e seções
/// de histórico ("Registro histórico preservado — regras supersedidas").
pub fn strip_superseded(body: &str) -> String {
    let mut out = String::new();
    let mut skip_level: Option<usize> = None;
    for line in body.lines() {
        if let Some((level, title)) = heading_of(line) {
            if skip_level.map(|s| level <= s).unwrap_or(false) { skip_level = None; }
            let t = fold(title);
            if skip_level.is_none() && (t.contains("supersedid") || t.contains("historico preservado") || t.contains("superad")) { skip_level = Some(level); }
        }
        if skip_level.is_some() { continue; }
        if fold(line).contains("#canon/superado") { continue; }
        out.push_str(line);
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project() -> tempfile::TempDir { tempfile::tempdir().unwrap() }
    fn v(dir: &tempfile::TempDir) -> Vault { Vault::plain(dir.path().to_str().unwrap()) }

    #[test]
    fn creates_notes_from_templates_and_indexes_them() {
        let dir = project();
        let vault = v(&dir);
        assert!(vault.index().unwrap().notes.is_empty());
        let aria = vault.create_note(Kind::Personagem, "Aria").unwrap();
        vault.create_note(Kind::Lugar, "Torre Norte").unwrap();
        assert_eq!(aria, "Personagens/Aria.md");
        assert!(vault.create_note(Kind::Personagem, "Aria").is_err(), "duplicada");
        assert!(vault.read_note(&aria).unwrap().contains("**Ferida central:**"));
        let idx = vault.index().unwrap();
        assert!(idx.exists);
        assert_eq!(idx.notes.iter().filter(|n| n.kind == Kind::Personagem).count(), 1);
        assert!(idx.notes.iter().any(|n| n.title == "Torre Norte" && n.kind == Kind::Lugar));
    }

    #[test]
    fn resolves_wikilinks_aliases_and_reports_broken_ones() {
        let dir = project();
        let vault = v(&dir);
        vault.write_note("Personagens/Aria.md", "---\ntipo: personagem\naliases: [Ari, \"A Fugitiva\"]\n---\n# Aria\nVive na [[Torre Norte]].").unwrap();
        vault.write_note("Lugares/Torre Norte.md", "# Torre Norte\nMoradia de [[Ari]] e [[Personagens/Aria|ela]] e [[Fantasma#parte]].").unwrap();
        let idx = vault.index().unwrap();
        let aria = idx.notes.iter().find(|n| n.title == "Aria").unwrap();
        assert_eq!(aria.aliases, vec!["Ari", "A Fugitiva"]);
        let pairs: Vec<(&str, &str)> = idx.edges.iter().map(|e| (e.from.as_str(), e.to.as_str())).collect();
        assert!(pairs.contains(&("Personagens/Aria.md", "Lugares/Torre Norte.md")) && pairs.contains(&("Lugares/Torre Norte.md", "Personagens/Aria.md")));
        assert_eq!(idx.edges.len(), 2);
        assert_eq!(idx.unresolved.len(), 1);
        assert_eq!(idx.unresolved[0].target, "Fantasma");
    }

    #[test]
    fn resolves_the_authors_relative_path_links_without_frontmatter() {
        let dir = project();
        let vault = v(&dir);
        vault.write_note("04-Personagens/Fichas/Iris-Barbosa.md", "# Íris\nProtagonista.").unwrap();
        vault.write_note("04-Personagens/Indice.md", "# Índice").unwrap();
        vault.write_note("Arcos/Indice.md", "# Índice dos arcos").unwrap();
        vault.write_note("05-Plots/Plottwists-L1.md", "Veja [[../04-Personagens/Fichas/Iris-Barbosa]] e [[Arcos/Indice]] e [[../04-Personagens/Indice]] e [[../01-Biblias/Biblia-L1#6-Mecânica]].").unwrap();
        let idx = vault.index().unwrap();
        let to: Vec<&str> = idx.edges.iter().filter(|e| e.from.contains("Plottwists")).map(|e| e.to.as_str()).collect();
        assert!(to.contains(&"04-Personagens/Fichas/Iris-Barbosa.md") && to.contains(&"Arcos/Indice.md") && to.contains(&"04-Personagens/Indice.md"), "{to:?}");
        assert_eq!(idx.unresolved.iter().map(|u| u.target.as_str()).collect::<Vec<_>>(), vec!["../01-Biblias/Biblia-L1"]);
    }

    #[test]
    fn folder_mapping_decides_kinds_for_notes_without_frontmatter() {
        let dir = project();
        let mut entries = BTreeMap::new();
        entries.insert("04-Personagens".to_string(), "personagem".to_string());
        entries.insert("07-Revisao".to_string(), "registro".to_string());
        entries.insert("01-Biblias".to_string(), "biblia".to_string());
        let vault = Vault::new(dir.path().to_str().unwrap(), KindMap::from_entries(&entries));
        vault.write_note("04-Personagens/Fichas/Caio-Barbosa.md", "# Caio").unwrap();
        vault.write_note("07-Revisao/Registro-de-Conflitos.md", "| CF-01 | x | y | aberto |").unwrap();
        vault.write_note("01-Biblias/Biblia-L1.md", "# Bíblia").unwrap();
        vault.write_note("Soltas/ideia.md", "texto").unwrap();
        let kinds: BTreeMap<String, Kind> = vault.index().unwrap().notes.into_iter().map(|n| (n.title, n.kind)).collect();
        assert_eq!(kinds["Caio-Barbosa"], Kind::Personagem);
        assert_eq!(kinds["Registro-de-Conflitos"], Kind::Registro);
        assert_eq!(kinds["Biblia-L1"], Kind::Biblia);
        assert_eq!(kinds["ideia"], Kind::Nota);
        assert_eq!(vault.map().folder_for(Kind::Personagem), "04-personagens", "novas notas vão para a pasta mapeada");
    }

    #[test]
    fn indexes_records_by_id_with_latest_status_and_does_not_touch_files() {
        let dir = project();
        let vault = v(&dir);
        let content = "# Registro\n\n| ID | Questão | Estado |\n|---|---|---|\n| CF-34 | C35 manda Caio correr | aberto para revisão |\n\n| CF-34 | Resolvido por continuidade local |\n";
        vault.write_note("07-Revisao/Registro.md", content).unwrap();
        let before = fs::metadata(dir.path().join("07-Revisao/Registro.md")).unwrap().modified().unwrap();
        let idx = vault.index().unwrap();
        vault.memory(&["Caio correu".to_string()], &[]).unwrap();
        assert_eq!(idx.records.len(), 1);
        assert_eq!((idx.records[0].status, idx.records[0].occurrences), (Some("resolvido"), 2));
        assert_eq!(idx.record_rows.len(), 2);
        assert_eq!(fs::read_to_string(dir.path().join("07-Revisao/Registro.md")).unwrap(), content, "ler nunca reescreve notas do autor");
        assert_eq!(fs::metadata(dir.path().join("07-Revisao/Registro.md")).unwrap().modified().unwrap(), before);
    }

    #[test]
    fn rename_rewrites_links_everywhere_but_not_alias_links() {
        let dir = project();
        let vault = v(&dir);
        vault.write_note("Personagens/Aria.md", "---\naliases: [Ari]\n---\n# Aria\n").unwrap();
        vault.write_note("Lugares/Torre.md", "# Torre\n[[Aria]] [[aria|ela]] [[Aria#Estado]] [[Personagens/Aria]] [[Ari]] [[Outra]]").unwrap();
        let result = vault.rename_note("Personagens/Aria.md", "Aria Vhraress").unwrap();
        assert_eq!(result.new_path, "Personagens/Aria Vhraress.md");
        let torre = vault.read_note("Lugares/Torre.md").unwrap();
        assert_eq!(torre.trim(), "# Torre\n[[Aria Vhraress]] [[Aria Vhraress|ela]] [[Aria Vhraress#Estado]] [[Personagens/Aria Vhraress]] [[Ari]] [[Outra]]");
        assert!(vault.read_note("Personagens/Aria Vhraress.md").unwrap().contains("# Aria Vhraress"));
        assert!(vault.index().unwrap().unresolved.iter().all(|u| u.target != "Aria"));
    }

    #[test]
    fn titles_are_sanitized_and_paths_cannot_escape() {
        assert_eq!(sanitize_title("  A/B: C*?  ").unwrap(), "AB C");
        assert!(sanitize_title("///").is_err());
        let dir = project();
        let vault = v(&dir);
        assert!(vault.write_note("../fora.md", "x").is_err());
        assert!(vault.write_note("Notas/a.txt", "x").is_err());
        assert!(vault.read_note("/etc/passwd.md").is_err());
    }

    #[test]
    fn mentions_match_names_without_accents_and_first_names() {
        let dir = project();
        let mut entries = BTreeMap::new();
        entries.insert("04-Personagens".to_string(), "personagem".to_string());
        let vault = Vault::new(dir.path().to_str().unwrap(), KindMap::from_entries(&entries));
        vault.write_note("04-Personagens/Fichas/Iris-Barbosa.md", "# Íris Barbosa").unwrap();
        vault.write_note("Personagens/Aria.md", "---\ntipo: personagem\naliases: [Ari]\n---\n# Aria").unwrap();
        vault.write_note("Lugares/Torre.md", "---\ntipo: lugar\n---\n# Torre").unwrap();
        let chapters = vec![vec!["Íris olhou. Iris Barbosa riu. Aria entrou; Ari sorriu na torre.".to_string()], vec!["Ariana não conta. A Torre caiu.".to_string()]];
        let found = vault.mentions(&chapters).unwrap();
        let get = |p: &str| found.iter().find(|m| m.path == p).map(|m| m.total);
        assert_eq!(get("04-Personagens/Fichas/Iris-Barbosa.md"), Some(3), "Íris, Iris Barbosa (nome inteiro) e Iris");
        assert_eq!(get("Personagens/Aria.md"), Some(2));
        assert_eq!(get("Lugares/Torre.md"), Some(2));
    }

    #[test]
    fn references_and_entity_context_gather_the_dossier_of_a_character() {
        let dir = project();
        let vault = v(&dir);
        vault.write_note("Personagens/Caio-Barbosa.md", "---\naliases: [Jordan]\n---\n# Caio Barbosa\nIrmão de [[Íris]]. Operador.\n").unwrap();
        vault.write_note("Personagens/Íris.md", "# Íris\nLiga-se a [[Caio-Barbosa]].\n").unwrap();
        vault.write_note("Lugares/Chácara.md", "# Chácara\nOnde Jordan dorme.\n").unwrap();
        vault.write_note("Biblias/B.md", "---\ntipo: biblia\n---\n# B\n## 4. Personagens\n### Caio Barbosa\nAdotado.\n\n#canon/proposto\n").unwrap();
        vault.write_note("Registros/PT.md", "| ID | Reviravolta |\n|---|---|\n| PT-L1-04 | Jordan é adotivo |\n| PT-L1-05 | outra coisa |\n").unwrap();
        vault.write_note("Capitulos/c1.md", "---\ntipo: capitulo\n---\n# c1\nJordan chegou.\n").unwrap();
        let refs = vault.references(&["Jordan e Íris olharam a chácara. Jordan sorriu.".to_string()]).unwrap();
        assert_eq!(refs[0].title, "Caio-Barbosa");
        assert_eq!(refs[0].count, 2, "o apelido Jordan conta");
        assert!(refs.iter().any(|r| r.title == "Íris") && refs.iter().any(|r| r.title == "Chácara"));
        let ctx = vault.entity_context("Personagens/Caio-Barbosa.md").unwrap();
        assert!(ctx.content.contains("Operador"));
        assert_eq!(ctx.outgoing.len(), 1);
        assert_eq!(ctx.backlinks.len(), 1);
        assert_eq!(ctx.records.len(), 1);
        assert_eq!(ctx.records[0].id, "PT-L1-04");
        assert!(ctx.sections.iter().any(|s| s.heading == "Caio Barbosa" && s.status.as_deref() == Some("proposto")));
        assert!(ctx.notes.iter().any(|n| n.title == "c1") && ctx.notes.iter().any(|n| n.title == "Chácara"));
    }

    #[test]
    fn index_and_matrix_files_in_the_character_folder_are_not_characters() {
        let dir = project();
        let mut entries = BTreeMap::new();
        entries.insert("04-Personagens".to_string(), "personagem".to_string());
        let vault = Vault::new(dir.path().to_str().unwrap(), KindMap::from_entries(&entries));
        vault.write_note("04-Personagens/Indice.md", "# Índice de Personagens\n").unwrap();
        vault.write_note("04-Personagens/Matriz-de-Relacoes.md", "# Matriz\n").unwrap();
        vault.write_note("04-Personagens/Fichas/Eugenio-Castelo.md", "# Eugênio Castelo\n").unwrap();
        let idx = vault.index().unwrap();
        let kind = |t: &str| idx.notes.iter().find(|n| n.title == t).unwrap().kind;
        assert_eq!((kind("Indice"), kind("Matriz-de-Relacoes")), (Kind::Nota, Kind::Registro));
        assert_eq!(idx.notes.iter().find(|n| n.title == "Eugenio-Castelo").unwrap().heading.as_deref(), Some("Eugênio Castelo"));
        assert_eq!(kind("Eugenio-Castelo"), Kind::Personagem);
        // "índice" no texto do capítulo não puxa a nota de índice para a memória.
        assert!(!vault.memory(&["O índice do livro.".to_string()], &[]).unwrap().notes_used.iter().any(|n| n.contains("Indice")));
    }

    #[test]
    fn add_aliases_handles_inline_block_missing_and_no_frontmatter() {
        let n = |a: &[&str]| a.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(add_aliases("---\ntipo: personagem\naliases: []\n---\n# X\n", &n(&["Jordan"])), "---\ntipo: personagem\naliases: [Jordan]\n---\n# X\n");
        assert_eq!(add_aliases("---\naliases: [Cai]\n---\nx", &n(&["Jordan", "cai"])), "---\naliases: [Cai, Jordan]\n---\nx");
        assert_eq!(add_aliases("---\naliases:\n  - Cai\ntipo: x\n---\nx", &n(&["Jordan"])), "---\naliases:\n  - Cai\n  - Jordan\ntipo: x\n---\nx");
        assert_eq!(add_aliases("---\ntipo: x\n---\nx", &n(&["Jordan"])), "---\ntipo: x\naliases: [Jordan]\n---\nx");
        assert_eq!(add_aliases("# Caio\nx", &n(&["Jordan"])), "---\naliases: [Jordan]\n---\n# Caio\nx");
        assert_eq!(add_aliases("---\naliases: [Jordan]\n---\nx", &n(&["jordan"])), "---\naliases: [Jordan]\n---\nx", "sem duplicar");
    }

    #[test]
    fn memory_labels_proposed_and_open_bible_sections_and_hides_the_tags() {
        let dir = project();
        let vault = v(&dir);
        vault.write_note("01-Biblias/Biblia.md", "---\ntipo: biblia\n---\n# Bíblia\n## Personagens\n### Lívia Valença\nTem 12 anos.\n\n#canon/proposto\n\n## Grupos\n### Minerva\nObjetivo desconhecido.\n\n#canon/aberto\n\n## Regras\n### Lucas\nConfirmado.\n\n#canon/confirmado\n").unwrap();
        let ctx = vault.memory(&["Lívia, Minerva e Lucas.".to_string()], &[]).unwrap();
        assert!(ctx.text.contains("[PROPOSTO, NÃO CONFIRMADO PELO AUTOR] Lívia Valença"), "{}", ctx.text);
        assert!(ctx.text.contains("[LACUNA EM ABERTO] Minerva"), "{}", ctx.text);
        assert!(ctx.text.contains("### Lucas") && !ctx.text.contains("[PROPOSTO, NÃO CONFIRMADO PELO AUTOR] Lucas"));
        assert!(!ctx.text.contains("#canon/"), "as tags são metadados do app");
    }

    #[test]
    fn memory_gathers_recaps_cited_notes_bible_sections_and_open_items() {
        let dir = project();
        let vault = v(&dir);
        vault.write_note("Capitulos/cap1.md", "---\ntipo: capitulo\n---\n# cap1\n## Resumo\nAria fugiu.").unwrap();
        vault.write_note("Capitulos/cap2.md", "---\ntipo: capitulo\n---\n# cap2\n## Resumo\nAria chegou à torre.").unwrap();
        vault.write_note("Personagens/Aria.md", "---\ntipo: personagem\n---\n# Aria\nFerida: perdeu a irmã.").unwrap();
        vault.write_note("Personagens/Beto.md", "---\ntipo: personagem\n---\n# Beto\nNão aparece.").unwrap();
        vault.write_note("Fios/Quem matou o rei.md", "---\ntipo: fio\nestado: aberto\n---\n# Quem matou o rei\nMistério central.").unwrap();
        vault.write_note("Fios/Dívida.md", "---\ntipo: fio\nestado: fechado\n---\n# Dívida\nResolvido.").unwrap();
        vault.write_note("01-Biblias/Biblia.md", "---\ntipo: biblia\n---\n# Bíblia\n## Personagens\n### Lívia Valença\nTem 12 anos; adotada aos 6.\n### Marta Seles\nVeterinária.\n## Mecânica do poder\n- Regra vigente: sinais, não diagnóstico.\n### Registro histórico preservado — regras supersedidas\n- o custo é proporcional à complexidade.\n## Fim\n").unwrap();
        vault.write_note("07-Revisao/CF.md", "| ID | Questão | Estado |\n|---|---|---|\n| CF-30 | Lívia e Lucas no C31 | aberto: sem decisão |\n| CF-31 | Marta no C33 | resolvido |\n").unwrap();
        let ctx = vault.memory(&["Aria olhou a torre. Lívia riu.".to_string()], &["cap1.xhtml".to_string(), "cap2.xhtml".to_string(), "cap3.xhtml".to_string()]).unwrap();
        assert!(ctx.text.find("Aria fugiu").unwrap() < ctx.text.find("Aria chegou").unwrap(), "ordem cronológica");
        assert!(ctx.text.contains("perdeu a irmã") && !ctx.text.contains("Não aparece"));
        assert!(ctx.text.contains("Tem 12 anos; adotada aos 6."), "seção da bíblia sobre quem o capítulo cita");
        assert!(!ctx.text.contains("Veterinária"), "Marta não foi citada");
        assert!(ctx.text.contains("Quem matou o rei") && !ctx.text.contains("Resolvido."));
        assert!(ctx.text.contains("CF-30 (em aberto)") && !ctx.text.contains("CF-31"));
        assert_eq!(ctx.recaps_used.len(), 2);
        assert!(!ctx.text.contains("proporcional"), "regra supersedida nunca entra na memória");
    }

    #[test]
    fn superseded_sections_and_tagged_lines_are_removed() {
        let body = "## Regras\n- vigente #canon/confirmado\n- velha #canon/superado\n### Registro histórico preservado — regras supersedidas\n- antiga\n#### sub\n- mais antiga\n## Outro\n- fica\n";
        let out = strip_superseded(body);
        assert!(out.contains("vigente") && out.contains("fica"));
        assert!(!out.contains("velha") && !out.contains("antiga") && !out.contains("mais antiga"));
    }

    #[test]
    fn detects_a_folder_mapping_from_the_vault_layout() {
        let dir = project();
        for p in ["01-Biblias", "04-Personagens/Fichas", "05-Plots", "06-Mapas", "07-Revisao", "Logs/L1", "03-Mundo", "02-Trilogia"] { fs::create_dir_all(dir.path().join(p)).unwrap(); }
        let map = detect_mapping(dir.path().to_str().unwrap());
        assert_eq!(map["01-Biblias"], "biblia");
        assert_eq!(map["04-Personagens"], "personagem");
        assert_eq!(map["04-Personagens/Fichas"], "personagem");
        assert_eq!((map["05-Plots"].as_str(), map["07-Revisao"].as_str(), map["Logs"].as_str(), map["03-Mundo"].as_str()), ("registro", "registro", "capitulo", "mundo"));
        assert!(!map.contains_key("02-Trilogia"), "pasta sem palavra-chave fica como Nota");
    }

    #[test]
    fn applying_a_recap_writes_the_log_creates_stubs_and_closes_threads() {
        let dir = project();
        let vault = v(&dir);
        vault.create_note(Kind::Personagem, "Aria").unwrap();
        vault.create_note(Kind::Fio, "Quem matou o rei").unwrap();
        let recap = Recap {
            summary: "Aria foge da torre.".into(),
            timeline: vec![TimelineEvent { event: "Aria escapa".into(), scene: "cena 2".into() }],
            characters: vec![RecapCharacter { name: "Aria".into(), state: "ferida, sozinha".into() }, RecapCharacter { name: "Beto".into(), state: "preso".into() }],
            places: vec![RecapPlace { name: "Torre Norte".into(), note: "em chamas".into() }],
            threads_open: vec![RecapThread { title: "Traição do conselho".into(), text: "alguém abriu o portão".into() }],
            threads_closed: vec![RecapThread { title: "Quem matou o rei".into(), text: "foi o regente".into() }],
            facts: vec!["O portão só abre por dentro".into()],
            alerts: vec!["Beto sumiu no cap. 4?".into()],
        };
        let changes = vault.apply_recap("capitulo 3", 3, &recap).unwrap();
        assert_eq!(changes[0].path, "Capitulos/capitulo 3.md");
        let created: Vec<&str> = changes.iter().filter(|c| c.action == "criada").map(|c| c.path.as_str()).collect();
        assert!(created.contains(&"Personagens/Beto.md") && created.contains(&"Lugares/Torre Norte.md") && created.contains(&"Fios/Traição do conselho.md"), "{created:?}");
        let log = vault.read_note("Capitulos/capitulo 3.md").unwrap();
        assert!(log.contains("ordem: 3") && log.contains("[[Aria]]: ferida, sozinha") && log.contains("✓ [[Quem matou o rei]]"));
        let fio = vault.read_note("Fios/Quem matou o rei.md").unwrap();
        assert!(fio.contains("estado: fechado") && fio.contains("Fechado em [[capitulo 3]]"));
        assert!(vault.read_note("Personagens/Aria.md").unwrap().contains("- [[capitulo 3]]: ferida, sozinha"));
        let again = vault.apply_recap("capitulo 3", 3, &recap).unwrap();
        assert_eq!(again[0].action, "recap substituído");
        assert_eq!(vault.read_note("Personagens/Aria.md").unwrap().matches("[[capitulo 3]]: ferida").count(), 1);
        let events = vault.timeline().unwrap();
        assert_eq!(events[0].events, vec!["Aria escapa — cena 2"]);
        assert!(vault.index().unwrap().unresolved.is_empty());
    }

    #[test]
    fn frontmatter_helpers_roundtrip() {
        let text = "---\ntipo: fio\nestado: aberto\naliases:\n  - X\n  - \"Y\"\n---\n# T\ncorpo";
        let (front, body) = split_frontmatter(text);
        assert_eq!(parse_aliases(front.unwrap()), vec!["X", "Y"]);
        assert_eq!(body, "# T\ncorpo");
        let changed = set_frontmatter(text, "estado", "fechado");
        assert!(changed.contains("estado: fechado") && !changed.contains("estado: aberto") && changed.ends_with("# T\ncorpo"));
        assert!(set_frontmatter("sem front", "k", "v").starts_with("---\nk: v\n---\n"));
    }

    /// Diagnóstico opt-in com arquivos reais do autor: LIVRO_FIXTURE_DIR=<pasta com os .md>.
    #[test]
    #[ignore]
    fn real_vault_end_to_end_report() {
        let fixtures = std::env::var("LIVRO_FIXTURE_DIR").expect("defina LIVRO_FIXTURE_DIR");
        let dir = project();
        for (file, rel) in [("182635df-Biblia-L1-A-Cura.md", "01-Biblias/Biblia-L1-A-Cura.md"), ("1f7cb255-Registro-de-Conflitos-de-Fonte.md", "07-Revisao/Registro-de-Conflitos-de-Fonte.md"), ("b4d13cf7-Plottwists-L1.md", "05-Plots/Plottwists-L1.md"), ("8114a467-Eugenio-Castelo.md", "04-Personagens/Fichas/Eugenio-Castelo.md"), ("955f70e0-Matriz-de-Relacoes.md", "04-Personagens/Matriz-de-Relacoes.md"), ("26bbd4b0-Indice.md", "04-Personagens/Indice.md"), ("9aa13957-Regras-Canonicas.md", "03-Mundo/Regras-Canonicas.md"), ("1ed8972a-Cronologia-Canonica.md", "03-Mundo/Cronologia-Canonica.md")] {
            let target = dir.path().join(rel);
            fs::create_dir_all(target.parent().unwrap()).unwrap();
            fs::copy(std::path::Path::new(&fixtures).join(file), target).unwrap();
        }
        fs::create_dir_all(dir.path().join("04-Personagens/Fichas")).unwrap();
        fs::write(dir.path().join("04-Personagens/Fichas/Iris-Barbosa.md"), "# Íris Barbosa").unwrap();
        let mapping = detect_mapping(dir.path().to_str().unwrap());
        println!("mapeamento detectado: {mapping:?}");
        let vault = Vault::new(dir.path().to_str().unwrap(), KindMap::from_entries(&mapping));
        let idx = vault.index().unwrap();
        println!("notas: {:?}", idx.notes.iter().map(|n| (n.title.as_str(), n.kind.id())).collect::<Vec<_>>());
        println!("arestas resolvidas: {}  links sem nota: {}", idx.edges.len(), idx.unresolved.len());
        let mut targets: Vec<&str> = idx.unresolved.iter().map(|u| u.target.as_str()).collect();
        targets.sort(); targets.dedup();
        println!("alvos ausentes (notas que o vault completo teria): {}", targets.len());
        println!("registros: {} IDs ({} linhas)", idx.records.len(), idx.record_rows.len());
        let ctx = vault.memory(&["Íris abriu os olhos. Lívia escreveu símbolos enquanto Eugênio observava. O poder falhou outra vez.".to_string()], &[]).unwrap();
        println!("--- MEMÓRIA ({} caracteres) ---\n{}", ctx.text.chars().count(), ctx.text);
        println!("--- usados: {:?}", ctx.notes_used);
        assert!(!ctx.text.contains("proporcional à complexidade"), "a regra supersedida da seção 6 não pode entrar");
        // Substituição de nome sobre o vault real (só prévia, nada é gravado).
        let opts = crate::replace::Options { ignore_accents: false, preserve_case: true };
        for (rel, text) in vault.raw_notes().unwrap().iter().map(|(m, t)| (m.path.clone(), t.clone())) {
            let (_, r) = crate::replace::replace_in_markdown(&text, "Caio", "Jordan", opts);
            if r.count > 0 || r.partial > 0 { println!("Caio→Jordan em {rel}: {} (parecidas {})", r.count, r.partial); }
        }
        let eugenio = vault.read_note("04-Personagens/Fichas/Eugenio-Castelo.md").unwrap();
        let (out, r) = crate::replace::replace_in_markdown(&eugenio, "Eugênio", "Mateus", opts);
        println!("Eugênio→Mateus na ficha: {} ocorrência(s); links preservados: {}", r.count, out.matches("[[../../").count() == eugenio.matches("[[../../").count());
    }
}
