//! Acesso seguro a arquivos DENTRO da pasta do projeto aberto.
//! Nenhum comando de gravação aceita caminho absoluto, `..` ou link simbólico
//! que escape da pasta do projeto.

use std::fs;
use std::path::{Component, Path, PathBuf};

/// Resolve `rel` (caminho relativo com `/`) dentro de `root`.
pub fn resolve(root: &str, rel: &str) -> Result<PathBuf, String> {
    let root_path = Path::new(root);
    if !root_path.is_dir() { return Err("A pasta do projeto não existe ou não pode ser acessada.".to_string()); }
    let relative = Path::new(rel);
    if rel.trim().is_empty() || rel.contains(['\\', ':']) || relative.is_absolute() || relative.components().any(|c| !matches!(c, Component::Normal(_))) {
        return Err("Caminho inválido: só são permitidos caminhos relativos dentro do projeto.".to_string());
    }
    let canonical_root = fs::canonicalize(root_path).map_err(|error| format!("Não foi possível acessar a pasta do projeto: {error}"))?;
    let target = canonical_root.join(relative);
    // O ancestral existente mais próximo precisa continuar dentro do projeto (bloqueia links simbólicos).
    let mut probe = target.as_path();
    while !probe.exists() { probe = probe.parent().ok_or("Caminho inválido.")?; }
    let canonical_probe = fs::canonicalize(probe).map_err(|error| format!("Não foi possível validar o caminho: {error}"))?;
    if !canonical_probe.starts_with(&canonical_root) { return Err("O caminho sai da pasta do projeto.".to_string()); }
    Ok(target)
}

/// Garante que um caminho absoluto (vindo da interface) está dentro de `root`.
pub fn ensure_inside(root: &str, absolute: &str) -> Result<PathBuf, String> {
    let canonical_root = fs::canonicalize(root).map_err(|error| format!("Pasta do projeto inacessível: {error}"))?;
    let canonical = fs::canonicalize(absolute).map_err(|error| format!("Arquivo inacessível: {error}"))?;
    if canonical.starts_with(&canonical_root) { Ok(canonical) } else { Err("O arquivo está fora da pasta do projeto.".to_string()) }
}

/// Grava de forma atômica (arquivo temporário + rename), criando pastas se preciso.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() { fs::create_dir_all(parent).map_err(|error| format!("Não foi possível criar a pasta: {error}"))?; }
    let temp = path.with_extension(format!("tmp-{}", std::process::id()));
    fs::write(&temp, bytes).map_err(|error| format!("Não foi possível gravar: {error}"))?;
    fs::rename(&temp, path).map_err(|error| { let _ = fs::remove_file(&temp); format!("Não foi possível concluir a gravação: {error}") })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_plain_relative_paths_and_rejects_escapes() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_str().unwrap();
        assert!(resolve(root, "glossario.json").is_ok());
        assert!(resolve(root, ".livro-notas/Personagens/Ana.md").is_ok());
        for bad in ["../fora.txt", "/etc/passwd", "a/../../b", "", "C:\\x", "./a/../b"] {
            assert!(resolve(root, bad).is_err(), "{bad}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn symlinks_cannot_escape_the_project() {
        let dir = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(outside.path(), dir.path().join("atalho")).unwrap();
        assert!(resolve(dir.path().to_str().unwrap(), "atalho/segredo.md").is_err());
    }

    #[test]
    fn ensure_inside_checks_absolute_paths() {
        let dir = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.xhtml"), "x").unwrap();
        fs::write(other.path().join("b.xhtml"), "x").unwrap();
        let root = dir.path().to_str().unwrap();
        assert!(ensure_inside(root, dir.path().join("a.xhtml").to_str().unwrap()).is_ok());
        assert!(ensure_inside(root, other.path().join("b.xhtml").to_str().unwrap()).is_err());
    }

    #[test]
    fn atomic_write_replaces_content_and_leaves_no_temp_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("sub/a.json");
        write_atomic(&file, b"1").unwrap();
        write_atomic(&file, b"22").unwrap();
        assert_eq!(fs::read(&file).unwrap(), b"22");
        assert_eq!(fs::read_dir(dir.path().join("sub")).unwrap().count(), 1);
    }
}
