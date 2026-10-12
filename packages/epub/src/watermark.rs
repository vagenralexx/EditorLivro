//! Marca d'água por exemplar para o PRÓPRIO livro do autor ("social DRM").
//!
//! NÃO é DRM: não impede cópia. Identifica a origem de um exemplar vazado:
//! 1. aviso visível (página de licença com o nome do destinatário e um código);
//! 2. marca forense: o código é gravado em caracteres de largura zero ao fim de
//!    parágrafos, repetido ao longo do livro, e pode ser lido depois.
//! Não remove proteção de terceiros.

use crate::model::{Package, Report};
use crate::opf::{parse_opf, parse_xml};
use crate::util::*;
use serde::{Deserialize, Serialize};

const START: char = '\u{2060}'; // WORD JOINER: início da marca
const ZERO: char = '\u{200B}'; // ZERO WIDTH SPACE: bit 0
const ONE: char = '\u{200C}'; // ZERO WIDTH NON-JOINER: bit 1
const TOKEN_BITS: usize = 32;
/// A marca aparece ao fim de 1 em cada N parágrafos.
const EVERY: usize = 4;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct WatermarkOptions {
    /// Para quem é este exemplar (nome, e-mail ou número do pedido).
    pub recipient: String,
    /// Inclui a página de licença com nome e código.
    pub visible_notice: bool,
    /// Grava o código em caracteres invisíveis nos capítulos.
    pub forensic: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WatermarkInfo { pub token: String, pub recipient: String, pub forensic_markers: usize }

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Detection {
    pub token: String,
    /// Quantas marcas foram lidas e quantas concordam com o código escolhido.
    pub markers: usize,
    pub agreeing: usize,
}

pub fn new_token() -> String {
    let bytes = uuid::Uuid::new_v4().into_bytes();
    format!("{:02X}{:02X}{:02X}{:02X}", bytes[0], bytes[1], bytes[2], bytes[3])
}

fn encode(token: &str) -> Option<String> {
    let value = u32::from_str_radix(token, 16).ok()?;
    if token.len() != 8 { return None; }
    let mut out = String::with_capacity(TOKEN_BITS + 1);
    out.push(START);
    for bit in (0..TOKEN_BITS).rev() { out.push(if (value >> bit) & 1 == 1 { ONE } else { ZERO }); }
    Some(out)
}

/// Insere a marca antes de `</p>` em 1 de cada `EVERY` parágrafos. Devolve o texto e o número de marcas.
pub fn embed(xhtml: &str, token: &str) -> (String, usize) {
    let Some(mark) = encode(token) else { return (xhtml.to_string(), 0) };
    let lower = xhtml.to_ascii_lowercase();
    let mut out = String::with_capacity(xhtml.len() + 200);
    let (mut last, mut count, mut seen) = (0, 0, 0);
    for (at, _) in lower.match_indices("</p>") {
        if seen % EVERY == 0 && !xhtml[..at].trim_end().is_empty() {
            out.push_str(&xhtml[last..at]);
            out.push_str(&mark);
            last = at;
            count += 1;
        }
        seen += 1;
    }
    out.push_str(&xhtml[last..]);
    (out, count)
}

/// Lê todas as marcas do texto e devolve o código mais frequente.
pub fn extract(text: &str) -> Option<Detection> {
    let chars: Vec<char> = text.chars().collect();
    let mut votes: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut markers = 0;
    let mut i = 0;
    while i + TOKEN_BITS < chars.len() {
        if chars[i] == START && chars[i + 1..=i + TOKEN_BITS].iter().all(|c| *c == ZERO || *c == ONE) {
            let value = chars[i + 1..=i + TOKEN_BITS].iter().fold(0u32, |acc, c| (acc << 1) | u32::from(*c == ONE));
            *votes.entry(format!("{value:08X}")).or_default() += 1;
            markers += 1;
            i += TOKEN_BITS + 1;
        } else { i += 1; }
    }
    votes.into_iter().max_by_key(|(_, count)| *count).map(|(token, agreeing)| Detection { token, markers, agreeing })
}

/// Detecta a marca num .epub (qualquer capítulo XHTML).
pub fn detect_in_epub(path: &str) -> Result<Option<Detection>, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("Não foi possível abrir o EPUB: {e}"))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("Arquivo EPUB inválido: {e}"))?;
    let mut all = String::new();
    for index in 0..archive.len().min(5000) {
        let Ok(entry) = archive.by_index(index) else { continue };
        let name = entry.name().to_ascii_lowercase();
        if !(name.ends_with(".xhtml") || name.ends_with(".html") || name.ends_with(".htm")) || entry.size() > 32 * 1024 * 1024 { continue; }
        use std::io::Read;
        let mut text = String::new();
        if entry.take(32 * 1024 * 1024).read_to_string(&mut text).is_ok() { all.push_str(&text); }
    }
    Ok(extract(&all))
}

