//! Estado do projeto guardado NA PASTA do projeto (`.livro-projeto.json`).
//!
//! Tudo que descreve o livro (gênero, modo tradução, posição de leitura,
//! original carregado, metadados do EPUB…) mora aqui, não no navegador do app:
//! reinstalar, atualizar ou formatar o computador não perde nada enquanto a
//! pasta do projeto existir. O arquivo é JSON legível, com chaves em português.
//!
//! Gravação por seção: cada tela atualiza só a sua parte, relendo o arquivo,
//! então telas diferentes nunca sobrescrevem as alterações umas das outras.

use crate::project_files;
use serde::Serialize;
use serde_json::{json, Map, Value};
use std::fs;

pub const FILE_NAME: &str = ".livro-projeto.json";
const BACKUP_NAME: &str = ".livro-projeto.json.bak";
const FORMAT_VERSION: u64 = 1;
const MAX_BYTES: u64 = 2 * 1024 * 1024;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Loaded {
    pub exists: bool,
    /// Objeto com as seções do projeto (vazio se o arquivo não existe).
    pub state: Value,
    /// Preenchido quando o arquivo existe mas não pôde ser lido; nada é alterado.
    pub error: Option<String>,
}

fn read_object(root: &str) -> Result<Option<Map<String, Value>>, String> {
    let path = project_files::resolve(root, FILE_NAME)?;
    if !path.is_file() { return Ok(None); }
    if fs::metadata(&path).map(|m| m.len()).unwrap_or(0) > MAX_BYTES { return Err("O arquivo do projeto é grande demais para ser lido.".to_string()); }
    let text = fs::read_to_string(&path).map_err(|error| format!("Não foi possível ler {FILE_NAME}: {error}"))?;
    match serde_json::from_str::<Value>(&text) {
        Ok(Value::Object(map)) => Ok(Some(map)),
        Ok(_) => Err(format!("{FILE_NAME} não contém um objeto JSON. Nada foi alterado; corrija ou apague o arquivo.")),
        Err(error) => Err(format!("{FILE_NAME} está com erro de formatação ({error}). Nada foi alterado; corrija ou restaure {BACKUP_NAME}.")),
    }
}

pub fn load(root: &str) -> Result<Loaded, String> {
    match read_object(root) {
        Ok(Some(map)) => Ok(Loaded { exists: true, state: Value::Object(map), error: None }),
        Ok(None) => Ok(Loaded { exists: false, state: json!({}), error: None }),
        // Erro de leitura NÃO é erro do comando: a interface precisa continuar com padrões e avisar.
        Err(message) if project_files::resolve(root, FILE_NAME).map(|p| p.is_file()).unwrap_or(false) => Ok(Loaded { exists: true, state: json!({}), error: Some(message) }),
        Err(message) => Err(message),
    }
}

fn valid_section(name: &str) -> bool {
    !name.is_empty() && name.len() <= 32 && name.chars().all(|c| c.is_ascii_lowercase() || c == '_') && !matches!(name, "versao" | "atualizado_em")
}

