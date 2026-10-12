//! Snapshots do projeto: cópias restauráveis do que é difícil de refazer
//! (estado do projeto, glossário e caderno), tiradas antes de operações em lote
//! (importar bíblia, aplicar recap, renomear, excluir). Ficam em `.livro/snapshots/`.

use crate::project_files;
use serde::Serialize;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use zip::write::SimpleFileOptions;

const DIR: &str = ".livro/snapshots";
const KEEP: usize = 30;
const MAX_FILE_BYTES: u64 = 5 * 1024 * 1024;
const MAX_TOTAL_BYTES: u64 = 200 * 1024 * 1024;
/// Intervalo mínimo entre snapshots automáticos.
pub const AUTO_INTERVAL_SECS: u64 = 300;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotInfo {
    pub name: String,
    pub reason: String,
    pub created_at: String,
    pub size_bytes: u64,
}

fn stamp() -> String { book_epub::now_utc().replace(['-', ':'], "").replace('T', "-").trim_end_matches('Z').to_string() }

fn slug(reason: &str) -> String {
    let s: String = reason.to_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect();
    let s = s.split('-').filter(|p| !p.is_empty()).collect::<Vec<_>>().join("-");
    if s.is_empty() { "manual".to_string() } else { s.chars().take(32).collect() }
}

fn valid_name(name: &str) -> bool {
    name.ends_with(".zip") && name.len() <= 80 && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.')
}

fn info_for(path: &Path) -> Option<SnapshotInfo> {
    let name = path.file_name()?.to_str()?.to_string();
    if !valid_name(&name) { return None; }
    let stem = name.trim_end_matches(".zip").to_string();
    let (date, rest) = stem.split_once('-')?;
    let (time, reason) = rest.split_once('-').unwrap_or((rest, "manual"));
    let created_at = format!("{}-{}-{}T{}:{}:{}Z", date.get(0..4)?, date.get(4..6)?, date.get(6..8)?, time.get(0..2)?, time.get(2..4)?, time.get(4..6)?);
    Some(SnapshotInfo { name, reason: reason.to_string(), created_at, size_bytes: fs::metadata(path).ok()?.len() })
}

pub fn list(root: &str) -> Result<Vec<SnapshotInfo>, String> {
    let dir = project_files::resolve(root, DIR)?;
    if !dir.is_dir() { return Ok(Vec::new()); }
    let mut out: Vec<SnapshotInfo> = fs::read_dir(&dir).map_err(|e| e.to_string())?.filter_map(|e| e.ok()).filter_map(|e| info_for(&e.path())).collect();
    out.sort_by(|a, b| b.name.cmp(&a.name));
    Ok(out)
}

fn collect_md(base: &Path, rel: &str, out: &mut Vec<(String, PathBuf)>, total: &mut u64) {
    let Ok(entries) = fs::read_dir(base) else { return };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Ok(meta) = fs::symlink_metadata(entry.path()) else { continue };
        if meta.file_type().is_symlink() || name.starts_with('.') { continue; }
        let relative = if rel.is_empty() { name.clone() } else { format!("{rel}/{name}") };
        if meta.is_dir() { collect_md(&entry.path(), &relative, out, total); }
        else if name.ends_with(".md") && meta.len() <= MAX_FILE_BYTES && *total + meta.len() <= MAX_TOTAL_BYTES { *total += meta.len(); out.push((format!("vault/{relative}"), entry.path())); }
    }
}

/// Cria um snapshot. `vault_dir` é a pasta do caderno (pode estar fora do projeto).
pub fn create(root: &str, vault_dir: Option<&Path>, reason: &str) -> Result<SnapshotInfo, String> {
    let dir = project_files::resolve(root, DIR)?;
    fs::create_dir_all(&dir).map_err(|e| format!("Não foi possível criar a pasta de snapshots: {e}"))?;
    let mut files: Vec<(String, PathBuf)> = Vec::new();
    let mut total = 0u64;
    for (entry, rel) in [("projeto.json", ".livro/projeto.json"), ("glossario.json", "glossario.json")] {
        let path = project_files::resolve(root, rel)?;
        if path.is_file() { files.push((entry.to_string(), path)); }
    }
    if let Some(vault) = vault_dir { collect_md(vault, "", &mut files, &mut total); }
    let name = format!("{}-{}.zip", stamp(), slug(reason));
    let path = dir.join(&name);
    let temp = dir.join(format!("{name}.tmp"));
    let result = (|| -> Result<(), String> {
        let file = File::create(&temp).map_err(|e| e.to_string())?;
        let mut zip = zip::ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for (entry, source) in &files {
            let bytes = fs::read(source).map_err(|e| format!("{entry}: {e}"))?;
            zip.start_file(entry.as_str(), options).map_err(|e| e.to_string())?;
            zip.write_all(&bytes).map_err(|e| e.to_string())?;
        }
        let file = zip.finish().map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())
    })();
    if let Err(error) = result { let _ = fs::remove_file(&temp); return Err(format!("Não foi possível criar o snapshot: {error}")); }
    fs::rename(&temp, &path).map_err(|e| { let _ = fs::remove_file(&temp); e.to_string() })?;
    // Mantém só os mais recentes.
    let all = list(root)?;
    for old in all.iter().skip(KEEP) { let _ = fs::remove_file(dir.join(&old.name)); }
    info_for(&path).ok_or_else(|| "Snapshot criado com nome inesperado.".to_string())
}

