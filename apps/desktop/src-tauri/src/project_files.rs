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

/// Grava de forma durável: arquivo temporário → fsync → rename → fsync da pasta.
/// Sem o fsync, uma queda de energia pode deixar o arquivo vazio em alguns sistemas de arquivos.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write;
    if let Some(parent) = path.parent() { fs::create_dir_all(parent).map_err(|error| format!("Não foi possível criar a pasta: {error}"))?; }
    let temp = path.with_extension(format!("tmp-{}", std::process::id()));
    let result = (|| -> std::io::Result<()> {
        let mut file = fs::File::create(&temp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        Ok(())
    })();
    if let Err(error) = result { let _ = fs::remove_file(&temp); return Err(format!("Não foi possível gravar: {error}")); }
    fs::rename(&temp, path).map_err(|error| { let _ = fs::remove_file(&temp); format!("Não foi possível concluir a gravação: {error}") })?;
    // Persistir a entrada de diretório (não é possível abrir pastas para fsync no Windows).
    #[cfg(unix)]
    if let Some(parent) = path.parent() { if let Ok(dir) = fs::File::open(parent) { let _ = dir.sync_all(); } }
    Ok(())
}

const BACKUP_DIR: &str = ".livro/backups";
const BACKUPS_PER_FILE: usize = 10;

/// Salva um capítulo: valida que está dentro do projeto, guarda cópia do anterior em
/// `.livro/backups/` (as 10 mais recentes por arquivo; antes ficavam espalhadas ao lado do capítulo,
/// sem limite) e grava de forma atômica. Devolve o caminho do backup.
pub fn save_chapter(root: &str, path: &str, content: &str) -> Result<String, String> {
    let source = ensure_inside(root, path)?;
    let ext = source.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    if !matches!(ext.as_str(), "xhtml" | "html" | "htm") { return Err("Por segurança, apenas arquivos XHTML ou HTML podem ser gravados por este comando.".to_string()); }
    if !source.is_file() { return Err("O arquivo original não existe mais ou não pode ser acessado.".to_string()); }
    let canonical_root = fs::canonicalize(root).map_err(|e| format!("Pasta do projeto inacessível: {e}"))?;
    let flat = source.strip_prefix(&canonical_root).map_err(|_| "O arquivo está fora da pasta do projeto.".to_string())?.to_string_lossy().replace(['/', '\\'], "__");
    let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|e| e.to_string())?.as_secs();
    let dir = canonical_root.join(BACKUP_DIR);
    fs::create_dir_all(&dir).map_err(|e| format!("Não foi possível criar a pasta de backups: {e}"))?;
    let backup = dir.join(format!("{flat}.{timestamp}"));
    fs::copy(&source, &backup).map_err(|e| format!("Não foi possível criar o backup antes de salvar: {e}"))?;
    write_atomic(&source, content.as_bytes()).map_err(|e| format!("O backup foi criado, mas não foi possível gravar o XHTML: {e}"))?;
    // Mantém só as mais recentes deste arquivo.
    let prefix = format!("{flat}.");
    let mut mine: Vec<(u64, PathBuf)> = fs::read_dir(&dir).map(|read| read.flatten().filter_map(|entry| {
        let name = entry.file_name().to_string_lossy().into_owned();
        name.strip_prefix(&prefix).and_then(|stamp| stamp.parse::<u64>().ok()).map(|stamp| (stamp, entry.path()))
    }).collect()).unwrap_or_default();
    mine.sort();
    while mine.len() > BACKUPS_PER_FILE { let (_, old) = mine.remove(0); let _ = fs::remove_file(old); }
    Ok(backup.to_string_lossy().into_owned())
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
    fn save_chapter_backs_up_inside_livro_prunes_and_refuses_outsiders() {
        let dir = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        let root = dir.path().to_str().unwrap();
        fs::create_dir_all(dir.path().join("OEBPS")).unwrap();
        let chapter = dir.path().join("OEBPS/c1.xhtml");
        fs::write(&chapter, "v0").unwrap();
        let backup = save_chapter(root, chapter.to_str().unwrap(), "v1").unwrap();
        assert_eq!(fs::read_to_string(&chapter).unwrap(), "v1");
        assert_eq!(fs::read_to_string(&backup).unwrap(), "v0");
        assert!(backup.contains(".livro") && fs::read_dir(dir.path().join("OEBPS")).unwrap().count() == 1, "nada de .backup-* ao lado do capítulo");
        // Muitos saves: só as 10 mais recentes ficam.
        for i in 0..14u64 { let name = format!("OEBPS__c1.xhtml.{}", 1000 + i); fs::write(dir.path().join(".livro/backups").join(name), "x").unwrap(); }
        save_chapter(root, chapter.to_str().unwrap(), "v2").unwrap();
        let count = fs::read_dir(dir.path().join(".livro/backups")).unwrap().count();
        assert_eq!(count, 10);
        // Fora do projeto e extensões erradas são recusados.
        fs::write(other.path().join("x.xhtml"), "x").unwrap();
        assert!(save_chapter(root, other.path().join("x.xhtml").to_str().unwrap(), "y").is_err());
        fs::write(dir.path().join("notas.txt"), "x").unwrap();
        assert!(save_chapter(root, dir.path().join("notas.txt").to_str().unwrap(), "y").is_err());
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
