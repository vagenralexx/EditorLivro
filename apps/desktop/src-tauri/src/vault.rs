//! Caderno do livro: notas Markdown compatíveis com o Obsidian
//! (`[[wikilinks]]`, frontmatter YAML) guardadas em `.livro-notas/` no projeto.
//! Mantém a "memória" entre capítulos: personagens, lugares, fios narrativos,
//! eventos e o LOG-RECAP de cada capítulo (formato da skill).

use crate::project_files;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::sync::OnceLock;
use std::time::UNIX_EPOCH;

pub const DIR: &str = ".livro-notas";
const MAX_NOTES: usize = 5000;
const MAX_NOTE_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum Kind { Personagem, Lugar, Fio, Evento, Mundo, Capitulo, Nota }

impl Kind {
    pub const ALL: [Kind; 7] = [Kind::Personagem, Kind::Lugar, Kind::Fio, Kind::Evento, Kind::Mundo, Kind::Capitulo, Kind::Nota];
    pub fn folder(self) -> &'static str {
        match self { Kind::Personagem => "Personagens", Kind::Lugar => "Lugares", Kind::Fio => "Fios", Kind::Evento => "Eventos", Kind::Mundo => "Mundo", Kind::Capitulo => "Capitulos", Kind::Nota => "Notas" }
    }
    pub fn id(self) -> &'static str {
        match self { Kind::Personagem => "personagem", Kind::Lugar => "lugar", Kind::Fio => "fio", Kind::Evento => "evento", Kind::Mundo => "mundo", Kind::Capitulo => "capitulo", Kind::Nota => "nota" }
    }
    fn from_id(id: &str) -> Option<Kind> { Kind::ALL.into_iter().find(|k| k.id() == id.trim().to_lowercase()) }
    fn from_folder(folder: &str) -> Option<Kind> { Kind::ALL.into_iter().find(|k| k.folder() == folder) }

    /// Modelo de nota nova (baseado em `criacao.md`, `SKILL.md` e `revisao-sistemica.md`).
    pub fn template(self, title: &str) -> String {
        let head = format!("---\ntipo: {}\naliases: []\n{}---\n# {title}\n\n", self.id(), if self == Kind::Fio { "estado: aberto\n" } else { "" });
        let body = match self {
            Kind::Personagem => "**Papel:** \n**Ferida central:** \n**Desejo consciente:** \n**Necessidade inconsciente:** \n**Obsessão irrelevante:** \n**Distorção cognitiva:** \n**Vício de linguagem:** \n**O que evita dizer:** \n**Sob pressão máxima:** \n**Maior contradição:** \n\n## Estado atual\n\n## Aparições\n",
            Kind::Lugar => "**Descrição sensorial:** \n**Regras / o que acontece aqui:** \n\n## Aparições\n",
            Kind::Fio => "**Descrição do fio:** \n**Aberto em:** \n**Fechado em:** \n\n## Desenvolvimento\n",
            Kind::Evento => "**Quando (ordem cronológica):** \n**Onde:** \n**Quem sabe disso:** \n**Consequências:** \n",
            Kind::Mundo => "**Regra:** \n**Custo físico:** \n**Custo emocional:** \n**Custo narrativo:** \n**Limites (o que NÃO pode fazer):** \n",
            Kind::Capitulo => "## Resumo\n",
            Kind::Nota => "",
        };
        format!("{head}{body}")
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteMeta {
    /// Caminho relativo ao caderno (ex.: `Personagens/Aria.md`).
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
pub struct VaultIndex { pub exists: bool, pub notes: Vec<NoteMeta>, pub edges: Vec<Edge>, pub unresolved: Vec<Unresolved> }

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

pub fn sanitize_title(title: &str) -> Result<String, String> {
    let cleaned: String = title.chars().filter(|c| !matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '#' | '^' | '[' | ']') && !c.is_control()).collect();
    let cleaned = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    let cleaned = cleaned.trim_matches('.').trim().to_string();
    if cleaned.is_empty() { return Err("Informe um título para a nota.".to_string()); }
    Ok(cleaned.chars().take(100).collect())
}

fn vault_root(root: &str) -> Result<std::path::PathBuf, String> { project_files::resolve(root, DIR) }

fn note_path(root: &str, rel: &str) -> Result<std::path::PathBuf, String> {
    if !rel.ends_with(".md") { return Err("Só notas .md são permitidas.".to_string()); }
    project_files::resolve(root, &format!("{DIR}/{rel}"))
}

struct Loaded { meta: NoteMeta, body: String, links: Vec<String> }

fn load_all(root: &str) -> Result<Vec<Loaded>, String> {
    let base = vault_root(root)?;
    if !base.is_dir() { return Ok(Vec::new()); }
    let mut out = Vec::new();
    let mut stack = vec![(base.clone(), String::new())];
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
            let folder_kind = rel.split('/').next().and_then(Kind::from_folder);
            let kind = front.and_then(|f| frontmatter_value(f, "tipo")).and_then(Kind::from_id).or(folder_kind).unwrap_or(Kind::Nota);
            let links = wikilink().captures_iter(body).map(|c| c[1].trim().to_string()).collect();
            let modified = meta.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0);
            out.push(Loaded {
                meta: NoteMeta { title: rel.rsplit('/').next().unwrap_or(&rel).trim_end_matches(".md").to_string(), path: rel, kind, aliases: front.map(parse_aliases).unwrap_or_default(), words: body.split_whitespace().count(), modified, state: front.and_then(|f| frontmatter_value(f, "estado")).map(str::to_string) },
                body: body.to_string(), links,
            });
        }
    }
    out.sort_by(|a, b| (a.meta.kind, a.meta.title.to_lowercase()).cmp(&(b.meta.kind, b.meta.title.to_lowercase())));
    Ok(out)
}

