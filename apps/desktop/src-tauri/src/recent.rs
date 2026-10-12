//! Projetos abertos recentemente ("Abrir último trabalho").
//!
//! A lista fica na pasta de dados do APP (`recentes.json`), não no navegador: sobrevive a
//! atualizar/reinstalar o app. A posição de leitura (capítulo e página) NÃO é guardada aqui:
//! ela já mora em `<projeto>/.livro/projeto.json` e é lida na hora, então nunca fica defasada.

use crate::project_files;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

const FILE_NAME: &str = "recentes.json";
const MAX_RECENT: usize = 8;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Stored { path: String, opened_at: String }

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Recent {
    pub path: String,
    pub name: String,
    pub opened_at: String,
    /// A pasta ainda existe? (movida, apagada ou unidade desconectada = falso)
    pub exists: bool,
    /// Capítulo (relativo à pasta) e página onde a leitura parou, se o projeto já gravou isso.
    pub document: Option<String>,
    pub page: Option<u64>,
}

fn same(a: &str, b: &str) -> bool {
    let norm = |p: &str| p.replace('\\', "/").trim_end_matches('/').to_lowercase();
    norm(a) == norm(b)
}

fn read_stored(dir: &Path) -> Vec<Stored> {
    let path = dir.join(FILE_NAME);
    let Ok(text) = fs::read_to_string(&path) else { return Vec::new() };
    match serde_json::from_str::<Vec<Stored>>(&text) {
        Ok(list) => list,
        Err(_) => { let _ = fs::copy(&path, dir.join("recentes.json.bak")); Vec::new() } // ilegível: guarda cópia e recomeça
    }
}

fn describe(item: &Stored) -> Recent {
    let folder = Path::new(&item.path);
    let name = folder.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| item.path.clone());
    let resume = fs::read_to_string(folder.join(".livro/projeto.json")).ok().and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok()).and_then(|v| v.get("leitura").cloned());
    Recent {
        path: item.path.clone(), name, opened_at: item.opened_at.clone(), exists: folder.is_dir(),
        document: resume.as_ref().and_then(|l| l.get("documento")).and_then(|d| d.as_str()).map(str::to_string),
        page: resume.as_ref().and_then(|l| l.get("pagina")).and_then(|p| p.as_u64()),
    }
}

pub fn list(dir: &Path) -> Vec<Recent> { read_stored(dir).iter().map(describe).collect() }

/// A pasta está na lista de recentes? (só essas têm o acesso de leitura reaberto.)
pub fn contains(dir: &Path, project: &str) -> bool { read_stored(dir).iter().any(|item| same(&item.path, project)) }

/// Move (ou acrescenta) o projeto para o topo da lista.
pub fn touch(dir: &Path, project: &str) -> Result<Vec<Recent>, String> {
    let mut items = read_stored(dir);
    items.retain(|item| !same(&item.path, project));
    items.insert(0, Stored { path: project.to_string(), opened_at: book_epub::now_utc() });
    items.truncate(MAX_RECENT);
    let json = serde_json::to_string_pretty(&items).map_err(|e| e.to_string())?;
    project_files::write_atomic(&dir.join(FILE_NAME), json.as_bytes())?;
    Ok(items.iter().map(describe).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn touch_orders_by_recency_dedupes_and_caps() {
        let app = tempfile::tempdir().unwrap();
        let a = tempfile::tempdir().unwrap();
        let b = tempfile::tempdir().unwrap();
        touch(app.path(), a.path().to_str().unwrap()).unwrap();
        touch(app.path(), b.path().to_str().unwrap()).unwrap();
        // A mesma pasta com outra grafia de caixa/barras não duplica.
        let variant = a.path().to_str().unwrap().to_uppercase();
        assert_eq!(touch(app.path(), &variant).unwrap().len(), 2);
        let again = touch(app.path(), a.path().to_str().unwrap()).unwrap();
        assert_eq!(again.len(), 2, "A e B, sem duplicata");
        assert_eq!(again[0].path, a.path().to_str().unwrap());
        for i in 0..12 { touch(app.path(), &format!("/nao/existe/{i}")).unwrap(); }
        assert_eq!(list_len(app.path()), MAX_RECENT);
    }

    fn list_len(dir: &Path) -> usize { list(dir).len() }

    #[test]
    fn reads_resume_position_from_the_project_file_and_flags_missing_folders() {
        let app = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();
        fs::create_dir_all(project.path().join(".livro")).unwrap();
        fs::write(project.path().join(".livro/projeto.json"), r#"{"versao":1,"leitura":{"documento":"cap3.xhtml","pagina":12}}"#).unwrap();
        touch(app.path(), project.path().to_str().unwrap()).unwrap();
        touch(app.path(), "/pasta/que/sumiu").unwrap();
        let items = list(app.path());
        assert!(!items[0].exists && items[0].document.is_none());
        assert!(items[1].exists);
        assert_eq!((items[1].document.as_deref(), items[1].page), (Some("cap3.xhtml"), Some(12)));
    }

    #[test]
    fn unreadable_file_is_backed_up_and_does_not_block_new_entries() {
        let app = tempfile::tempdir().unwrap();
        fs::write(app.path().join(FILE_NAME), "{ quebrado").unwrap();
        assert!(list(app.path()).is_empty());
        assert!(app.path().join("recentes.json.bak").is_file());
        touch(app.path(), "/x/livro").unwrap();
        assert_eq!(list(app.path()).len(), 1);
    }

    #[test]
    fn same_path_compares_slashes_and_case() {
        assert!(same("D:\\Livros\\A\\", "d:/livros/a"));
        assert!(!same("D:\\Livros\\A", "D:\\Livros\\B"));
    }
}
