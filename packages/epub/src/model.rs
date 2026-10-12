//! Tipos públicos trocados com o aplicativo (serializados em camelCase).

use serde::{Deserialize, Serialize};

/// Metadados editáveis. Texto vazio significa "não informado": ao reempacotar
/// um EPUB existente, somente campos preenchidos substituem o OPF original.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Metadata {
    pub title: String,
    pub authors: Vec<String>,
    pub language: String,
    pub publisher: String,
    pub description: String,
    pub rights: String,
    pub identifier: String,
    pub series: String,
    pub series_index: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct BuildRequest {
    /// Pasta do projeto aberto no editor.
    pub root: String,
    pub metadata: Metadata,
    /// Capítulos na ordem de leitura (caminhos absolutos ou relativos à raiz).
    /// Só é usado quando a pasta não contém um EPUB extraído.
    pub chapters: Vec<String>,
    /// Imagem de capa opcional (caminho absoluto).
    pub cover: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectMode {
    /// A pasta contém META-INF/container.xml e um OPF: reempacota fielmente.
    Extracted,
    /// A pasta só tem XHTML/CSS/imagens: o pacote (OPF, nav, NCX) é gerado.
    Loose,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectInfo {
    pub mode: ProjectMode,
    pub opf_path: Option<String>,
    pub epub_version: Option<String>,
    /// Metadados encontrados no OPF (vazios no modo `Loose`).
    pub metadata: Metadata,
    pub chapter_count: usize,
    pub has_cover: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    pub severity: Severity,
    pub code: String,
    pub file: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub issues: Vec<Issue>,
    pub errors: usize,
    pub warnings: usize,
}

impl Report {
    pub fn push(&mut self, severity: Severity, code: &str, file: Option<&str>, message: impl Into<String>) {
        match severity {
            Severity::Error => self.errors += 1,
            Severity::Warning => self.warnings += 1,
            Severity::Info => {}
        }
        self.issues.push(Issue { severity, code: code.to_string(), file: file.map(str::to_string), message: message.into() });
    }
    pub fn error(&mut self, code: &str, file: Option<&str>, message: impl Into<String>) {
        self.push(Severity::Error, code, file, message);
    }
    pub fn warn(&mut self, code: &str, file: Option<&str>, message: impl Into<String>) {
        self.push(Severity::Warning, code, file, message);
    }
    pub fn info(&mut self, code: &str, file: Option<&str>, message: impl Into<String>) {
        self.push(Severity::Info, code, file, message);
    }
    pub fn can_build(&self) -> bool {
        self.errors == 0
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildOutcome {
    pub path: String,
    pub size_bytes: u64,
    pub file_count: usize,
    pub identifier: String,
    pub mode: ProjectMode,
    pub report: Report,
}

/// Arquivo do pacote final (`path` é o caminho dentro do ZIP).
#[derive(Debug, Clone)]
pub struct PackageFile {
    pub path: String,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Default)]
pub struct Package {
    pub files: Vec<PackageFile>,
    pub identifier: String,
}

impl Package {
    pub fn get(&self, path: &str) -> Option<&PackageFile> {
        self.files.iter().find(|file| file.path == path)
    }
    pub fn set(&mut self, path: &str, data: Vec<u8>) {
        match self.files.iter_mut().find(|file| file.path == path) {
            Some(file) => file.data = data,
            None => self.files.push(PackageFile { path: path.to_string(), data }),
        }
    }
}