/// Snapshot automático antes de operações em lote; não repete dentro de `AUTO_INTERVAL_SECS`.
pub fn auto(root: &str, vault_dir: Option<&Path>, reason: &str) -> Result<Option<SnapshotInfo>, String> {
    if let Some(last) = list(root)?.first() {
        let age = book_epub::now_utc();
        if seconds_between(&last.created_at, &age).map(|s| s < AUTO_INTERVAL_SECS as i64).unwrap_or(false) { return Ok(None); }
    }
    create(root, vault_dir, reason).map(Some)
}

fn seconds(stamp: &str) -> Option<i64> {
    let (date, time) = stamp.trim_end_matches('Z').split_once('T')?;
    let d: Vec<i64> = date.split('-').filter_map(|p| p.parse().ok()).collect();
    let t: Vec<i64> = time.split(':').filter_map(|p| p.parse().ok()).collect();
    if d.len() != 3 || t.len() != 3 { return None; }
    // Dias desde 1970 (algoritmo civil → dias).
    let (y, m) = if d[1] <= 2 { (d[0] - 1, d[1] + 9) } else { (d[0], d[1] - 3) };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * m + 2) / 5 + d[2] - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some((era * 146_097 + doe - 719_468) * 86_400 + t[0] * 3600 + t[1] * 60 + t[2])
}

fn seconds_between(from: &str, to: &str) -> Option<i64> { Some(seconds(to)? - seconds(from)?) }

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreResult { pub restored: usize, pub safety_snapshot: String }

