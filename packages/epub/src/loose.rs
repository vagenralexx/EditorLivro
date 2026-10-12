//! Geração de um pacote EPUB 3 completo a partir de uma pasta com capítulos
//! XHTML/HTML, CSS e imagens (sem OPF). Estrutura e CSS seguem a especificação
//! `producao.md` da skill editor-profissional-de-livros.

use crate::model::{Metadata, Package, Report};
use crate::scan::ScannedFile;
use crate::util::*;
use crate::xhtml;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

const DEFAULT_CSS: &str = include_str!("../assets/stylesheet.css");
const OEBPS: &str = "OEBPS";

pub struct LooseInput<'a> {
    pub root: &'a Path,
    pub files: &'a [ScannedFile],
    pub metadata: &'a Metadata,
    pub chapters: &'a [String],
    pub cover: Option<&'a Path>,
}

struct Chapter {
    rel: String,
    id: String,
    title: String,
    svg: bool,
    text: String,
}

fn is_html(path: &str) -> bool {
    matches!(extension(path).as_str(), "xhtml" | "html" | "htm")
}

fn unique_name(taken: &mut HashSet<String>, wanted: &str) -> String {
    let mut candidate = wanted.to_string();
    let (stem, ext) = match wanted.rfind('.') {
        Some(dot) => (wanted[..dot].to_string(), wanted[dot..].to_string()),
        None => (wanted.to_string(), String::new()),
    };
    let mut counter = 2;
    while taken.contains(&candidate.to_ascii_lowercase()) {
        candidate = format!("{stem}-{counter}{ext}");
        counter += 1;
    }
    taken.insert(candidate.to_ascii_lowercase());
    candidate
}