/// Substitui uma seção (ou a remove, se `value` for `null`), preservando as demais.
pub fn patch(root: &str, section: &str, value: Value) -> Result<(), String> {
    if !valid_section(section) { return Err("Nome de seção inválido.".to_string()); }
    let path = project_files::resolve(root, FILE_NAME)?;
    let mut map = read_object(root)?.unwrap_or_default();
    // Cópia de segurança antes de mudanças "grandes" (a posição de leitura muda a todo instante).
    if path.is_file() && section != "leitura" { let _ = fs::copy(&path, path.with_file_name(BACKUP_NAME)); }
    if value.is_null() { map.remove(section); } else { map.insert(section.to_string(), value); }
    map.insert("versao".to_string(), json!(FORMAT_VERSION));
    map.insert("atualizado_em".to_string(), json!(book_epub::now_utc()));
    let text = serde_json::to_string_pretty(&Value::Object(map)).map_err(|error| error.to_string())?;
    project_files::write_atomic(&path, text.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir() -> tempfile::TempDir { tempfile::tempdir().unwrap() }
    fn r(d: &tempfile::TempDir) -> &str { d.path().to_str().unwrap() }

    #[test]
    fn missing_file_loads_as_empty_without_error() {
        let d = dir();
        let loaded = load(r(&d)).unwrap();
        assert!(!loaded.exists && loaded.error.is_none() && loaded.state == json!({}));
    }

    #[test]
    fn patches_sections_independently_and_keeps_the_rest() {
        let d = dir();
        patch(r(&d), "configuracoes", json!({"genero": "romance", "modoTraducao": true})).unwrap();
        patch(r(&d), "leitura", json!({"documento": "text/cap1.xhtml", "pagina": 7})).unwrap();
        patch(r(&d), "configuracoes", json!({"genero": "thriller"})).unwrap();
        let state = load(r(&d)).unwrap().state;
        assert_eq!(state["configuracoes"], json!({"genero": "thriller"}));
        assert_eq!(state["leitura"]["pagina"], 7);
        assert_eq!(state["versao"], 1);
        assert!(state["atualizado_em"].as_str().unwrap().ends_with('Z'));
        patch(r(&d), "leitura", Value::Null).unwrap();
        assert!(load(r(&d)).unwrap().state.get("leitura").is_none());
        assert!(load(r(&d)).unwrap().state.get("configuracoes").is_some());
    }

    #[test]
    fn unknown_future_sections_survive_and_names_are_validated() {
        let d = dir();
        fs::write(d.path().join(FILE_NAME), r#"{"versao":1,"biblia":{"titulo":"X"}}"#).unwrap();
        patch(r(&d), "epub", json!({"capa": "capa.jpg"})).unwrap();
        let state = load(r(&d)).unwrap().state;
        assert_eq!(state["biblia"]["titulo"], "X");
        for bad in ["", "../x", "Maiuscula", "versao", "a b", &"x".repeat(40)] { assert!(patch(r(&d), bad, json!(1)).is_err(), "{bad}"); }
    }

    #[test]
    fn corrupted_file_is_reported_and_never_overwritten() {
        let d = dir();
        let file = d.path().join(FILE_NAME);
        fs::write(&file, "{ isto não é json").unwrap();
        let loaded = load(r(&d)).unwrap();
        assert!(loaded.exists && loaded.error.as_deref().unwrap().contains("erro de formatação"));
        assert!(patch(r(&d), "configuracoes", json!({})).is_err());
        assert_eq!(fs::read_to_string(&file).unwrap(), "{ isto não é json", "o arquivo do usuário fica intacto");
    }

    #[test]
    fn keeps_a_backup_of_the_previous_version_except_for_reading_position() {
        let d = dir();
        patch(r(&d), "configuracoes", json!({"genero": "memoir"})).unwrap();
        patch(r(&d), "epub", json!({"capa": "a.jpg"})).unwrap();
        let backup: Value = serde_json::from_str(&fs::read_to_string(d.path().join(BACKUP_NAME)).unwrap()).unwrap();
        assert!(backup.get("configuracoes").is_some() && backup.get("epub").is_none(), "o .bak é a versão anterior");
        patch(r(&d), "leitura", json!({"pagina": 1})).unwrap();
        let again: Value = serde_json::from_str(&fs::read_to_string(d.path().join(BACKUP_NAME)).unwrap()).unwrap();
        assert!(again.get("epub").is_none(), "posição de leitura não gera backup");
    }

    #[test]
    fn rejects_paths_outside_and_non_object_files() {
        let d = dir();
        fs::write(d.path().join(FILE_NAME), "[1,2]").unwrap();
        assert!(load(r(&d)).unwrap().error.is_some());
        assert!(patch("/nao/existe/mesmo", "epub", json!(1)).is_err());
    }
}
