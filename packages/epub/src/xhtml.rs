//! Leitura leve de capítulos XHTML: título para o sumário e características
//! que afetam o manifest.

use crate::opf::parse_xml;
use crate::util::file_stem;
use roxmltree::Node;

#[derive(Debug, Default)]
pub struct ChapterInfo {
    pub title: String,
    pub has_stylesheet: bool,
    pub has_svg: bool,
    pub parse_error: Option<String>,
}

fn clean(text: &str) -> String {
    let collapsed = text.replace('\u{a0}', " ").split_whitespace().collect::<Vec<_>>().join(" ");
    collapsed.chars().take(120).collect()
}

fn text_of(node: Node) -> String {
    clean(&node.descendants().filter(|n| n.is_text()).map(|n| n.text().unwrap_or("")).collect::<String>())
}

fn has_class(node: Node, class: &str) -> bool {
    node.attribute("class").map(|value| value.split_whitespace().any(|c| c == class)).unwrap_or(false)
}

pub fn analyze(path: &str, text: &str) -> ChapterInfo {
    let doc = match parse_xml(text) {
        Ok(doc) => doc,
        Err(error) => {
            return ChapterInfo { title: file_stem(path), parse_error: Some(error), ..ChapterInfo::default() };
        }
    };
    let mut info = ChapterInfo::default();
    let mut heading: Option<String> = None;
    let mut number: Option<String> = None;
    let mut subtitle: Option<String> = None;
    let mut doc_title: Option<String> = None;
    for node in doc.descendants().filter(Node::is_element) {
        let name = node.tag_name().name();
        match name {
            "h1" | "h2" | "h3" if heading.is_none() => {
                let value = text_of(node);
                if !value.is_empty() { heading = Some(value); }
            }
            "title" if doc_title.is_none() => {
                let value = text_of(node);
                if !value.is_empty() { doc_title = Some(value); }
            }
            "link" if node.attribute("rel").map(|r| r.to_ascii_lowercase().contains("stylesheet")).unwrap_or(false) => info.has_stylesheet = true,
            "svg" => info.has_svg = true,
            _ => {}
        }
        if number.is_none() && has_class(node, "chapter-number") {
            let value = text_of(node);
            if !value.is_empty() { number = Some(value); }
        }
        if subtitle.is_none() && has_class(node, "chapter-title") {
            let value = text_of(node);
            if !value.is_empty() { subtitle = Some(value); }
        }
    }
    info.title = match (heading, number, subtitle) {
        (Some(heading), _, _) => heading,
        (None, Some(number), Some(subtitle)) => format!("{number} — {subtitle}"),
        (None, Some(only), None) | (None, None, Some(only)) => only,
        (None, None, None) => doc_title.unwrap_or_else(|| file_stem(path)),
    };
    info
}

/// Insere `<link rel="stylesheet">` antes de `</head>`. Devolve `None` se não houver `</head>`.
pub fn inject_stylesheet(text: &str, href: &str) -> Option<String> {
    let lower = text.to_ascii_lowercase();
    let at = lower.rfind("</head>")?;
    let link = format!("<link rel=\"stylesheet\" type=\"text/css\" href=\"{}\" />\n", crate::util::xml_escape(href));
    let mut out = String::with_capacity(text.len() + link.len());
    out.push_str(&text[..at]);
    out.push_str(&link);
    out.push_str(&text[at..]);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = "<?xml version=\"1.0\"?><!DOCTYPE html><html xmlns=\"http://www.w3.org/1999/xhtml\"><head><title>T</title></head><body>";

    #[test]
    fn title_prefers_heading_then_skill_classes_then_title() {
        let a = analyze("a.xhtml", &format!("{DOC}<h2>Primeiro</h2><p>x</p></body></html>"));
        assert_eq!(a.title, "Primeiro");
        let b = analyze("b.xhtml", &format!("{DOC}<div class=\"chapter-break\"><p class=\"chapter-number\">CAPÍTULO 3</p><p class=\"chapter-title\">A queda</p></div></body></html>"));
        assert_eq!(b.title, "CAPÍTULO 3 — A queda");
        let c = analyze("c.xhtml", &format!("{DOC}<p>sem título</p></body></html>"));
        assert_eq!(c.title, "T");
    }

    #[test]
    fn detects_stylesheet_svg_and_broken_markup() {
        let a = analyze("a.xhtml", "<html xmlns=\"http://www.w3.org/1999/xhtml\"><head><link rel=\"stylesheet\" href=\"s.css\"/></head><body><svg/></body></html>");
        assert!(a.has_stylesheet && a.has_svg);
        let broken = analyze("b.xhtml", "<html><body><p>a&nbsp;b</p></body></html>");
        assert!(broken.parse_error.is_some());
        assert_eq!(broken.title, "b");
    }

    #[test]
    fn stylesheet_injection_is_placed_in_head() {
        let out = inject_stylesheet("<html><head><title>x</title></head><body/></html>", "../css/s.css").unwrap();
        assert!(out.contains("href=\"../css/s.css\" />\n</head>"));
        assert!(inject_stylesheet("<p>sem head</p>", "s.css").is_none());
    }
}
