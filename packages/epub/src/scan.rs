//! Varredura segura da pasta do projeto.

use crate::util::natural_cmp;
use std::fs;
use std::path::{Path, PathBuf};

pub struct ScannedFile {
    /// Caminho relativo à raiz, sempre com `/`.
    pub rel: String,
    pub abs: PathBuf,
}

const MAX_FILE_BYTES: u64 = 256 * 1024 * 1024;

/// Itens que nunca entram no EPUB: ocultos, backups do editor, sistema e pacotes prontos.
fn is_junk(name: &str) -> bool {
    name.starts_with('.')
        || name.contains(".backup-")
        || name.eq_ignore_ascii_case("thumbs.db")
        || name.eq_ignore_ascii_case("desktop.ini")
        || name.to_ascii_lowercase().ends_with(".epub")
        || name.ends_with('~')
}

/// Lista recursivamente os arquivos da pasta. Links simbólicos são ignorados
/// (nunca seguimos um link para fora do projeto).
pub fn collect_files(root: &Path) -> Result<Vec<ScannedFile>, String> {
    let mut found = Vec::new();
    walk(root, "", &mut found)?;
    found.sort_by(|a, b| natural_cmp(&a.rel, &b.rel));
    Ok(found)
}

fn walk(dir: &Path, prefix: &str, out: &mut Vec<ScannedFile>) -> Result<(), String> {
    let entries = fs::read_dir(dir).map_err(|error| format!("Não foi possível ler a pasta {}: {error}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("Falha ao listar {}: {error}", dir.display()))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if is_junk(&name) {
            continue;
        }
        let metadata = fs::symlink_metadata(entry.path()).map_err(|error| format!("Não foi possível inspecionar {name}: {error}"))?;
        if metadata.file_type().is_symlink() {
            continue;
        }
        let rel = if prefix.is_empty() { name.clone() } else { format!("{prefix}/{name}") };
        if metadata.is_dir() {
            walk(&entry.path(), &rel, out)?;
        } else if metadata.is_file() && metadata.len() <= MAX_FILE_BYTES {
            out.push(ScannedFile { rel, abs: entry.path() });
        }
    }
    Ok(())
}
