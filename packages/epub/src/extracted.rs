//! Reempacotamento de uma pasta que já contém um EPUB extraído (container.xml + OPF).
//! Preserva todos os arquivos declarados no manifest e aplica só os metadados
//! que o usuário alterou.

use crate::model::{Metadata, Package, Report};
use crate::opf::{self, parse_opf, parse_xml, Opf};
use crate::scan::ScannedFile;
use crate::util::*;
use std::collections::HashSet;
use std::fs;

pub struct ExtractedLayout {
    pub opf_path: String,
    pub opf: Opf,
    pub opf_xml: String,
}

/// Lê `META-INF/container.xml` e o OPF principal. `Ok(None)` quando a pasta não é um EPUB extraído.
pub fn detect(files: &[ScannedFile]) -> Result<Option<ExtractedLayout>, String> {
    let Some(container) = files.iter().find(|file| file.rel == "META-INF/container.xml") else { return Ok(None) };
    let text = fs::read_to_string(&container.abs).map_err(|error| format!("Não foi possível ler container.xml: {error}"))?;
    let doc = parse_xml(&text).map_err(|error| format!("container.xml malformado: {error}"))?;
    let full_path = doc
        .descendants()
        .find(|n| n.is_element() && n.tag_name().name() == "rootfile")
        .and_then(|n| n.attribute("full-path"))
        .ok_or("container.xml não aponta para um arquivo OPF.")?
        .to_string();
    let opf_file = files.iter().find(|file| file.rel == full_path).ok_or_else(|| format!("O OPF indicado em container.xml ({full_path}) não existe na pasta."))?;
    let opf_xml = fs::read_to_string(&opf_file.abs).map_err(|error| format!("Não foi possível ler {full_path}: {error}"))?;
    let opf = parse_opf(&opf_xml).map_err(|error| format!("OPF malformado: {error}"))?;
    Ok(Some(ExtractedLayout { opf_path: full_path, opf, opf_xml }))
}

pub fn build_extracted(layout: &ExtractedLayout, files: &[ScannedFile], wanted: &Metadata, report: &mut Report) -> Result<Package, String> {
    let (opf_xml, warnings) = opf::apply_overrides(&layout.opf_xml, wanted, &utc_timestamp())?;
    for warning in warnings {
        report.warn("META-NOT-APPLIED", Some(&layout.opf_path), warning);
    }
    let mut declared: HashSet<String> = HashSet::new();
    declared.insert(layout.opf_path.clone());
    for item in &layout.opf.manifest {
        let decoded = percent_decode(item.href.split('#').next().unwrap_or(""));
        if let Some(resolved) = resolve_href(&layout.opf_path, &decoded) {
            declared.insert(resolved);
        }
    }

    let mut package = Package { identifier: layout.opf.metadata.identifier.clone(), ..Package::default() };
    for file in files {
        if file.rel == "mimetype" || file.rel == layout.opf_path {
            continue;
        }
        let keep = file.rel.starts_with("META-INF/") || declared.contains(&file.rel);
        if !keep {
            report.info("FILE-UNDECLARED", Some(&file.rel), "Arquivo fora do manifest; não foi incluído no livro.");
            continue;
        }
        let mut data = fs::read(&file.abs).map_err(|error| format!("Não foi possível ler {}: {error}", file.rel))?;
        if matches!(extension(&file.rel).as_str(), "xhtml" | "html" | "htm") {
            if let Ok(text) = String::from_utf8(data.clone()) {
                let (fixed, count) = fix_html_entities(&text);
                if count > 0 {
                    report.info("XHTML-ENTITIES", Some(&file.rel), format!("{count} entidade(s) HTML (como &nbsp;) convertida(s) para o equivalente numérico exigido pelo XHTML."));
                    data = fixed.into_bytes();
                }
            }
        }
        package.set(&file.rel, data);
    }
    package.set(&layout.opf_path, opf_xml.into_bytes());
    // Garante que a identificação final reflita o OPF resultante.
    if let Some(file) = package.get(&layout.opf_path) {
        if let Ok(updated) = parse_opf(&String::from_utf8_lossy(&file.data)) {
            package.identifier = updated.metadata.identifier;
        }
    }
    Ok(package)
}