fn resolver(notes: &[Loaded]) -> HashMap<String, String> {
    let mut map: HashMap<String, String> = HashMap::new();
    for note in notes {
        map.entry(note.meta.title.to_lowercase()).or_insert_with(|| note.meta.path.clone());
        map.entry(note.meta.path.trim_end_matches(".md").to_lowercase()).or_insert_with(|| note.meta.path.clone());
    }
    for note in notes { for alias in &note.meta.aliases { map.entry(alias.to_lowercase()).or_insert_with(|| note.meta.path.clone()); } }
    map
}

fn resolve_target<'a>(map: &'a HashMap<String, String>, target: &str) -> Option<&'a String> {
    let lower = target.trim().to_lowercase();
    map.get(&lower).or_else(|| map.get(lower.rsplit('/').next().unwrap_or(&lower)))
}

pub fn index(root: &str) -> Result<VaultIndex, String> {
    let exists = vault_root(root)?.is_dir();
    let notes = load_all(root)?;
    let map = resolver(&notes);
    let (mut edges, mut unresolved, mut seen) = (Vec::new(), Vec::new(), HashSet::new());
    for note in &notes {
        for target in &note.links {
            match resolve_target(&map, target) {
                Some(to) if *to != note.meta.path => { if seen.insert((note.meta.path.clone(), to.clone())) { edges.push(Edge { from: note.meta.path.clone(), to: to.clone() }); } }
                Some(_) => {}
                None => unresolved.push(Unresolved { from: note.meta.path.clone(), target: target.clone() }),
            }
        }
    }
    Ok(VaultIndex { exists, notes: notes.into_iter().map(|n| n.meta).collect(), edges, unresolved })
}

pub fn read_note(root: &str, rel: &str) -> Result<String, String> {
    let path = note_path(root, rel)?;
    if fs::metadata(&path).map(|m| m.len()).unwrap_or(0) > MAX_NOTE_BYTES { return Err("A nota é grande demais.".to_string()); }
    fs::read_to_string(&path).map_err(|e| format!("Não foi possível ler a nota: {e}"))
}