fn license_page(language: &str, recipient: &str, token: &str, title: &str) -> String {
    let (heading, line1, line2, code) = if language.to_ascii_lowercase().starts_with("pt") {
        ("Exemplar licenciado", "Este exemplar foi licenciado para uso pessoal de", "É proibida a cópia ou redistribuição sem autorização do autor.", "Código do exemplar")
    } else {
        ("Licensed copy", "This copy is licensed for the personal use of", "Copying or redistribution without the author's permission is prohibited.", "Copy code")
    };
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE html>\n<html xmlns=\"http://www.w3.org/1999/xhtml\" xml:lang=\"{lang}\" lang=\"{lang}\">\n<head>\n<meta charset=\"UTF-8\" />\n<title>{heading}</title>\n<style type=\"text/css\">body {{ text-align: center; margin: 3em 1em; }} p {{ margin: .8em 0; text-indent: 0; }} .small {{ font-size: .8em; color: #666; }}</style>\n</head>\n<body>\n<h2>{heading}</h2>\n<p>{title}</p>\n<p>{line1}</p>\n<p><strong>{recipient}</strong></p>\n<p class=\"small\">{line2}</p>\n<p class=\"small\">{code}: {token}</p>\n</body>\n</html>\n",
        lang = xml_escape(language), heading = heading, title = xml_escape(title), line1 = line1, recipient = xml_escape(recipient), line2 = line2, code = code, token = token
    )
}

/// Aplica a marca d'água ao pacote já montado (modo `Loose` ou `Extracted`).
pub fn apply(package: &mut Package, options: &WatermarkOptions, report: &mut Report) -> Option<WatermarkInfo> {
    let recipient = options.recipient.trim();
    if !options.visible_notice && !options.forensic { return None; }
    if recipient.is_empty() { report.error("WATERMARK-RECIPIENT", None, "Informe para quem é este exemplar (nome ou e-mail) para aplicar a marca d'água."); return None; }
    let container = package.get("META-INF/container.xml").map(|f| String::from_utf8_lossy(&f.data).into_owned())?;
    let opf_path = parse_xml(&container).ok()?.descendants().find(|n| n.is_element() && n.tag_name().name() == "rootfile")?.attribute("full-path")?.to_string();
    let opf_xml = String::from_utf8_lossy(&package.get(&opf_path)?.data).into_owned();
    let opf = parse_opf(&opf_xml).ok()?;
    let token = new_token();
    let mut markers = 0;

    if options.forensic {
        for item in opf.manifest.iter().filter(|i| i.media_type == "application/xhtml+xml" && !i.properties.iter().any(|p| p == "nav")) {
            let Some(path) = resolve_href(&opf_path, &percent_decode(&item.href)) else { continue };
            let Some(file) = package.get(&path) else { continue };
            let text = String::from_utf8_lossy(&file.data).into_owned();
            // Páginas de capa e licença não têm parágrafos de leitura.
            if !text.contains("</p>") { continue; }
            let (marked, count) = embed(&text, &token);
            if count > 0 { markers += count; package.set(&path, marked.into_bytes()); }
        }
        if markers == 0 { report.warn("WATERMARK-NO-TEXT", None, "Não há parágrafos suficientes para gravar a marca forense."); }
    }

    if options.visible_notice {
        let dir = dir_of(&opf_path).to_string();
        let name = (0..).map(|n| if n == 0 { "licenca.xhtml".to_string() } else { format!("licenca-{n}.xhtml") }).find(|candidate| package.get(&if dir.is_empty() { candidate.clone() } else { format!("{dir}/{candidate}") }).is_none()).expect("nome livre");
        let full = if dir.is_empty() { name.clone() } else { format!("{dir}/{name}") };
        package.set(&full, license_page(if opf.metadata.language.is_empty() { "pt-BR" } else { &opf.metadata.language }, recipient, &token, &opf.metadata.title).into_bytes());
        let id = (0..).map(|n| if n == 0 { "licenca".to_string() } else { format!("licenca{n}") }).find(|c| !opf.manifest.iter().any(|i| &i.id == c)).expect("id livre");
        let after_cover = opf.spine.first().and_then(|first| opf.manifest.iter().find(|i| i.id == first.idref)).map(|item| { let probe = format!("{} {}", item.id, item.href).to_lowercase(); probe.contains("cover") || probe.contains("capa") }).unwrap_or(false);
        if let Some(updated) = insert_license_item(&opf_xml, &id, &name, after_cover) { package.set(&opf_path, updated.into_bytes()); }
        else { report.warn("WATERMARK-OPF", Some(&opf_path), "Não foi possível acrescentar a página de licença ao OPF; só a marca forense foi aplicada."); }
    }
    Some(WatermarkInfo { token, recipient: recipient.to_string(), forensic_markers: markers })
}

