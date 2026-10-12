//! Leitura de texto de capítulos: pastas (XHTML/HTML/TXT/MD) e arquivos .epub.
//! Usado para carregar o texto original numa revisão de tradução e para
//! sugerir termos de glossário.

use crate::opf::{parse_opf, parse_xml};
use crate::scan::collect_files;
use crate::util::*;
use roxmltree::Node;
use serde::Serialize;
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChapterText {
    /// Nome do arquivo do capítulo (sem pasta).
    pub name: String,
    /// Uma entrada por parágrafo.
    pub lines: Vec<String>,
}

const BLOCKS: [&str; 12] = ["p", "h1", "h2", "h3", "h4", "h5", "h6", "li", "blockquote", "div", "section", "td"];
const MAX_ENTRY_BYTES: u64 = 32 * 1024 * 1024;
const MAX_ENTRIES: usize = 5000;

fn normalize_line(text: &str) -> String {
    text.replace('\u{a0}', " ").split_whitespace().collect::<Vec<_>>().join(" ")
}

fn is_block(node: Node) -> bool {
    node.is_element() && BLOCKS.contains(&node.tag_name().name())
}

/// Texto de um documento XHTML/HTML, um item por bloco (parágrafo, título, item de lista).
pub fn lines_from_xhtml(source: &str) -> Vec<String> {
    let attempt = |text: &str| -> Option<Vec<String>> {
        let doc = parse_xml(text).ok()?;
        let body = doc.descendants().find(|n| n.is_element() && n.tag_name().name() == "body").unwrap_or(doc.root_element());
        let mut lines = Vec::new();
        for node in body.descendants().filter(|n| is_block(*n)) {
            // Só blocos "folha": o texto de um div que contém parágrafos já vem dos parágrafos.
            if node.descendants().skip(1).any(is_block) { continue; }
            let text = normalize_line(&node.descendants().filter(|n| n.is_text()).map(|n| n.text().unwrap_or("")).collect::<String>());
            if !text.is_empty() { lines.push(text); }
        }
        if lines.is_empty() {
            let text = normalize_line(&body.descendants().filter(|n| n.is_text()).map(|n| n.text().unwrap_or("")).collect::<String>());
            if !text.is_empty() { lines.push(text); }
        }
        Some(lines)
    };
    if let Some(lines) = attempt(source) { return lines; }
    let (fixed, _) = fix_html_entities(source);
    if let Some(lines) = attempt(&fixed) { return lines; }
    // HTML que não é XML bem formado: remove tags preservando o fim de parágrafo.
    let mut text = String::new();
    let mut in_tag = false;
    let mut tag = String::new();
    for ch in source.chars() {
        match ch {
            '<' => { in_tag = true; tag.clear(); }
            '>' if in_tag => {
                in_tag = false;
                let name = tag.trim_start_matches('/').split(|c: char| c.is_whitespace() || c == '/').next().unwrap_or("").to_ascii_lowercase();
                if tag.starts_with('/') && BLOCKS.contains(&name.as_str()) || name == "br" { text.push('\n'); }
            }
            _ if in_tag => tag.push(ch),
            _ => text.push(ch),
        }
    }
    text.replace("&nbsp;", " ").replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">").lines().map(normalize_line).filter(|l| !l.is_empty()).collect()
}

fn lines_from_plain(text: &str) -> Vec<String> {
    text.lines().map(normalize_line).filter(|l| !l.is_empty()).collect()
}

fn chapter_name(path: &str) -> String {
    path.rsplit('/').next().unwrap_or(path).to_string()
}

/// Lê os capítulos de `path`: um .epub (na ordem da spine) ou uma pasta de arquivos de texto.
pub fn read_source(path: &str) -> Result<Vec<ChapterText>, String> {
    let source = Path::new(path);
    if source.is_dir() { return read_folder(source); }
    if source.extension().and_then(|e| e.to_str()).map(|e| e.eq_ignore_ascii_case("epub")).unwrap_or(false) {
        return read_epub(source);
    }
    Err("Escolha uma pasta de capítulos ou um arquivo .epub.".to_string())
}

