//! Orquestra "substituir no livro": prévia (capítulos + impacto no caderno/glossário),
//! aplicação com backup e desfazer. A transformação de texto está em `replace.rs`.
//!
//! Antes de gravar qualquer coisa, os originais são copiados para
//! `.livro/substituicoes/<id>/` junto com um manifesto (impressão digital do que foi gravado);
//! se uma gravação falhar no meio, o que já foi escrito é revertido. Desfazer só restaura arquivos
//! que não mudaram desde a substituição (ou tudo, com `force`, perdendo edições posteriores).

use crate::glossary::{self, Glossary};
use crate::project_files;
use crate::replace::{self, Options, Report, Sample};
use crate::vault::{self, Kind, Vault};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const DIR: &str = ".livro/substituicoes";
const KEEP: usize = 20;
const MAX_TERM_CHARS: usize = 100;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub root: String,
    /// Capítulos (caminhos absolutos) a considerar.
    pub paths: Vec<String>,
    pub find: String,
    pub replace: String,
    #[serde(default)]
    pub ignore_accents: bool,
    #[serde(default = "yes")]
    pub preserve_case: bool,
}
fn yes() -> bool { true }

impl Request {
    fn options(&self) -> Options { Options { ignore_accents: self.ignore_accents, preserve_case: self.preserve_case } }
    pub fn validate(&self) -> Result<(), String> {
        let (find, repl) = (self.find.trim(), self.replace.trim());
        if find.is_empty() { return Err("Informe a palavra a procurar.".to_string()); }
        if repl.is_empty() { return Err("Informe a palavra nova. Para apagar uma palavra, edite o texto manualmente.".to_string()); }
        if find.chars().count() > MAX_TERM_CHARS || repl.chars().count() > MAX_TERM_CHARS { return Err(format!("Use no máximo {MAX_TERM_CHARS} caracteres.")); }
        if find == repl { return Err("A palavra nova é igual à procurada.".to_string()); }
        if find.contains(['<', '>']) || repl.contains(['<', '>']) { return Err("Não use < ou > (marcação) nos termos; só texto.".to_string()); }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChapterHit { pub path: String, pub name: String, pub count: usize, pub partial: usize, pub forms: BTreeMap<String, usize>, pub samples: Vec<Sample> }

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteHit {
    pub path: String,
    pub title: String,
    pub kind: Kind,
    pub count: usize,
    /// O termo é (parte do) nome desta nota: ficha do personagem/lugar a que a palavra se refere.
    pub entity: bool,
    /// Nota que guarda fatos da história (ficha, bíblia, registro, mundo, fio…): pergunta antes de mexer.
    pub canon: bool,
    pub samples: Vec<Sample>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub chapters: Vec<ChapterHit>,
    pub scanned: usize,
    pub notes: Vec<NoteHit>,
    pub glossary_count: usize,
    pub total: usize,
    pub partial_total: usize,
    pub forms: BTreeMap<String, usize>,
    pub partial_words: BTreeMap<String, usize>,
}

fn is_canon(kind: Kind) -> bool { !matches!(kind, Kind::Capitulo | Kind::Nota) }

fn read_chapter(root: &str, path: &str) -> Result<(PathBuf, String), String> {
    let inside = project_files::ensure_inside(root, path)?;
    let ext = inside.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    if !matches!(ext.as_str(), "xhtml" | "html" | "htm") { return Err("Só capítulos XHTML/HTML podem ser alterados.".to_string()); }
    let text = fs::read_to_string(&inside).map_err(|e| format!("Não foi possível ler {path}: {e}"))?;
    Ok((inside, text))
}

fn glossary_path(root: &str) -> Result<PathBuf, String> { project_files::resolve(root, glossary::FILE_NAME) }

fn load_glossary(root: &str) -> Result<Option<Glossary>, String> {
    let path = glossary_path(root)?;
    if !path.is_file() { return Ok(None); }
    let text = fs::read_to_string(&path).map_err(|e| format!("Não foi possível ler o glossário: {e}"))?;
    serde_json::from_str(&text).map(Some).map_err(|e| format!("O glossario.json é inválido (nada foi alterado): {e}"))
}

/// Aplica a substituição às cadeias do glossário. Devolve o novo glossário e quantas entradas mudaram;
/// `None` se renomear colidiria com outro personagem já existente.
fn replace_in_glossary(g: &Glossary, req: &Request) -> (Glossary, usize, Vec<String>) {
    let opts = req.options();
    let mut changed = 0;
    let mut warnings = Vec::new();
    let mut sub = |text: &str| -> String { let (out, rep) = replace::replace_in_markdown(text, &req.find, &req.replace, opts); if rep.count > 0 { changed += 1; } out };
    let mut next = g.clone();
    next.nao_traduzir = g.nao_traduzir.iter().map(|t| sub(t)).collect();
    next.traducoes_fixas = g.traducoes_fixas.iter().map(|(k, v)| (sub(k), sub(v))).collect();
    let mut people = BTreeMap::new();
    for (name, c) in &g.personagens {
        let new_name = sub(name);
        let value = glossary::Character { apelidos: c.apelidos.iter().map(|a| sub(a)).collect(), tratamentos: c.tratamentos.iter().map(|a| sub(a)).collect() };
        if people.contains_key(&new_name) { warnings.push(format!("O glossário já tem “{new_name}”: “{name}” não foi renomeado.")); people.insert(name.clone(), value); } else { people.insert(new_name, value); }
    }
    next.personagens = people;
    (next, changed, warnings)
}

fn name_has_term(title: &str, aliases: &[String], req: &Request) -> bool {
    let spaced = title.replace(['-', '_'], " ");
    std::iter::once(spaced.as_str()).chain(aliases.iter().map(String::as_str)).any(|t| replace::replace_in_markdown(t, &req.find, &req.replace, req.options()).1.count > 0)
}

pub fn preview(req: &Request, vault: &Vault) -> Result<Preview, String> {
    req.validate()?;
    let opts = req.options();
    let (mut chapters, mut total_report) = (Vec::new(), Report::default());
    for path in req.paths.iter().take(1000) {
        let (inside, text) = read_chapter(&req.root, path)?;
        let (_, report) = replace::replace_in_xhtml(&text, &req.find, &req.replace, opts);
        if report.count == 0 && report.partial == 0 { continue; }
        total_report.merge(&report, 0);
        chapters.push(ChapterHit { path: path.clone(), name: inside.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(), count: report.count, partial: report.partial, forms: report.forms, samples: report.samples });
    }
    let mut notes = Vec::new();
    for (meta, text) in vault.raw_notes()? {
        let (_, report) = replace::replace_in_markdown(&text, &req.find, &req.replace, opts);
        let entity = matches!(meta.kind, Kind::Personagem | Kind::Lugar | Kind::Fio | Kind::Evento | Kind::Mundo) && name_has_term(&meta.title, &meta.aliases, req);
        if report.count == 0 && !entity { continue; }
        notes.push(NoteHit { path: meta.path, title: meta.title, kind: meta.kind, count: report.count, entity, canon: is_canon(meta.kind), samples: report.samples });
    }
    notes.sort_by(|a, b| (!a.entity, !a.canon, a.title.to_lowercase()).cmp(&(!b.entity, !b.canon, b.title.to_lowercase())));
    let glossary_count = match load_glossary(&req.root)? { Some(g) => replace_in_glossary(&g, req).1, None => 0 };
    let scanned = req.paths.len().min(1000);
    Ok(Preview { total: chapters.iter().map(|c| c.count).sum(), partial_total: chapters.iter().map(|c| c.partial).sum(), chapters, scanned, notes, glossary_count, forms: total_report.forms, partial_words: total_report.partial_words })
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyRequest {
    #[serde(flatten)]
    pub base: Request,
    /// Capítulos a alterar (subconjunto de `paths`).
    pub chapters: Vec<String>,
    /// Notas do caderno cujo texto será alterado (caminhos relativos ao caderno).
    #[serde(default)]
    pub notes: Vec<String>,
    /// Fichas que ganham o nome novo como apelido (para a memória continuar reconhecendo o personagem).
    #[serde(default)]
    pub alias_notes: Vec<String>,
    #[serde(default)]
    pub glossary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FileEntry { kind: String, key: String, after: String }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Manifest { id: String, created_at: String, find: String, replace: String, files: Vec<FileEntry> }

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Applied { pub id: String, pub changed_chapters: Vec<String>, pub replaced: usize, pub notes_changed: Vec<String>, pub aliases_added: Vec<String>, pub glossary_changed: bool, pub warnings: Vec<String> }

struct Op { kind: &'static str, key: String, target: PathBuf, /** caminho como veio da interface (para a tela recarregar o capítulo aberto) */ shown: String, before: Vec<u8>, after: Vec<u8> }

fn stamp() -> String { book_epub::now_utc().replace(['-', ':'], "").replace('T', "-").trim_end_matches('Z').to_string() }

fn alias_candidates(title: &str, req: &Request) -> Vec<String> {
    let spaced = title.replace(['-', '_'], " ");
    let (new_title, _) = replace::replace_in_markdown(&spaced, &req.find, &req.replace, req.options());
    let mut out = vec![new_title.clone()];
    if !req.find.trim().contains(' ') && new_title.split_whitespace().count() > 1 { out.push(req.replace.trim().to_string()); }
    out.retain(|a| !a.is_empty() && vault::fold(a) != vault::fold(&spaced));
    out
}

pub fn apply(req: &ApplyRequest, vault: &Vault) -> Result<Applied, String> {
    req.base.validate()?;
    let root = req.base.root.as_str();
    let opts = req.base.options();
    let mut ops: Vec<Op> = Vec::new();
    let mut replaced = 0usize;
    let mut warnings = Vec::new();
    let canonical_root = fs::canonicalize(root).map_err(|e| format!("Pasta do projeto inacessível: {e}"))?;

    for path in &req.chapters {
        if !req.base.paths.contains(path) { return Err("Capítulo fora da lista analisada; analise de novo.".to_string()); }
        let (inside, text) = read_chapter(root, path)?;
        let (out, report) = replace::replace_in_xhtml(&text, &req.base.find, &req.base.replace, opts);
        if out == text { continue; }
        replaced += report.count;
        let rel = inside.strip_prefix(&canonical_root).map_err(|_| "O arquivo está fora da pasta do projeto.".to_string())?.to_string_lossy().replace('\\', "/");
        ops.push(Op { kind: "capitulo", key: rel, target: inside, shown: path.clone(), before: text.into_bytes(), after: out.into_bytes() });
    }

    let mut note_texts: BTreeMap<String, (String, String)> = BTreeMap::new();
    for rel in req.notes.iter().chain(req.alias_notes.iter()) {
        if note_texts.contains_key(rel) { continue; }
        let text = vault.read_note(rel)?;
        note_texts.insert(rel.clone(), (text.clone(), text));
    }
    for rel in &req.notes {
        let entry = note_texts.get_mut(rel).expect("carregada acima");
        let (out, report) = replace::replace_in_markdown(&entry.0, &req.base.find, &req.base.replace, opts);
        replaced += report.count;
        entry.1 = out;
    }
    let mut aliases_added = Vec::new();
    for rel in &req.alias_notes {
        let title = rel.rsplit('/').next().unwrap_or(rel).trim_end_matches(".md").to_string();
        let entry = note_texts.get_mut(rel).expect("carregada acima");
        let with = vault::add_aliases(&entry.1, &alias_candidates(&title, &req.base));
        if with != entry.1 { entry.1 = with; aliases_added.push(rel.clone()); }
    }
    let mut notes_changed = Vec::new();
    for (rel, (before, after)) in &note_texts {
        if before == after { continue; }
        let target = project_files::resolve(vault.dir(), rel)?;
        notes_changed.push(rel.clone());
        ops.push(Op { kind: "nota", key: rel.clone(), target, shown: rel.clone(), before: before.clone().into_bytes(), after: after.clone().into_bytes() });
    }

    let mut glossary_changed = false;
    if req.glossary {
        if let Some(g) = load_glossary(root)? {
            let (next, changed, warn) = replace_in_glossary(&g, &req.base);
            warnings.extend(warn);
            if changed > 0 {
                let target = glossary_path(root)?;
                let before = fs::read(&target).map_err(|e| e.to_string())?;
                let json = serde_json::to_string_pretty(&next.cleaned()).map_err(|e| e.to_string())?;
                glossary_changed = true;
                ops.push(Op { kind: "glossario", key: glossary::FILE_NAME.to_string(), target, shown: glossary::FILE_NAME.to_string(), before, after: json.into_bytes() });
            }
        }
    }
    if ops.is_empty() { return Err("Nada a substituir com a seleção atual.".to_string()); }

    // 1) Backup + manifesto ANTES de tocar em qualquer arquivo.
    let base = project_files::resolve(root, DIR)?;
    let mut id = stamp();
    while base.join(&id).exists() { id.push('x'); }
    let dir = base.join(&id);
    for op in &ops { project_files::write_atomic(&dir.join("antes").join(op.kind).join(&op.key), &op.before)?; }
    let manifest = Manifest { id: id.clone(), created_at: book_epub::now_utc(), find: req.base.find.trim().to_string(), replace: req.base.replace.trim().to_string(), files: ops.iter().map(|o| FileEntry { kind: o.kind.to_string(), key: o.key.clone(), after: replace::fingerprint(&o.after) }).collect() };
    project_files::write_atomic(&dir.join("manifesto.json"), serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?.as_bytes())?;

    // 2) Gravação; se algo falhar, reverte o que já foi escrito.
    let mut written: Vec<&Op> = Vec::new();
    for op in &ops {
        if let Err(error) = project_files::write_atomic(&op.target, &op.after) {
            for done in &written { let _ = project_files::write_atomic(&done.target, &done.before); }
            let _ = fs::remove_dir_all(&dir);
            return Err(format!("Falha ao gravar {}: {error}. Nada foi alterado.", op.key));
        }
        written.push(op);
    }
    prune(&base);
    Ok(Applied { id, changed_chapters: ops.iter().filter(|o| o.kind == "capitulo").map(|o| o.shown.clone()).collect(), replaced, notes_changed, aliases_added, glossary_changed, warnings })
}

fn prune(base: &Path) {
    let Ok(read) = fs::read_dir(base) else { return };
    let mut dirs: Vec<String> = read.flatten().filter(|e| e.path().is_dir()).map(|e| e.file_name().to_string_lossy().into_owned()).collect();
    dirs.sort();
    while dirs.len() > KEEP { let old = dirs.remove(0); let _ = fs::remove_dir_all(base.join(old)); }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryItem { pub id: String, pub created_at: String, pub find: String, pub replace: String, pub files: usize }

pub fn history(root: &str) -> Result<Vec<HistoryItem>, String> {
    let base = project_files::resolve(root, DIR)?;
    let Ok(read) = fs::read_dir(&base) else { return Ok(Vec::new()) };
    let mut out: Vec<HistoryItem> = read.flatten().filter_map(|e| {
        let manifest: Manifest = serde_json::from_str(&fs::read_to_string(e.path().join("manifesto.json")).ok()?).ok()?;
        Some(HistoryItem { id: manifest.id, created_at: manifest.created_at, find: manifest.find, replace: manifest.replace, files: manifest.files.len() })
    }).collect();
    out.sort_by(|a, b| b.id.cmp(&a.id));
    Ok(out)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
/// `changed_chapters`: caminhos relativos à pasta do projeto.
pub struct Undone { pub restored: Vec<String>, pub changed_chapters: Vec<String>, pub conflicts: Vec<String> }

/// Desfaz uma substituição. Sem `force`, se QUALQUER arquivo foi editado depois, nada é restaurado.
pub fn undo(root: &str, vault: &Vault, id: &str, force: bool) -> Result<Undone, String> {
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') { return Err("Identificador inválido.".to_string()); }
    let dir = project_files::resolve(root, DIR)?.join(id);
    let manifest: Manifest = serde_json::from_str(&fs::read_to_string(dir.join("manifesto.json")).map_err(|_| "Substituição não encontrada.".to_string())?).map_err(|e| format!("Manifesto ilegível: {e}"))?;
    let canonical_root = fs::canonicalize(root).map_err(|e| e.to_string())?;
    let mut plan: Vec<(PathBuf, Vec<u8>, &FileEntry)> = Vec::new();
    let mut conflicts = Vec::new();
    for entry in &manifest.files {
        let target = match entry.kind.as_str() {
            "capitulo" => project_files::resolve(canonical_root.to_str().unwrap_or(root), &entry.key)?,
            "nota" => project_files::resolve(vault.dir(), &entry.key)?,
            "glossario" => glossary_path(root)?,
            _ => continue,
        };
        let before = fs::read(dir.join("antes").join(&entry.kind).join(&entry.key)).map_err(|e| format!("Backup de {} ausente: {e}", entry.key))?;
        match fs::read(&target) { Ok(now) if replace::fingerprint(&now) == entry.after => {} _ => conflicts.push(entry.key.clone()) }
        plan.push((target, before, entry));
    }
    if !conflicts.is_empty() && !force { return Ok(Undone { restored: Vec::new(), changed_chapters: Vec::new(), conflicts }); }
    let (mut restored, mut chapters) = (Vec::new(), Vec::new());
    for (target, before, entry) in plan {
        project_files::write_atomic(&target, &before)?;
        restored.push(entry.key.clone());
        if entry.kind == "capitulo" { chapters.push(entry.key.clone()); }
    }
    let _ = fs::remove_dir_all(&dir);
    Ok(Undone { restored, changed_chapters: chapters, conflicts })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fx { dir: tempfile::TempDir, vault: Vault }
    fn fixture() -> Fx {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::write(root.join("c1.xhtml"), "<html><body><p>Caio chegou. Caiozinho riu. caio saiu.</p></body></html>").unwrap();
        fs::write(root.join("c2.xhtml"), "<html><body><p id=\"caio\">Sem ele aqui.</p></body></html>").unwrap();
        let vault = Vault::new(root.join("caderno").to_str().unwrap(), vault::KindMap::default());
        vault.write_note("Personagens/Caio-Barbosa.md", "# Caio Barbosa\nCaio é irmão de Íris.\n").unwrap();
        vault.write_note("Capitulos/c1.md", "---\ntipo: capitulo\n---\n# c1\nCaio chegou.\n").unwrap();
        fs::write(root.join("glossario.json"), r#"{"personagens":{"Caio":{"apelidos":["Cai"],"tratamentos":[]}}}"#).unwrap();
        Fx { dir, vault }
    }
    fn req(fx: &Fx) -> Request {
        let root = fx.dir.path();
        Request { root: root.to_string_lossy().into_owned(), paths: vec![root.join("c1.xhtml").to_string_lossy().into_owned(), root.join("c2.xhtml").to_string_lossy().into_owned()], find: "Caio".into(), replace: "Jordan".into(), ignore_accents: false, preserve_case: true }
    }

    #[test]
    fn preview_counts_chapters_notes_entities_and_glossary_without_writing() {
        let fx = fixture();
        let before = fs::read_to_string(fx.dir.path().join("c1.xhtml")).unwrap();
        let p = preview(&req(&fx), &fx.vault).unwrap();
        assert_eq!((p.total, p.partial_total, p.scanned), (2, 1, 2));
        assert_eq!(p.chapters.len(), 1, "c2 só tem o termo em atributo");
        let ficha = p.notes.iter().find(|n| n.title == "Caio-Barbosa").unwrap();
        assert!(ficha.entity && ficha.canon && ficha.count >= 2);
        assert!(!p.notes.iter().find(|n| n.kind == Kind::Capitulo).unwrap().canon);
        assert_eq!(p.glossary_count, 1);
        assert_eq!(fs::read_to_string(fx.dir.path().join("c1.xhtml")).unwrap(), before, "a prévia não grava");
    }

    #[test]
    fn apply_updates_everything_selected_and_undo_restores_it_exactly() {
        let fx = fixture();
        let r = req(&fx);
        let c1 = r.paths[0].clone();
        let original = fs::read(&c1).unwrap();
        let applied = apply(&ApplyRequest { base: r.clone(), chapters: vec![c1.clone()], notes: vec!["Personagens/Caio-Barbosa.md".into()], alias_notes: vec!["Personagens/Caio-Barbosa.md".into()], glossary: true }, &fx.vault).unwrap();
        let text = fs::read_to_string(&c1).unwrap();
        assert_eq!(text, "<html><body><p>Jordan chegou. Caiozinho riu. jordan saiu.</p></body></html>");
        let ficha = fx.vault.read_note("Personagens/Caio-Barbosa.md").unwrap();
        assert!(ficha.contains("# Jordan Barbosa") && ficha.contains("Jordan é irmão") && ficha.contains("aliases: [Jordan Barbosa, Jordan]"), "{ficha}");
        assert!(fx.vault.read_note("Capitulos/c1.md").unwrap().contains("Caio chegou"), "nota não selecionada fica intacta");
        let g: Glossary = serde_json::from_str(&fs::read_to_string(fx.dir.path().join("glossario.json")).unwrap()).unwrap();
        assert!(g.personagens.contains_key("Jordan") && !g.personagens.contains_key("Caio"));
        assert_eq!(applied.replaced, 4);
        assert_eq!(history(&r.root).unwrap().len(), 1);

        let undone = undo(&r.root, &fx.vault, &applied.id, false).unwrap();
        assert!(undone.conflicts.is_empty() && undone.restored.len() == 3);
        assert_eq!(fs::read(&c1).unwrap(), original);
        assert!(fx.vault.read_note("Personagens/Caio-Barbosa.md").unwrap().contains("# Caio Barbosa"));
        assert!(history(&r.root).unwrap().is_empty());
    }

    #[test]
    fn undo_refuses_when_a_file_was_edited_afterwards_unless_forced() {
        let fx = fixture();
        let r = req(&fx);
        let c1 = r.paths[0].clone();
        let applied = apply(&ApplyRequest { base: r.clone(), chapters: vec![c1.clone()], notes: vec![], alias_notes: vec![], glossary: false }, &fx.vault).unwrap();
        fs::write(&c1, "editado depois").unwrap();
        let blocked = undo(&r.root, &fx.vault, &applied.id, false).unwrap();
        assert_eq!(blocked.conflicts.len(), 1);
        assert!(blocked.restored.is_empty());
        assert_eq!(fs::read_to_string(&c1).unwrap(), "editado depois");
        let forced = undo(&r.root, &fx.vault, &applied.id, true).unwrap();
        assert_eq!(forced.restored.len(), 1);
        assert!(fs::read_to_string(&c1).unwrap().contains("Caio chegou"));
    }

    #[test]
    fn validation_and_scope_guards() {
        let fx = fixture();
        let mut r = req(&fx);
        r.replace = "  ".into();
        assert!(preview(&r, &fx.vault).is_err());
        r.replace = "Caio".into();
        assert!(preview(&r, &fx.vault).is_err(), "iguais");
        r.replace = "Jordan".into();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("x.xhtml"), "<p>Caio</p>").unwrap();
        r.paths = vec![outside.path().join("x.xhtml").to_string_lossy().into_owned()];
        assert!(preview(&r, &fx.vault).is_err(), "fora da pasta do projeto");
        let r = req(&fx);
        let not_listed = apply(&ApplyRequest { base: r.clone(), chapters: vec!["/etc/passwd".into()], notes: vec![], alias_notes: vec![], glossary: false }, &fx.vault);
        assert!(not_listed.is_err());
        assert!(undo(&r.root, &fx.vault, "../x", false).is_err());
    }

    #[test]
    fn nothing_changed_is_an_error_and_leaves_no_backup_folder() {
        let fx = fixture();
        let mut r = req(&fx);
        r.find = "Inexistente".into();
        let result = apply(&ApplyRequest { base: r.clone(), chapters: r.paths.clone(), notes: vec![], alias_notes: vec![], glossary: false }, &fx.vault);
        assert!(result.is_err());
        assert!(!fx.dir.path().join(".livro/substituicoes").exists());
    }
}