pub fn write_note(root: &str, rel: &str, content: &str) -> Result<(), String> {
    if content.len() as u64 > MAX_NOTE_BYTES { return Err("A nota passa de 1 MB.".to_string()); }
    project_files::write_atomic(&note_path(root, rel)?, content.as_bytes())
}

pub fn create_note(root: &str, kind: Kind, title: &str) -> Result<String, String> {
    let title = sanitize_title(title)?;
    let rel = format!("{}/{title}.md", kind.folder());
    let path = note_path(root, &rel)?;
    if path.exists() { return Err(format!("Já existe uma nota “{title}” em {}.", kind.folder())); }
    project_files::write_atomic(&path, kind.template(&title).as_bytes())?;
    Ok(rel)
}

pub fn delete_note(root: &str, rel: &str) -> Result<(), String> {
    let path = note_path(root, rel)?;
    fs::remove_file(&path).map_err(|e| format!("Não foi possível excluir a nota: {e}"))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameResult { pub new_path: String, pub updated_notes: usize }

/// Renomeia a nota e reescreve os `[[links]]` que apontavam para ela (como o Obsidian).
pub fn rename_note(root: &str, rel: &str, new_title: &str) -> Result<RenameResult, String> {
    let new_title = sanitize_title(new_title)?;
    let folder = rel.rsplit_once('/').map(|(f, _)| f).unwrap_or("");
    let new_rel = if folder.is_empty() { format!("{new_title}.md") } else { format!("{folder}/{new_title}.md") };
    let (old, new) = (note_path(root, rel)?, note_path(root, &new_rel)?);
    if !old.is_file() { return Err("A nota não existe mais.".to_string()); }
    if new.exists() && new_rel.to_lowercase() != rel.to_lowercase() { return Err(format!("Já existe uma nota “{new_title}”.")); }
    let old_title = rel.rsplit('/').next().unwrap_or(rel).trim_end_matches(".md").to_string();
    fs::rename(&old, &new).map_err(|e| format!("Não foi possível renomear: {e}"))?;
    let mut updated = 0;
    for note in load_all(root)? {
        let current = read_note(root, &note.meta.path)?;
        let rewritten = wikilink().replace_all(&current, |caps: &regex::Captures| {
            let target = caps[1].trim();
            let matches_old = target.eq_ignore_ascii_case(&old_title) || target.to_lowercase().trim_end_matches(".md") == rel.trim_end_matches(".md").to_lowercase();
            if matches_old { format!("[[{new_title}{}{}]]", caps.get(2).map(|m| m.as_str()).unwrap_or(""), caps.get(3).map(|m| m.as_str()).unwrap_or("")) } else { caps[0].to_string() }
        }).into_owned();
        // Se o título do próprio arquivo aparece como “# Título” na primeira linha, acompanha.
        let rewritten = if note.meta.path == new_rel { rewritten.replacen(&format!("# {old_title}\n"), &format!("# {new_title}\n"), 1) } else { rewritten };
        if rewritten != current { write_note(root, &note.meta.path, &rewritten)?; updated += 1; }
    }
    Ok(RenameResult { new_path: new_rel, updated_notes: updated })
}

// ---------- Menções por capítulo ----------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteMentions { pub path: String, pub chapters: Vec<ChapterCount>, pub total: usize }

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChapterCount { pub chapter: usize, pub count: usize }

fn count_word(haystack: &str, needle: &str) -> usize {
    if needle.chars().count() < 3 { return 0; }
    haystack.match_indices(needle).filter(|(start, _)| {
        haystack[..*start].chars().next_back().map(|c| !c.is_alphanumeric()).unwrap_or(true)
            && haystack[start + needle.len()..].chars().next().map(|c| !c.is_alphanumeric()).unwrap_or(true)
    }).count()
}

fn note_names(meta: &NoteMeta) -> Vec<String> {
    let mut names = vec![meta.title.to_lowercase()];
    names.extend(meta.aliases.iter().map(|a| a.to_lowercase()));
    names
}

/// Quantas vezes cada nota (título ou apelido) é citada em cada capítulo.
pub fn mentions(root: &str, chapters: &[Vec<String>]) -> Result<Vec<NoteMentions>, String> {
    let lowered: Vec<String> = chapters.iter().map(|lines| lines.join("\n").to_lowercase()).collect();
    let mut out = Vec::new();
    for note in load_all(root)?.iter().filter(|n| n.meta.kind != Kind::Capitulo) {
        let names = note_names(&note.meta);
        let per: Vec<ChapterCount> = lowered.iter().enumerate().map(|(chapter, text)| ChapterCount { chapter, count: names.iter().map(|n| count_word(text, n)).sum() }).filter(|c| c.count > 0).collect();
        let total = per.iter().map(|c| c.count).sum();
        if total > 0 { out.push(NoteMentions { path: note.meta.path.clone(), chapters: per, total }); }
    }
    out.sort_by(|a, b| b.total.cmp(&a.total));
    Ok(out)
}

// ---------- Memória para os prompts ----------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryContext { pub text: String, pub notes_used: Vec<String>, pub recaps_used: Vec<String> }

fn clip(text: &str, max: usize) -> String {
    let trimmed = text.trim();
    if trimmed.chars().count() <= max { trimmed.to_string() } else { format!("{}…", trimmed.chars().take(max).collect::<String>()) }
}

/// Monta o contexto de memória para a IA: recaps dos capítulos anteriores,
/// notas dos elementos citados no capítulo atual e fios ainda abertos.
pub fn memory(root: &str, current: &[String], previous_chapters: &[String]) -> Result<MemoryContext, String> {
    const TOTAL_MAX: usize = 9000;
    let notes = load_all(root)?;
    let text = current.join("\n").to_lowercase();
    let mut sections: Vec<String> = Vec::new();
    let mut recaps_used = Vec::new();
    let mut notes_used = Vec::new();

    let stems: Vec<String> = previous_chapters.iter().map(|n| n.rsplit_once('.').map(|(s, _)| s).unwrap_or(n).to_string()).collect();
    let mut recaps = Vec::new();
    for stem in stems.iter().rev() {
        if recaps.len() >= 3 { break; }
        if let Some(note) = notes.iter().find(|n| n.meta.kind == Kind::Capitulo && n.meta.title.eq_ignore_ascii_case(stem)) {
            recaps.push(format!("### {}\n{}", note.meta.title, clip(&note.body, 1500)));
            recaps_used.push(note.meta.path.clone());
        }
    }
    recaps.reverse();
    if !recaps.is_empty() { sections.push(format!("RECAPS DOS CAPÍTULOS ANTERIORES\n{}", recaps.join("\n\n"))); }

    let mut cited: Vec<(usize, &Loaded)> = notes.iter().filter(|n| !matches!(n.meta.kind, Kind::Capitulo)).filter_map(|n| {
        let hits: usize = note_names(&n.meta).iter().map(|name| count_word(&text, name)).sum();
        (hits > 0).then_some((hits, n))
    }).collect();
    cited.sort_by(|a, b| b.0.cmp(&a.0));
    let blocks: Vec<String> = cited.iter().take(8).map(|(_, n)| { notes_used.push(n.meta.path.clone()); format!("### {} ({})\n{}", n.meta.title, n.meta.kind.id(), clip(&n.body, 700)) }).collect();
    if !blocks.is_empty() { sections.push(format!("NOTAS DO AUTOR SOBRE ELEMENTOS CITADOS NESTE CAPÍTULO\n{}", blocks.join("\n\n"))); }

    let open_threads: Vec<String> = notes.iter().filter(|n| n.meta.kind == Kind::Fio && n.meta.state.as_deref().map(|s| !s.eq_ignore_ascii_case("fechado")).unwrap_or(true)).take(10).map(|n| format!("- {}: {}", n.meta.title, clip(&n.body.lines().filter(|l| !l.trim().is_empty() && !l.starts_with('#')).collect::<Vec<_>>().join(" "), 160))).collect();
    if !open_threads.is_empty() { sections.push(format!("FIOS NARRATIVOS ABERTOS\n{}", open_threads.join("\n"))); }

    let mut joined = sections.join("\n\n");
    if joined.chars().count() > TOTAL_MAX { joined = joined.chars().take(TOTAL_MAX).collect::<String>() + "…"; }
    Ok(MemoryContext { text: joined, notes_used, recaps_used })
}

// ---------- LOG-RECAP ----------

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

fn link_to(map: &HashMap<String, String>, name: &str) -> String {
    let name = name.trim();
    match resolve_target(map, name) { Some(path) => format!("[[{}]]", path.rsplit('/').next().unwrap_or(path).trim_end_matches(".md")), None => format!("[[{name}]]") }
}

fn append_unique(root: &str, rel: &str, section: &str, line: &str) -> Result<bool, String> {
    let current = read_note(root, rel)?;
    if current.lines().any(|l| l.trim() == line.trim()) { return Ok(false); }
    let mut updated = current.trim_end().to_string();
    if !updated.lines().any(|l| l.trim() == section) { updated.push_str(&format!("\n\n{section}")); }
    // Insere a linha ao fim da seção (ou do arquivo).
    let section_pos = updated.rfind(section).expect("seção garantida");
    let after = &updated[section_pos + section.len()..];
    let end = after.find("\n## ").map(|i| section_pos + section.len() + i).unwrap_or(updated.len());
    updated.insert_str(end, &format!("\n{line}"));
    updated.push('\n');
    write_note(root, rel, &updated)?;
    Ok(true)
}

/// Grava o LOG-RECAP do capítulo e atualiza as notas relacionadas (cria o que faltar).
pub fn apply_recap(root: &str, chapter_stem: &str, order: u32, recap: &Recap) -> Result<Vec<Change>, String> {
    let stem = sanitize_title(chapter_stem)?;
    let mut changes = Vec::new();
    let mut notes = load_all(root)?;
    let mut map = resolver(&notes);

    let ensure = |kind: Kind, name: &str, changes: &mut Vec<Change>, notes: &mut Vec<Loaded>, map: &mut HashMap<String, String>| -> Result<Option<String>, String> {
        let name = name.trim();
        if name.is_empty() { return Ok(None); }
        if let Some(path) = resolve_target(map, name) { return Ok(Some(path.clone())); }
        let rel = create_note(root, kind, name)?;
        changes.push(Change { path: rel.clone(), action: "criada" });
        *notes = load_all(root)?;
        *map = resolver(notes);
        Ok(Some(rel))
    };

    let mut body = format!("---\ntipo: capitulo\nordem: {order}\naliases: []\n---\n# {stem}\n\n## Resumo\n{}\n\n## Linha do tempo\n", recap.summary.trim());
    for e in &recap.timeline { body.push_str(&format!("- {}{}\n", e.event.trim(), if e.scene.trim().is_empty() { String::new() } else { format!(" — {}", e.scene.trim()) })); }
    body.push_str("\n## Estado dos personagens\n");
    for c in &recap.characters {
        let Some(path) = ensure(Kind::Personagem, &c.name, &mut changes, &mut notes, &mut map)? else { continue };
        let link = link_to(&map, &c.name);
        body.push_str(&format!("- {link}: {}\n", c.state.trim()));
        let line = format!("- [[{stem}]]: {}", c.state.trim());
        if append_unique(root, &path, "## Aparições", &line)? { changes.push(Change { path, action: "atualizada" }); }
    }
    body.push_str("\n## Lugares\n");
    for p in &recap.places {
        if ensure(Kind::Lugar, &p.name, &mut changes, &mut notes, &mut map)?.is_none() { continue; }
        body.push_str(&format!("- {}{}\n", link_to(&map, &p.name), if p.note.trim().is_empty() { String::new() } else { format!(": {}", p.note.trim()) }));
    }
    body.push_str("\n## Fios abertos\n");
    for t in &recap.threads_open {
        let Some(path) = ensure(Kind::Fio, &t.title, &mut changes, &mut notes, &mut map)? else { continue };
        body.push_str(&format!("- {}: {}\n", link_to(&map, &t.title), t.text.trim()));
        if append_unique(root, &path, "## Desenvolvimento", &format!("- [[{stem}]]: {}", t.text.trim()))? { changes.push(Change { path, action: "atualizada" }); }
    }
    body.push_str("\n## Fios fechados\n");
    for t in &recap.threads_closed {
        let Some(path) = ensure(Kind::Fio, &t.title, &mut changes, &mut notes, &mut map)? else { continue };
        body.push_str(&format!("- ✓ {}: {}\n", link_to(&map, &t.title), t.text.trim()));
        let current = read_note(root, &path)?;
        write_note(root, &path, &set_frontmatter(&current, "estado", "fechado"))?;
        append_unique(root, &path, "## Desenvolvimento", &format!("- ✓ Fechado em [[{stem}]]: {}", t.text.trim()))?;
        changes.push(Change { path, action: "fio fechado" });
    }
    body.push_str("\n## Fatos estabelecidos\n");
    for f in &recap.facts { body.push_str(&format!("- {}\n", f.trim())); }
    body.push_str("\n## Alertas para os próximos capítulos\n");
    for a in &recap.alerts { body.push_str(&format!("- ⚠ {}\n", a.trim())); }

    let rel = format!("{}/{stem}.md", Kind::Capitulo.folder());
    let existed = note_path(root, &rel)?.exists();
    write_note(root, &rel, &body)?;
    changes.insert(0, Change { path: rel, action: if existed { "recap substituído" } else { "recap criado" } });
    Ok(changes)
}

// ---------- Linha do tempo ----------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineEntry { pub chapter: String, pub order: u32, pub events: Vec<String> }

/// Eventos dos LOG-RECAPs, na ordem dos capítulos.
pub fn timeline(root: &str) -> Result<Vec<TimelineEntry>, String> {
    let mut entries: Vec<TimelineEntry> = Vec::new();
    for note in load_all(root)?.iter().filter(|n| n.meta.kind == Kind::Capitulo) {
        let text = read_note(root, &note.meta.path)?;
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


#[cfg(test)]
mod tests {
    use super::*;

    fn project() -> tempfile::TempDir { tempfile::tempdir().unwrap() }
    fn r(dir: &tempfile::TempDir) -> &str { dir.path().to_str().unwrap() }

    #[test]
    fn creates_notes_from_templates_and_indexes_them() {
        let dir = project();
        assert!(!index(r(&dir)).unwrap().exists);
        let aria = create_note(r(&dir), Kind::Personagem, "Aria").unwrap();
        create_note(r(&dir), Kind::Lugar, "Torre Norte").unwrap();
        assert_eq!(aria, "Personagens/Aria.md");
        assert!(create_note(r(&dir), Kind::Personagem, "Aria").is_err(), "duplicada");
        assert!(read_note(r(&dir), &aria).unwrap().contains("**Ferida central:**"));
        let idx = index(r(&dir)).unwrap();
        assert!(idx.exists);
        assert_eq!(idx.notes.iter().filter(|n| n.kind == Kind::Personagem).count(), 1);
        assert!(idx.notes.iter().any(|n| n.title == "Torre Norte" && n.kind == Kind::Lugar));
    }

    #[test]
    fn resolves_wikilinks_aliases_and_reports_broken_ones() {
        let dir = project();
        create_note(r(&dir), Kind::Personagem, "Aria").unwrap();
        write_note(r(&dir), "Personagens/Aria.md", "---\ntipo: personagem\naliases: [Ari, \"A Fugitiva\"]\n---\n# Aria\nVive na [[Torre Norte]].").unwrap();
        create_note(r(&dir), Kind::Lugar, "Torre Norte").unwrap();
        write_note(r(&dir), "Lugares/Torre Norte.md", "# Torre Norte\nMoradia de [[Ari]] e [[Personagens/Aria|ela]] e [[Fantasma#parte]].").unwrap();
        let idx = index(r(&dir)).unwrap();
        let aria = idx.notes.iter().find(|n| n.title == "Aria").unwrap();
        assert_eq!(aria.aliases, vec!["Ari", "A Fugitiva"]);
        let pairs: Vec<(&str, &str)> = idx.edges.iter().map(|e| (e.from.as_str(), e.to.as_str())).collect();
        assert!(pairs.contains(&("Personagens/Aria.md", "Lugares/Torre Norte.md")));
        assert!(pairs.contains(&("Lugares/Torre Norte.md", "Personagens/Aria.md")));
        assert_eq!(idx.edges.len(), 2, "arestas duplicadas são unificadas");
        assert_eq!(idx.unresolved.len(), 1);
        assert_eq!(idx.unresolved[0].target, "Fantasma");
    }

    #[test]
    fn rename_rewrites_links_everywhere() {
        let dir = project();
        create_note(r(&dir), Kind::Personagem, "Aria").unwrap();
        create_note(r(&dir), Kind::Lugar, "Torre").unwrap();
        write_note(r(&dir), "Lugares/Torre.md", "# Torre\n[[Aria]] [[aria|ela]] [[Aria#Estado]] [[Outra]]").unwrap();
        let result = rename_note(r(&dir), "Personagens/Aria.md", "Aria Vhraress").unwrap();
        assert_eq!(result.new_path, "Personagens/Aria Vhraress.md");
        assert!(result.updated_notes >= 1);
        let torre = read_note(r(&dir), "Lugares/Torre.md").unwrap();
        assert_eq!(torre.trim(), "# Torre\n[[Aria Vhraress]] [[Aria Vhraress|ela]] [[Aria Vhraress#Estado]] [[Outra]]");
        assert!(read_note(r(&dir), "Personagens/Aria Vhraress.md").unwrap().starts_with("---\ntipo: personagem"));
        assert!(index(r(&dir)).unwrap().unresolved.iter().all(|u| u.target != "Aria"));
        assert!(rename_note(r(&dir), "Lugares/Torre.md", "Aria Vhraress").is_ok(), "mesmo título em outra pasta é permitido");
    }

    #[test]
    fn titles_are_sanitized_and_paths_cannot_escape() {
        assert_eq!(sanitize_title("  A/B: C*?  ").unwrap(), "AB C");
        assert!(sanitize_title("///").is_err());
        let dir = project();
        assert!(write_note(r(&dir), "../fora.md", "x").is_err());
        assert!(write_note(r(&dir), "Notas/a.txt", "x").is_err());
        assert!(read_note(r(&dir), "/etc/passwd.md").is_err());
    }

    #[test]
    fn counts_mentions_per_chapter_using_titles_and_aliases() {
        let dir = project();
        write_note(r(&dir), "Personagens/Aria.md", "---\naliases: [Ari]\n---\n# Aria").unwrap();
        write_note(r(&dir), "Lugares/Torre.md", "# Torre").unwrap();
        let chapters = vec![vec!["Aria entrou. Ari sorriu na torre.".to_string()], vec!["Nada aqui. Ariana não conta.".to_string()], vec!["A Torre caiu; Aria correu.".to_string()]];
        let found = mentions(r(&dir), &chapters).unwrap();
        let aria = found.iter().find(|m| m.path == "Personagens/Aria.md").unwrap();
        assert_eq!((aria.total, aria.chapters.iter().map(|c| (c.chapter, c.count)).collect::<Vec<_>>()), (3, vec![(0, 2), (2, 1)]));
        assert_eq!(found.iter().find(|m| m.path == "Lugares/Torre.md").unwrap().total, 2);
    }

    #[test]
    fn memory_gathers_recaps_cited_notes_and_open_threads() {
        let dir = project();
        write_note(r(&dir), "Capitulos/cap1.md", "---\ntipo: capitulo\n---\n# cap1\n## Resumo\nAria fugiu.").unwrap();
        write_note(r(&dir), "Capitulos/cap2.md", "---\ntipo: capitulo\n---\n# cap2\n## Resumo\nAria chegou à torre.").unwrap();
        write_note(r(&dir), "Personagens/Aria.md", "---\ntipo: personagem\n---\n# Aria\nFerida: perdeu a irmã.").unwrap();
        write_note(r(&dir), "Personagens/Beto.md", "---\ntipo: personagem\n---\n# Beto\nNão aparece.").unwrap();
        write_note(r(&dir), "Fios/Quem matou o rei.md", "---\ntipo: fio\nestado: aberto\n---\n# Quem matou o rei\nMistério central.").unwrap();
        write_note(r(&dir), "Fios/Dívida.md", "---\ntipo: fio\nestado: fechado\n---\n# Dívida\nResolvido.").unwrap();
        let ctx = memory(r(&dir), &["Aria olhou a torre.".to_string()], &["cap1.xhtml".to_string(), "cap2.xhtml".to_string(), "cap3.xhtml".to_string()]).unwrap();
        assert!(ctx.text.contains("Aria fugiu") && ctx.text.contains("Aria chegou"));
        assert!(ctx.text.find("Aria fugiu").unwrap() < ctx.text.find("Aria chegou").unwrap(), "ordem cronológica");
        assert!(ctx.text.contains("perdeu a irmã") && !ctx.text.contains("Não aparece"));
        assert!(ctx.text.contains("Quem matou o rei") && !ctx.text.contains("Resolvido"));
        assert_eq!(ctx.recaps_used.len(), 2);
        assert_eq!(ctx.notes_used, vec!["Personagens/Aria.md"]);
        assert!(memory(r(&dir), &[], &[]).unwrap().text.contains("FIOS NARRATIVOS ABERTOS"));
    }

    #[test]
    fn applying_a_recap_writes_the_log_creates_stubs_and_closes_threads() {
        let dir = project();
        create_note(r(&dir), Kind::Personagem, "Aria").unwrap();
        create_note(r(&dir), Kind::Fio, "Quem matou o rei").unwrap();
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
        let changes = apply_recap(r(&dir), "capitulo 3", 3, &recap).unwrap();
        assert_eq!(changes[0].path, "Capitulos/capitulo 3.md");
        let created: Vec<&str> = changes.iter().filter(|c| c.action == "criada").map(|c| c.path.as_str()).collect();
        assert!(created.contains(&"Personagens/Beto.md") && created.contains(&"Lugares/Torre Norte.md") && created.contains(&"Fios/Traição do conselho.md"), "{created:?}");
        let log = read_note(r(&dir), "Capitulos/capitulo 3.md").unwrap();
        assert!(log.contains("ordem: 3") && log.contains("[[Aria]]: ferida, sozinha") && log.contains("✓ [[Quem matou o rei]]"));
        let fio = read_note(r(&dir), "Fios/Quem matou o rei.md").unwrap();
        assert!(fio.contains("estado: fechado") && fio.contains("Fechado em [[capitulo 3]]"));
        assert!(read_note(r(&dir), "Personagens/Aria.md").unwrap().contains("- [[capitulo 3]]: ferida, sozinha"));
        // Reaplicar é idempotente nas notas relacionadas.
        let again = apply_recap(r(&dir), "capitulo 3", 3, &recap).unwrap();
        assert_eq!(again[0].action, "recap substituído");
        assert_eq!(read_note(r(&dir), "Personagens/Aria.md").unwrap().matches("[[capitulo 3]]: ferida").count(), 1);
        let events = timeline(r(&dir)).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].events, vec!["Aria escapa — cena 2"]);
        let idx = index(r(&dir)).unwrap();
        assert!(idx.unresolved.is_empty(), "{:?}", idx.unresolved);
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
}
