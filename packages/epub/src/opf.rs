//! Leitura e edição pontual do arquivo OPF.
//!
//! A edição usa os intervalos de bytes do documento original: só o trecho
//! alterado muda, preservando comentários, atributos e a formatação do autor.

use crate::model::Metadata;
use crate::util::xml_escape;
use roxmltree::{Document, Node, ParsingOptions};
use std::ops::Range;

pub const DC_NS: &str = "http://purl.org/dc/elements/1.1/";

#[derive(Debug, Clone)]
pub struct ManifestItem {
    pub id: String,
    pub href: String,
    pub media_type: String,
    pub properties: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SpineRef {
    pub idref: String,
    pub linear: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Opf {
    pub version: String,
    pub unique_identifier: String,
    pub identifiers: Vec<(Option<String>, String)>,
    pub metadata: Metadata,
    pub modified: Option<String>,
    pub manifest: Vec<ManifestItem>,
    pub spine: Vec<SpineRef>,
    pub spine_toc: Option<String>,
    pub cover_meta: Option<String>,
}

pub fn parse_xml(text: &str) -> Result<Document<'_>, String> {
    let options = ParsingOptions { allow_dtd: true, ..ParsingOptions::default() };
    Document::parse_with_options(text, options).map_err(|error| error.to_string())
}

fn text_of(node: Node) -> String {
    node.descendants().filter(|n| n.is_text()).map(|n| n.text().unwrap_or("")).collect::<String>().split_whitespace().collect::<Vec<_>>().join(" ")
}

fn is_dc(node: Node, name: &str) -> bool {
    node.is_element() && node.tag_name().name() == name && node.tag_name().namespace() == Some(DC_NS)
}

pub fn parse_opf(xml: &str) -> Result<Opf, String> {
    let doc = parse_xml(xml)?;
    let package = doc.root_element();
    if package.tag_name().name() != "package" {
        return Err("O arquivo OPF não tem o elemento <package>.".to_string());
    }
    let mut opf = Opf {
        version: package.attribute("version").unwrap_or("").to_string(),
        unique_identifier: package.attribute("unique-identifier").unwrap_or("").to_string(),
        ..Opf::default()
    };
    for node in package.children().filter(Node::is_element) {
        match node.tag_name().name() {
            "metadata" => {
                for child in node.children().filter(Node::is_element) {
                    let value = text_of(child);
                    if is_dc(child, "title") {
                        if opf.metadata.title.is_empty() { opf.metadata.title = value; }
                    } else if is_dc(child, "creator") {
                        if !value.is_empty() { opf.metadata.authors.push(value); }
                    } else if is_dc(child, "language") {
                        if opf.metadata.language.is_empty() { opf.metadata.language = value; }
                    } else if is_dc(child, "publisher") {
                        opf.metadata.publisher = value;
                    } else if is_dc(child, "description") {
                        opf.metadata.description = value;
                    } else if is_dc(child, "rights") {
                        opf.metadata.rights = value;
                    } else if is_dc(child, "identifier") {
                        opf.identifiers.push((child.attribute("id").map(str::to_string), value));
                    } else if child.tag_name().name() == "meta" {
                        match (child.attribute("property"), child.attribute("name")) {
                            (Some("dcterms:modified"), _) => opf.modified = Some(value),
                            (Some("belongs-to-collection"), _) => opf.metadata.series = value,
                            (_, Some("cover")) => opf.cover_meta = child.attribute("content").map(str::to_string),
                            (_, Some("calibre:series")) if opf.metadata.series.is_empty() => opf.metadata.series = child.attribute("content").unwrap_or("").to_string(),
                            _ => {}
                        }
                    }
                }
            }
            "manifest" => {
                for item in node.children().filter(|c| c.is_element() && c.tag_name().name() == "item") {
                    opf.manifest.push(ManifestItem {
                        id: item.attribute("id").unwrap_or("").to_string(),
                        href: item.attribute("href").unwrap_or("").to_string(),
                        media_type: item.attribute("media-type").unwrap_or("").to_string(),
                        properties: item.attribute("properties").unwrap_or("").split_whitespace().map(str::to_string).collect(),
                    });
                }
            }
            "spine" => {
                opf.spine_toc = node.attribute("toc").map(str::to_string);
                for itemref in node.children().filter(|c| c.is_element() && c.tag_name().name() == "itemref") {
                    opf.spine.push(SpineRef { idref: itemref.attribute("idref").unwrap_or("").to_string(), linear: itemref.attribute("linear") != Some("no") });
                }
            }
            _ => {}
        }
    }
    if let Some((_, value)) = opf.identifiers.iter().find(|(id, _)| id.as_deref() == Some(opf.unique_identifier.as_str())).or(opf.identifiers.first()) {
        opf.metadata.identifier = value.clone();
    }
    Ok(opf)
}

struct Edit {
    range: Range<usize>,
    replacement: String,
}

fn apply_edits(xml: &str, mut edits: Vec<Edit>) -> String {
    edits.sort_by(|a, b| b.range.start.cmp(&a.range.start));
    let mut out = xml.to_string();
    for edit in edits {
        out.replace_range(edit.range, &edit.replacement);
    }
    out
}

/// Aplica metadados informados pelo usuário ao OPF existente e atualiza
/// `dcterms:modified`. Devolve o novo XML e avisos sobre campos não aplicados.
pub fn apply_overrides(xml: &str, wanted: &Metadata, modified: &str) -> Result<(String, Vec<String>), String> {
    let doc = parse_xml(xml)?;
    let package = doc.root_element();
    let metadata = package.children().find(|c| c.is_element() && c.tag_name().name() == "metadata").ok_or("O OPF não tem <metadata>.")?;
    let version3 = package.attribute("version").map(|v| v.starts_with('3')).unwrap_or(false);
    let mut edits: Vec<Edit> = Vec::new();
    let mut inserts: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    // Prefixo usado pelo próprio arquivo (normalmente "dc:"), lido do texto da tag.
    let dc_prefix = metadata
        .children()
        .find(|c| is_dc(*c, "title"))
        .map(|node| {
            let tag = xml[node.range().start + 1..].split(|c: char| c.is_whitespace() || c == '>' || c == '/').next().unwrap_or("dc:title");
            tag.strip_suffix("title").unwrap_or("dc:").to_string()
        })
        .unwrap_or_else(|| "dc:".to_string());

    let mut set_single = |name: &str, value: &str| {
        if value.trim().is_empty() { return; }
        match metadata.children().find(|c| is_dc(*c, name)) {
            Some(node) => {
                if let Some(text) = node.children().find(Node::is_text) {
                    edits.push(Edit { range: text.range(), replacement: xml_escape(value.trim()) });
                } else {
                    edits.push(Edit { range: node.range(), replacement: format!("<{dc_prefix}{name}>{}</{dc_prefix}{name}>", xml_escape(value.trim())) });
                }
            }
            None => inserts.push(format!("    <{dc_prefix}{name}>{}</{dc_prefix}{name}>\n", xml_escape(value.trim()))),
        }
    };
    set_single("title", &wanted.title);
    set_single("language", &wanted.language);
    set_single("publisher", &wanted.publisher);
    set_single("description", &wanted.description);
    set_single("rights", &wanted.rights);

    let authors: Vec<&str> = wanted.authors.iter().map(|a| a.trim()).filter(|a| !a.is_empty()).collect();
    if !authors.is_empty() {
        let creators: Vec<Node> = metadata.children().filter(|c| is_dc(*c, "creator")).collect();
        if creators.is_empty() {
            for author in &authors { inserts.push(format!("    <{dc_prefix}creator>{}</{dc_prefix}creator>\n", xml_escape(author))); }
        } else if creators.len() == authors.len() {
            for (node, author) in creators.iter().zip(&authors) {
                if let Some(text) = node.children().find(Node::is_text) {
                    edits.push(Edit { range: text.range(), replacement: xml_escape(author) });
                }
            }
        } else {
            warnings.push(format!("O OPF tem {} autor(es) e você informou {}: os autores não foram alterados. Ajuste o número de autores no OPF ou mantenha a mesma quantidade.", creators.len(), authors.len()));
        }
    }
    if !wanted.series.trim().is_empty() {
        warnings.push("Série não foi aplicada: este EPUB já tem OPF próprio. Edite a coleção no OPF.".to_string());
    }

    if version3 {
        let existing = metadata.children().find(|c| c.is_element() && c.tag_name().name() == "meta" && c.attribute("property") == Some("dcterms:modified"));
        match existing.and_then(|node| node.children().find(Node::is_text)) {
            Some(text) => edits.push(Edit { range: text.range(), replacement: modified.to_string() }),
            None => inserts.push(format!("    <meta property=\"dcterms:modified\">{modified}</meta>\n")),
        }
    }

    if !inserts.is_empty() {
        let range = metadata.range();
        let close = xml[range.clone()].rfind("</").ok_or("Não foi possível localizar o fim de <metadata>.")? + range.start;
        // Inserção no início da linha do fechamento (ou logo antes dele).
        let line_start = xml[..close].rfind('\n').map(|i| i + 1).filter(|&i| xml[i..close].trim().is_empty()).unwrap_or(close);
        edits.push(Edit { range: line_start..line_start, replacement: inserts.concat() });
    }
    Ok((apply_edits(xml, edits), warnings))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="bookid">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:identifier id="bookid">urn:uuid:1234</dc:identifier>
    <dc:title>Velho</dc:title>
    <dc:creator>Ana</dc:creator>
    <dc:language>en</dc:language>
    <meta property="dcterms:modified">2020-01-01T00:00:00Z</meta>
  </metadata>
  <manifest><item id="a" href="a.xhtml" media-type="application/xhtml+xml"/></manifest>
  <spine><itemref idref="a"/></spine>
</package>"#;

    #[test]
    fn parses_metadata_manifest_and_spine() {
        let opf = parse_opf(SAMPLE).unwrap();
        assert_eq!(opf.metadata.title, "Velho");
        assert_eq!(opf.metadata.authors, vec!["Ana"]);
        assert_eq!(opf.metadata.identifier, "urn:uuid:1234");
        assert_eq!(opf.manifest.len(), 1);
        assert_eq!(opf.spine[0].idref, "a");
    }

    #[test]
    fn overrides_touch_only_changed_text() {
        let wanted = Metadata { title: "Novo & Melhor".into(), publisher: "Editora X".into(), ..Metadata::default() };
        let (xml, warnings) = apply_overrides(SAMPLE, &wanted, "2026-10-12T10:00:00Z").unwrap();
        assert!(warnings.is_empty());
        assert!(xml.contains("<dc:title>Novo &amp; Melhor</dc:title>"));
        assert!(xml.contains("<dc:publisher>Editora X</dc:publisher>"));
        assert!(xml.contains("2026-10-12T10:00:00Z"));
        assert!(xml.contains("<dc:creator>Ana</dc:creator>"), "campos não informados permanecem");
        let reparsed = parse_opf(&xml).unwrap();
        assert_eq!(reparsed.metadata.title, "Novo & Melhor");
        assert_eq!(reparsed.metadata.publisher, "Editora X");
    }

    #[test]
    fn author_count_mismatch_is_reported_not_silently_applied() {
        let wanted = Metadata { authors: vec!["A".into(), "B".into()], ..Metadata::default() };
        let (xml, warnings) = apply_overrides(SAMPLE, &wanted, "2026-10-12T10:00:00Z").unwrap();
        assert_eq!(warnings.len(), 1);
        assert!(xml.contains("<dc:creator>Ana</dc:creator>"));
    }
}