/// Converte um caminho informado pelo app (absoluto ou relativo, com `\` ou `/`)
/// em caminho relativo à raiz. A comparação do prefixo ignora maiúsculas
/// (no Windows, `d:\\livro` e `D:\\livro` são a mesma pasta).
fn rel_of(root: &Path, requested: &str) -> String {
    let normalized = requested.replace('\\', "/");
    let root_text = root.to_string_lossy().replace('\\', "/");
    let root_text = root_text.trim_end_matches('/');
    match normalized.get(..root_text.len()) {
        Some(prefix) if !root_text.is_empty() && prefix.eq_ignore_ascii_case(root_text) && normalized[root_text.len()..].starts_with('/') => normalized[root_text.len() + 1..].to_string(),
        _ => normalized,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rel_of_strips_root_ignoring_case_and_separators() {
        let root = Path::new("D:\\Smart Print\\Livro");
        assert_eq!(rel_of(root, "d:\\smart print\\livro\\text\\a.xhtml"), "text/a.xhtml");
        assert_eq!(rel_of(root, "D:/Smart Print/Livro/b.xhtml"), "b.xhtml");
        assert_eq!(rel_of(root, "text/c.xhtml"), "text/c.xhtml");
        assert_eq!(rel_of(root, "D:/Smart Print/Livro2/x.xhtml"), "D:/Smart Print/Livro2/x.xhtml");
    }
}

pub fn build_loose(input: LooseInput, report: &mut Report) -> Result<Package, String> {
    let LooseInput { root, files, metadata, chapters, cover } = input;

    // 1) Capítulos, na ordem pedida (ou ordem natural dos nomes).
    let all_html: Vec<&ScannedFile> = files.iter().filter(|file| is_html(&file.rel)).collect();
    let mut chapter_paths: Vec<String> = Vec::new();
    for requested in chapters {
        let rel = rel_of(root, requested);
        if all_html.iter().any(|file| file.rel == rel) && !chapter_paths.contains(&rel) {
            chapter_paths.push(rel);
        } else {
            report.warn("CHAPTER-MISSING", Some(&rel), "Capítulo informado não foi encontrado na pasta e foi ignorado.");
        }
    }
    if chapter_paths.is_empty() {
        chapter_paths = all_html.iter().map(|file| file.rel.clone()).collect();
    }
    if chapter_paths.is_empty() {
        return Err("Nenhum capítulo .xhtml ou .html foi encontrado na pasta do projeto.".to_string());
    }
    for file in &all_html {
        if !chapter_paths.contains(&file.rel) {
            report.info("CHAPTER-SKIPPED", Some(&file.rel), "Arquivo HTML fora da lista de capítulos; não foi incluído no livro.");
        }
    }

    // 2) Metadados obrigatórios.
    let title = metadata.title.trim();
    if title.is_empty() {
        report.error("META-TITLE", None, "Informe o título do livro.");
    }
    let authors: Vec<&str> = metadata.authors.iter().map(|a| a.trim()).filter(|a| !a.is_empty()).collect();
    if authors.is_empty() {
        report.warn("META-AUTHOR", None, "Nenhum autor informado; lojas como a KDP exigem o autor.");
    }
    let language = if metadata.language.trim().is_empty() { "pt-BR" } else { metadata.language.trim() };
    let identifier = if metadata.identifier.trim().is_empty() { format!("urn:uuid:{}", uuid::Uuid::new_v4()) } else { metadata.identifier.trim().to_string() };

    // 3) Ler e analisar capítulos.
    let mut taken: HashSet<String> = files.iter().map(|file| file.rel.to_ascii_lowercase()).collect();
    let css_files: Vec<&ScannedFile> = files.iter().filter(|file| extension(&file.rel) == "css").collect();
    let generated_css = if css_files.is_empty() { Some(unique_name(&mut taken, "css/stylesheet.css")) } else { None };
    let stylesheet_path = generated_css.clone().or_else(|| css_files.first().map(|file| file.rel.clone())).unwrap_or_default();

    let mut parsed: Vec<Chapter> = Vec::new();
    for (index, rel) in chapter_paths.iter().enumerate() {
        let file = all_html.iter().find(|file| &file.rel == rel).expect("capítulo validado acima");
        let bytes = fs::read(&file.abs).map_err(|error| format!("Não foi possível ler {rel}: {error}"))?;
        let text = match String::from_utf8(bytes) {
            Ok(text) => text,
            Err(_) => {
                report.error("XHTML-ENCODING", Some(rel), "O arquivo não está em UTF-8.");
                continue;
            }
        };
        let (mut text, fixed) = fix_html_entities(&text);
        if fixed > 0 {
            report.info("XHTML-ENTITIES", Some(rel), format!("{fixed} entidade(s) HTML (como &nbsp;) convertida(s) para o equivalente numérico exigido pelo XHTML."));
        }
        let mut info = xhtml::analyze(rel, &text);
        // Erros de parse são reportados pela validação do pacote (uma única vez).
        if info.parse_error.is_none() && !info.has_stylesheet && !stylesheet_path.is_empty() {
            let href = percent_encode_path(&relative_href(rel, &stylesheet_path));
            if let Some(updated) = xhtml::inject_stylesheet(&text, &href) {
                text = updated;
                info.has_stylesheet = true;
            }
        }
        parsed.push(Chapter { rel: rel.clone(), id: format!("ch{:03}", index + 1), title: info.title, svg: info.has_svg, text });
    }

    // 4) Recursos (CSS, imagens, fontes). A capa escolhida é copiada à parte.
    let cover_abs = cover.and_then(|path| fs::canonicalize(path).ok());
    let mut resources: Vec<(&ScannedFile, &'static str)> = Vec::new();
    for file in files {
        if is_html(&file.rel) { continue; }
        if cover_abs.is_some() && fs::canonicalize(&file.abs).ok() == cover_abs { continue; }
        match media_type_for(&file.rel) {
            Some("text/javascript") => report.warn("RESOURCE-SCRIPT", Some(&file.rel), "Scripts não são incluídos: muitos leitores e lojas rejeitam EPUB com JavaScript."),
            Some("application/x-dtbncx+xml") => {}
            Some(media) => resources.push((file, media)),
            None => report.info("RESOURCE-SKIPPED", Some(&file.rel), "Tipo de arquivo não reconhecido; não foi incluído no livro."),
        }
    }

    // 5) Capa opcional.
    let mut cover_image: Option<(String, &'static str, Vec<u8>)> = None;
    if let Some(path) = cover {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
        match (media_type_for(&format!("x.{ext}")), fs::read(path)) {
            (Some(media @ ("image/png" | "image/jpeg" | "image/gif" | "image/webp")), Ok(bytes)) => {
                let name = unique_name(&mut taken, &format!("images/cover.{ext}"));
                cover_image = Some((name, media, bytes));
            }
            (_, Err(error)) => report.error("COVER-READ", None, format!("Não foi possível ler a capa: {error}")),
            _ => report.error("COVER-FORMAT", None, "A capa deve ser PNG, JPEG, GIF ou WebP."),
        }
    } else {
        report.warn("COVER-MISSING", None, "Sem capa: a maioria das lojas exige uma imagem de capa.");
    }

    // 6) Documentos gerados.
    let nav_path = unique_name(&mut taken, "nav.xhtml");
    let ncx_path = unique_name(&mut taken, "toc.ncx");
    let opf_path = unique_name(&mut taken, "content.opf");
    let cover_page = cover_image.as_ref().map(|_| unique_name(&mut taken, "cover.xhtml"));

    let mut manifest = String::new();
    let mut spine = String::new();
    manifest.push_str(&format!("    <item id=\"nav\" href=\"{}\" media-type=\"application/xhtml+xml\" properties=\"nav\"/>\n", percent_encode_path(&nav_path)));
    manifest.push_str(&format!("    <item id=\"ncx\" href=\"{}\" media-type=\"application/x-dtbncx+xml\"/>\n", percent_encode_path(&ncx_path)));
    if let (Some((image, media, _)), Some(page)) = (&cover_image, &cover_page) {
        manifest.push_str(&format!("    <item id=\"cover-image\" href=\"{}\" media-type=\"{media}\" properties=\"cover-image\"/>\n", percent_encode_path(image)));
        manifest.push_str(&format!("    <item id=\"cover\" href=\"{}\" media-type=\"application/xhtml+xml\"/>\n", percent_encode_path(page)));
        spine.push_str("    <itemref idref=\"cover\"/>\n");
    }
    if let Some(css) = &generated_css {
        manifest.push_str(&format!("    <item id=\"css-base\" href=\"{}\" media-type=\"text/css\"/>\n", percent_encode_path(css)));
    }
    for chapter in &parsed {
        let props = if chapter.svg { " properties=\"svg\"" } else { "" };
        manifest.push_str(&format!("    <item id=\"{}\" href=\"{}\" media-type=\"application/xhtml+xml\"{props}/>\n", chapter.id, percent_encode_path(&chapter.rel)));
        spine.push_str(&format!("    <itemref idref=\"{}\"/>\n", chapter.id));
    }
    for (index, (file, media)) in resources.iter().enumerate() {
        manifest.push_str(&format!("    <item id=\"res{:03}\" href=\"{}\" media-type=\"{media}\"/>\n", index + 1, percent_encode_path(&file.rel)));
    }

    let modified = utc_timestamp();
    let mut meta = String::new();
    meta.push_str(&format!("    <dc:identifier id=\"bookid\">{}</dc:identifier>\n", xml_escape(&identifier)));
    meta.push_str(&format!("    <dc:title>{}</dc:title>\n", xml_escape(if title.is_empty() { "Sem título" } else { title })));
    for (index, author) in authors.iter().enumerate() {
        meta.push_str(&format!("    <dc:creator id=\"creator{}\">{}</dc:creator>\n", index + 1, xml_escape(author)));
    }
    meta.push_str(&format!("    <dc:language>{}</dc:language>\n", xml_escape(language)));
    if !metadata.publisher.trim().is_empty() { meta.push_str(&format!("    <dc:publisher>{}</dc:publisher>\n", xml_escape(metadata.publisher.trim()))); }
    if !metadata.description.trim().is_empty() { meta.push_str(&format!("    <dc:description>{}</dc:description>\n", xml_escape(metadata.description.trim()))); }
    if !metadata.rights.trim().is_empty() { meta.push_str(&format!("    <dc:rights>{}</dc:rights>\n", xml_escape(metadata.rights.trim()))); }
    if !metadata.series.trim().is_empty() {
        meta.push_str(&format!("    <meta property=\"belongs-to-collection\" id=\"series\">{}</meta>\n    <meta refines=\"#series\" property=\"collection-type\">series</meta>\n", xml_escape(metadata.series.trim())));
        meta.push_str(&format!("    <meta name=\"calibre:series\" content=\"{}\"/>\n", xml_escape(metadata.series.trim())));
        if !metadata.series_index.trim().is_empty() {
            meta.push_str(&format!("    <meta refines=\"#series\" property=\"group-position\">{}</meta>\n    <meta name=\"calibre:series_index\" content=\"{}\"/>\n", xml_escape(metadata.series_index.trim()), xml_escape(metadata.series_index.trim())));
        }
    }
    meta.push_str(&format!("    <meta property=\"dcterms:modified\">{modified}</meta>\n"));
    if cover_image.is_some() { meta.push_str("    <meta name=\"cover\" content=\"cover-image\"/>\n"); }

    let opf = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<package xmlns=\"http://www.idpf.org/2007/opf\" version=\"3.0\" unique-identifier=\"bookid\" xml:lang=\"{lang}\">\n  <metadata xmlns:dc=\"http://purl.org/dc/elements/1.1/\">\n{meta}  </metadata>\n  <manifest>\n{manifest}  </manifest>\n  <spine toc=\"ncx\">\n{spine}  </spine>\n</package>\n",
        lang = xml_escape(language)
    );

    // Sumário EPUB 3 (nav) e EPUB 2 (NCX, usado pelo Kindle).
    let href_from = |from: &str, target: &str| percent_encode_path(&relative_href(from, target));
    let mut nav_items = String::new();
    let mut ncx_points = String::new();
    for (index, chapter) in parsed.iter().enumerate() {
        nav_items.push_str(&format!("      <li><a href=\"{}\">{}</a></li>\n", href_from(&nav_path, &chapter.rel), xml_escape(&chapter.title)));
        ncx_points.push_str(&format!(
            "    <navPoint id=\"np{n}\" playOrder=\"{n}\"><navLabel><text>{}</text></navLabel><content src=\"{}\"/></navPoint>\n",
            xml_escape(&chapter.title), href_from(&ncx_path, &chapter.rel), n = index + 1
        ));
    }
    let mut landmarks = String::new();
    if let Some(page) = &cover_page { landmarks.push_str(&format!("      <li><a epub:type=\"cover\" href=\"{}\">Capa</a></li>\n", href_from(&nav_path, page))); }
    if let Some(first) = parsed.first() { landmarks.push_str(&format!("      <li><a epub:type=\"bodymatter\" href=\"{}\">Início do livro</a></li>\n", href_from(&nav_path, &first.rel))); }
    let nav = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE html>\n<html xmlns=\"http://www.w3.org/1999/xhtml\" xmlns:epub=\"http://www.idpf.org/2007/ops\" xml:lang=\"{lang}\" lang=\"{lang}\">\n<head>\n<meta charset=\"UTF-8\" />\n<title>Sumário</title>\n{css}</head>\n<body>\n  <nav epub:type=\"toc\" id=\"toc\">\n    <h1>Sumário</h1>\n    <ol>\n{nav_items}    </ol>\n  </nav>\n  <nav epub:type=\"landmarks\" id=\"landmarks\" hidden=\"hidden\">\n    <ol>\n{landmarks}    </ol>\n  </nav>\n</body>\n</html>\n",
        lang = xml_escape(language),
        css = if stylesheet_path.is_empty() { String::new() } else { format!("<link rel=\"stylesheet\" type=\"text/css\" href=\"{}\" />\n", href_from(&nav_path, &stylesheet_path)) },
    );
    let ncx = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<ncx xmlns=\"http://www.daisy.org/z3986/2005/ncx/\" version=\"2005-1\" xml:lang=\"{lang}\">\n  <head>\n    <meta name=\"dtb:uid\" content=\"{id}\"/>\n    <meta name=\"dtb:depth\" content=\"1\"/>\n    <meta name=\"dtb:totalPageCount\" content=\"0\"/>\n    <meta name=\"dtb:maxPageNumber\" content=\"0\"/>\n  </head>\n  <docTitle><text>{title}</text></docTitle>\n  <navMap>\n{ncx_points}  </navMap>\n</ncx>\n",
        lang = xml_escape(language), id = xml_escape(&identifier), title = xml_escape(if title.is_empty() { "Sem título" } else { title })
    );

    // 7) Montagem do pacote.
    let mut package = Package { identifier: identifier.clone(), ..Package::default() };
    package.set("META-INF/container.xml", container_xml(&opf_path).into_bytes());
    package.set(&format!("{OEBPS}/{opf_path}"), opf.into_bytes());
    package.set(&format!("{OEBPS}/{nav_path}"), nav.into_bytes());
    package.set(&format!("{OEBPS}/{ncx_path}"), ncx.into_bytes());
    if let Some(css) = generated_css { package.set(&format!("{OEBPS}/{css}"), DEFAULT_CSS.as_bytes().to_vec()); }
    if let (Some((image, _, bytes)), Some(page)) = (cover_image, cover_page) {
        let page_xml = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE html>\n<html xmlns=\"http://www.w3.org/1999/xhtml\" xmlns:epub=\"http://www.idpf.org/2007/ops\" xml:lang=\"{lang}\" lang=\"{lang}\">\n<head>\n<meta charset=\"UTF-8\" />\n<title>Capa</title>\n{css}</head>\n<body>\n  <section epub:type=\"cover\" class=\"cover\"><img src=\"{src}\" alt=\"Capa de {title}\" /></section>\n</body>\n</html>\n",
            lang = xml_escape(language),
            css = if stylesheet_path.is_empty() { String::new() } else { format!("<link rel=\"stylesheet\" type=\"text/css\" href=\"{}\" />\n", href_from(&page, &stylesheet_path)) },
            src = href_from(&page, &image),
            title = xml_escape(title)
        );
        package.set(&format!("{OEBPS}/{page}"), page_xml.into_bytes());
        package.set(&format!("{OEBPS}/{image}"), bytes);
    }
    for chapter in parsed {
        package.set(&format!("{OEBPS}/{}", chapter.rel), chapter.text.into_bytes());
    }
    for (file, _) in resources {
        let bytes = fs::read(&file.abs).map_err(|error| format!("Não foi possível ler {}: {error}", file.rel))?;
        package.set(&format!("{OEBPS}/{}", file.rel), bytes);
    }
    Ok(package)
}

pub fn container_xml(opf_path: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<container version=\"1.0\" xmlns=\"urn:oasis:names:tc:opendocument:xmlns:container\">\n  <rootfiles>\n    <rootfile full-path=\"{OEBPS}/{}\" media-type=\"application/oebps-package+xml\"/>\n  </rootfiles>\n</container>\n",
        xml_escape(opf_path)
    )
}