/// Restaura um snapshot por cima dos arquivos atuais. Antes, guarda o estado atual
/// em outro snapshot. Não apaga arquivos que não existam no snapshot.
pub fn restore(root: &str, vault_dir: Option<&Path>, name: &str) -> Result<RestoreResult, String> {
    if !valid_name(name) { return Err("Nome de snapshot inválido.".to_string()); }
    let source = project_files::resolve(root, &format!("{DIR}/{name}"))?;
    if !source.is_file() { return Err("O snapshot não existe mais.".to_string()); }
    let safety = create(root, vault_dir, "antes-de-restaurar")?;
    let mut archive = zip::ZipArchive::new(File::open(&source).map_err(|e| e.to_string())?).map_err(|e| format!("Snapshot ilegível: {e}"))?;
    let mut restored = 0;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|e| e.to_string())?;
        let entry_name = entry.name().to_string();
        if entry.size() > MAX_FILE_BYTES * 2 { continue; }
        let target: PathBuf = if entry_name == "projeto.json" { project_files::resolve(root, ".livro/projeto.json")? }
            else if entry_name == "glossario.json" { project_files::resolve(root, "glossario.json")? }
            else if let Some(rel) = entry_name.strip_prefix("vault/") {
                let Some(vault) = vault_dir else { continue };
                if Path::new(rel).components().any(|c| !matches!(c, Component::Normal(_))) { return Err(format!("Entrada insegura no snapshot: {entry_name}")); }
                fs::create_dir_all(vault).map_err(|e| e.to_string())?;
                project_files::resolve(&vault.to_string_lossy(), rel)?
            } else { continue };
        let mut bytes = Vec::new();
        entry.by_ref().take(MAX_FILE_BYTES * 2).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
        project_files::write_atomic(&target, &bytes)?;
        restored += 1;
    }
    Ok(RestoreResult { restored, safety_snapshot: safety.name })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join(".livro")).unwrap();
        fs::write(dir.path().join(".livro/projeto.json"), r#"{"versao":1}"#).unwrap();
        fs::write(dir.path().join("glossario.json"), r#"{"nao_traduzir":["Kix"]}"#).unwrap();
        let vault = dir.path().join(".livro-notas");
        fs::create_dir_all(vault.join("Personagens")).unwrap();
        fs::write(vault.join("Personagens/Aria.md"), "# Aria\nferida").unwrap();
        fs::write(vault.join("ignorado.txt"), "x").unwrap();
        (dir, vault)
    }

    #[test]
    fn creates_lists_and_names_snapshots() {
        let (dir, vault) = setup();
        let root = dir.path().to_str().unwrap();
        assert!(list(root).unwrap().is_empty());
        let info = create(root, Some(&vault), "Importar Bíblia!").unwrap();
        assert!(info.name.ends_with("-importar-b-blia.zip"), "{}", info.name);
        assert!(info.created_at.ends_with('Z') && info.size_bytes > 0);
        let listed = list(root).unwrap();
        assert_eq!(listed.len(), 1);
        let archive = zip::ZipArchive::new(File::open(dir.path().join(".livro/snapshots").join(&info.name)).unwrap()).unwrap();
        let mut names: Vec<String> = archive.file_names().map(str::to_string).collect();
        names.sort();
        assert_eq!(names, vec!["glossario.json", "projeto.json", "vault/Personagens/Aria.md"]);
    }

    #[test]
    fn restore_brings_back_files_and_keeps_a_safety_snapshot() {
        let (dir, vault) = setup();
        let root = dir.path().to_str().unwrap();
        let info = create(root, Some(&vault), "base").unwrap();
        fs::write(vault.join("Personagens/Aria.md"), "# Aria\nESTRAGADO").unwrap();
        fs::write(dir.path().join("glossario.json"), "{}").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(1100));
        let result = restore(root, Some(&vault), &info.name).unwrap();
        assert_eq!(result.restored, 3);
        assert_eq!(fs::read_to_string(vault.join("Personagens/Aria.md")).unwrap(), "# Aria\nferida");
        assert!(fs::read_to_string(dir.path().join("glossario.json")).unwrap().contains("Kix"));
        assert!(list(root).unwrap().iter().any(|s| s.name == result.safety_snapshot), "estado anterior ao restore ficou guardado");
        assert!(restore(root, Some(&vault), "../fora.zip").is_err());
    }

    #[test]
    fn auto_snapshots_are_rate_limited() {
        let (dir, vault) = setup();
        let root = dir.path().to_str().unwrap();
        assert!(auto(root, Some(&vault), "recap").unwrap().is_some());
        assert!(auto(root, Some(&vault), "recap").unwrap().is_none(), "dentro do intervalo não repete");
    }

    #[test]
    fn keeps_only_the_most_recent_snapshots() {
        let (dir, _) = setup();
        let root = dir.path().to_str().unwrap();
        let snap_dir = dir.path().join(".livro/snapshots");
        fs::create_dir_all(&snap_dir).unwrap();
        for day in 1..=(KEEP + 5) { fs::write(snap_dir.join(format!("202601{:02}-000000-antigo.zip", day.min(28)) ), "x").ok(); }
        for n in 0..40 { fs::write(snap_dir.join(format!("20250101-{:06}-velho.zip", n)), "x").unwrap(); }
        create(root, None, "novo").unwrap();
        assert!(list(root).unwrap().len() <= KEEP);
        assert!(list(root).unwrap()[0].reason == "novo");
    }

    #[test]
    fn date_math_matches_known_instants() {
        assert_eq!(seconds("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(seconds("2024-01-01T00:00:00Z"), Some(1_704_067_200));
        assert_eq!(seconds_between("2026-10-12T12:00:00Z", "2026-10-12T12:04:00Z"), Some(240));
    }

    #[test]
    fn zip_slip_entries_are_refused() {
        let (dir, vault) = setup();
        let root = dir.path().to_str().unwrap();
        let snap_dir = dir.path().join(".livro/snapshots");
        fs::create_dir_all(&snap_dir).unwrap();
        let evil = snap_dir.join("20260101-000000-evil.zip");
        let mut zip = zip::ZipWriter::new(File::create(&evil).unwrap());
        zip.start_file("vault/../../fora.md", SimpleFileOptions::default()).unwrap();
        zip.write_all(b"x").unwrap();
        zip.finish().unwrap();
        assert!(restore(root, Some(&vault), "20260101-000000-evil.zip").is_err());
        assert!(!dir.path().parent().unwrap().join("fora.md").exists());
    }
}