fn read_folder(root: &Path) -> Result<Vec<ChapterText>, String> {
    let mut chapters = Vec::new();
    for file in collect_files(root)? {
        let ext = extension(&file.rel);
        if !matches!(ext.as_str(), "xhtml" | "html" | "htm" | "txt" | "md") { continue; }
        let text = fs::read_to_string(&file.abs).map_err(|error| format!("Não foi possível ler {}: {error}", file.rel))?;
        let lines = if matches!(ext.as_str(), "txt" | "md") { lines_from_plain(&text) } else { lines_from_xhtml(&text) };
        if !lines.is_empty() { chapters.push(ChapterText { name: chapter_name(&file.rel), lines }); }
    }
    if chapters.is_empty() { return Err("Nenhum capítulo (.xhtml, .html, .txt ou .md) com texto foi encontrado nessa pasta.".to_string()); }
    Ok(chapters)
}

fn read_entry(archive: &mut zip::ZipArchive<File>, name: &str) -> Result<String, String> {
    let entry = archive.by_name(name).map_err(|_| format!("O EPUB não contém {name}."))?;
    if entry.size() > MAX_ENTRY_BYTES { return Err(format!("{name} é grande demais para ser lido.")); }
    let mut text = String::new();
    entry.take(MAX_ENTRY_BYTES).read_to_string(&mut text).map_err(|error| format!("Não foi possível ler {name}: {error}"))?;
    Ok(text)
}

fn read_epub(path: &Path) -> Result<Vec<ChapterText>, String> {
    let file = File::open(path).map_err(|error| format!("Não foi possível abrir o EPUB: {error}"))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|error| format!("Arquivo EPUB inválido: {error}"))?;
    if archive.len() > MAX_ENTRIES { return Err("O EPUB tem arquivos demais.".to_string()); }
    let container = read_entry(&mut archive, "META-INF/container.xml")?;
    let opf_path = parse_xml(&container).ok().and_then(|doc| doc.descendants().find(|n| n.is_element() && n.tag_name().name() == "rootfile").and_then(|n| n.attribute("full-path").map(str::to_string))).ok_or("O EPUB não indica o arquivo OPF.")?;
    let opf = parse_opf(&read_entry(&mut archive, &opf_path)?)?;
    let mut chapters = Vec::new();
    for itemref in &opf.spine {
        let Some(item) = opf.manifest.iter().find(|i| i.id == itemref.idref) else { continue };
        if item.media_type != "application/xhtml+xml" || item.properties.iter().any(|p| p == "nav") { continue; }
        let Some(resolved) = resolve_href(&opf_path, &percent_decode(item.href.split('#').next().unwrap_or(""))) else { continue };
        let Ok(text) = read_entry(&mut archive, &resolved) else { continue };
        let lines = lines_from_xhtml(&text);
        if !lines.is_empty() { chapters.push(ChapterText { name: chapter_name(&resolved), lines }); }
    }
    if chapters.is_empty() { return Err("O EPUB não tem capítulos com texto.".to_string()); }
    Ok(chapters)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_one_line_per_leaf_block() {
        let xhtml = "<?xml version=\"1.0\"?><!DOCTYPE html><html xmlns=\"http://www.w3.org/1999/xhtml\"><body><div class=\"c\"><p class=\"t\">CAP&#160;1</p><p>Era <em>uma</em>  vez.</p></div><h2>Fim</h2></body></html>";
        assert_eq!(lines_from_xhtml(xhtml), vec!["CAP 1", "Era uma vez.", "Fim"]);
    }

    #[test]
    fn tolerates_html_that_is_not_xml() {
        let html = "<html><body><p>Um&nbsp;dois<br>tres</p><p>Quatro</p></body></html>";
        assert_eq!(lines_from_xhtml(html), vec!["Um dois", "tres", "Quatro"]);
    }
}