/// Acrescenta o item ao manifest e o itemref na spine: depois da capa (se o primeiro
/// item for a capa) ou antes de tudo, para a licença abrir o livro.
fn insert_license_item(opf_xml: &str, id: &str, href: &str, after_cover: bool) -> Option<String> {
    let doc = parse_xml(opf_xml).ok()?;
    let root = doc.root_element();
    let manifest = root.children().find(|n| n.is_element() && n.tag_name().name() == "manifest")?;
    let spine = root.children().find(|n| n.is_element() && n.tag_name().name() == "spine")?;
    let manifest_close = manifest.range().start + opf_xml[manifest.range()].rfind("</")?;
    let item = format!("<item id=\"{id}\" href=\"{}\" media-type=\"application/xhtml+xml\"/>\n  ", percent_encode_path(href));
    let first_ref = spine.children().find(|n| n.is_element() && n.tag_name().name() == "itemref");
    let (ref_pos, itemref) = match first_ref {
        Some(first) if after_cover => (first.range().end, format!("\n    <itemref idref=\"{id}\"/>")),
        Some(first) => (first.range().start, format!("<itemref idref=\"{id}\"/>\n    ")),
        None => (spine.range().start + opf_xml[spine.range()].rfind("</")?, format!("<itemref idref=\"{id}\"/>\n  ")),
    };
    let (first, second) = if manifest_close < ref_pos { ((manifest_close, item), (ref_pos, itemref)) } else { ((ref_pos, itemref), (manifest_close, item)) };
    let mut out = opf_xml.to_string();
    out.insert_str(second.0, &second.1);
    out.insert_str(first.0, &first.1);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embeds_and_extracts_the_same_token() {
        let html = "<html><body><p>um</p><p>dois</p><p>tres</p><p>quatro</p><p>cinco</p><p>seis</p><p>sete</p><p>oito</p><p>nove</p></body></html>";
        let (marked, count) = embed(html, "A1B2C3D4");
        assert_eq!(count, 3, "parágrafos 1, 5 e 9");
        let found = extract(&marked).unwrap();
        assert_eq!((found.token.as_str(), found.markers, found.agreeing), ("A1B2C3D4", 3, 3));
        assert!(marked.replace([START, ZERO, ONE], "") == html, "removendo os invisíveis sobra o original");
    }

    #[test]
    fn survives_partial_copies_and_majority_votes() {
        let (marked, _) = embed(&"<p>texto de um parágrafo</p>".repeat(12), "00FF00FF");
        let partial: String = marked.chars().skip(marked.chars().count() / 2).collect();
        assert_eq!(extract(&partial).unwrap().token, "00FF00FF");
        let corrupted = marked.replacen(ONE, "x", 1);
        let found = extract(&corrupted).unwrap();
        assert_eq!(found.token, "00FF00FF", "uma marca danificada não muda o voto");
        assert!(found.agreeing < found.markers + 1);
    }

    #[test]
    fn plain_text_has_no_mark_and_bad_tokens_are_ignored() {
        assert!(extract("Texto normal sem marca").is_none());
        assert_eq!(embed("<p>a</p>", "XYZ").1, 0);
        assert_eq!(new_token().len(), 8);
        assert_ne!(new_token(), new_token());
    }

    #[test]
    fn license_item_is_inserted_after_the_first_spine_item() {
        let opf = "<package xmlns=\"http://www.idpf.org/2007/opf\" version=\"3.0\"><manifest>\n  <item id=\"a\" href=\"a.xhtml\" media-type=\"application/xhtml+xml\"/>\n</manifest><spine><itemref idref=\"a\"/><itemref idref=\"b\"/></spine></package>";
        let spine_of = |xml: &str| parse_opf(xml).unwrap().spine.iter().map(|s| s.idref.clone()).collect::<Vec<_>>();
        let after = insert_license_item(opf, "licenca", "licenca.xhtml", true).unwrap();
        assert_eq!(parse_opf(&after).unwrap().manifest.iter().map(|i| i.id.as_str()).collect::<Vec<_>>(), vec!["a", "licenca"]);
        assert_eq!(spine_of(&after), vec!["a", "licenca", "b"]);
        assert_eq!(spine_of(&insert_license_item(opf, "licenca", "licenca.xhtml", false).unwrap()), vec!["licenca", "a", "b"]);
    }
}
