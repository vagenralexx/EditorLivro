//! Núcleo de EPUB do Editor de Livros: inspeção, validação e empacotamento.
//!
//! Dois modos, escolhidos automaticamente pela pasta do projeto:
//! - `Extracted`: a pasta tem `META-INF/container.xml` (EPUB já extraído) →
//!   reempacota fielmente e aplica só os metadados alterados.
//! - `Loose`: a pasta só tem capítulos XHTML/CSS/imagens → gera OPF, nav, NCX e capa.

mod extracted;
mod loose;
mod model;
mod opf;
mod scan;
mod text;
mod util;
mod validate;
mod watermark;
mod write;
mod xhtml;

pub use model::*;
pub use watermark::{detect_in_epub as detect_watermark, Detection, WatermarkInfo, WatermarkOptions};
pub use text::{lines_from_xhtml, read_source, ChapterText};

use std::path::Path;

struct Prepared {
    package: Package,
    mode: ProjectMode,
    report: Report,
    watermark: Option<WatermarkInfo>,
}

fn prepare(request: &BuildRequest) -> Result<Prepared, String> {
    let root = Path::new(&request.root);
    if !root.is_dir() {
        return Err("A pasta do projeto não existe ou não pode ser acessada.".to_string());
    }
    let files = scan::collect_files(root)?;
    let mut report = Report::default();
    let (mut package, mode) = match extracted::detect(&files)? {
        Some(layout) => (extracted::build_extracted(&layout, &files, &request.metadata, &mut report)?, ProjectMode::Extracted),
        None => {
            let cover = request.cover.as_deref().map(Path::new);
            let input = loose::LooseInput { root, files: &files, metadata: &request.metadata, chapters: &request.chapters, cover };
            (loose::build_loose(input, &mut report)?, ProjectMode::Loose)
        }
    };
    if package.identifier.is_empty() {
        package.identifier = request.metadata.identifier.clone();
    }
    let watermark = request.watermark.as_ref().and_then(|options| watermark::apply(&mut package, options, &mut report));
    validate::validate_package(&package, &mut report);
    Ok(Prepared { package, mode, report, watermark })
}

/// Data/hora UTC atual (AAAA-MM-DDThh:mm:ssZ).
pub fn now_utc() -> String { util::utc_timestamp() }

/// Descreve a pasta do projeto: modo, metadados detectados e se há capa.
pub fn inspect(root: &str) -> Result<ProjectInfo, String> {
    let root_path = Path::new(root);
    if !root_path.is_dir() {
        return Err("A pasta do projeto não existe ou não pode ser acessada.".to_string());
    }
    let files = scan::collect_files(root_path)?;
    match extracted::detect(&files)? {
        Some(layout) => {
            let has_cover = layout.opf.manifest.iter().any(|i| i.properties.iter().any(|p| p == "cover-image")) || layout.opf.cover_meta.is_some();
            Ok(ProjectInfo {
                mode: ProjectMode::Extracted,
                opf_path: Some(layout.opf_path.clone()),
                epub_version: Some(layout.opf.version.clone()),
                chapter_count: layout.opf.spine.len(),
                metadata: layout.opf.metadata,
                has_cover,
            })
        }
        None => Ok(ProjectInfo {
            mode: ProjectMode::Loose,
            opf_path: None,
            epub_version: None,
            metadata: Metadata::default(),
            chapter_count: files.iter().filter(|f| matches!(util::extension(&f.rel).as_str(), "xhtml" | "html" | "htm")).count(),
            has_cover: false,
        }),
    }
}

/// Monta o pacote em memória e devolve o relatório, sem gravar nada.
pub fn validate(request: &BuildRequest) -> Result<Report, String> {
    Ok(prepare(request)?.report)
}

/// Gera o `.epub`. Se houver erros de validação, nada é gravado e o relatório é devolvido no erro.
pub fn build(request: &BuildRequest, output: &str) -> Result<BuildOutcome, BuildError> {
    let prepared = prepare(request).map_err(BuildError::Failed)?;
    if !prepared.report.can_build() {
        return Err(BuildError::Invalid(prepared.report));
    }
    let out_path = Path::new(output);
    if out_path.extension().and_then(|e| e.to_str()).map(|e| !e.eq_ignore_ascii_case("epub")).unwrap_or(true) {
        return Err(BuildError::Failed("O arquivo de saída deve ter a extensão .epub.".to_string()));
    }
    let size = write::write_package(&prepared.package, out_path).map_err(BuildError::Failed)?;
    Ok(BuildOutcome {
        path: output.to_string(),
        size_bytes: size,
        file_count: prepared.package.files.len() + 1,
        identifier: prepared.package.identifier,
        mode: prepared.mode,
        report: prepared.report,
        watermark: prepared.watermark,
    })
}

#[derive(Debug)]
pub enum BuildError {
    /// Falha de leitura/gravação ou entrada inválida.
    Failed(String),
    /// O pacote tem erros de validação; nada foi gravado.
    Invalid(Report),
}

impl std::fmt::Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildError::Failed(message) => write!(f, "{message}"),
            BuildError::Invalid(report) => write!(f, "O livro tem {} erro(s) que impedem a geração do EPUB.", report.errors),
        }
    }
}

impl std::error::Error for BuildError {}
