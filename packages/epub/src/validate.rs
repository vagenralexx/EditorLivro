//! Validação estrutural do pacote EPUB antes da gravação.
//!
//! Cobre os erros que mais derrubam um livro em leitores e lojas (checklist de
//! `producao.md` da skill). Não substitui o epubcheck oficial, mas bloqueia a
//! geração de pacotes que certamente seriam rejeitados.

use crate::model::{Package, Report};
use crate::opf::{parse_opf, parse_xml, Opf};
use crate::util::*;
use std::collections::{HashMap, HashSet};

const XHTML_NS: &str = "http://www.w3.org/1999/xhtml";

fn valid_language(tag: &str) -> bool {
    let parts: Vec<&str> = tag.split('-').collect();
    !tag.is_empty()
        && parts.iter().all(|part| !part.is_empty() && part.len() <= 8 && part.chars().all(|c| c.is_ascii_alphanumeric()))
        && parts[0].len() >= 2
        && parts[0].chars().all(|c| c.is_ascii_alphabetic())
}

fn valid_modified(value: &str) -> bool {
    let b = value.as_bytes();
    b.len() == 20 && b[4] == b'-' && b[7] == b'-' && b[10] == b'T' && b[13] == b':' && b[16] == b':' && b[19] == b'Z'
        && value.chars().enumerate().all(|(i, c)| matches!(i, 4 | 7 | 10 | 13 | 16 | 19) || c.is_ascii_digit())
}

fn is_external(href: &str) -> bool {
    let lower = href.to_ascii_lowercase();
    lower.contains("://") || lower.starts_with("mailto:") || lower.starts_with("tel:") || lower.starts_with("data:") || lower.starts_with("javascript:")
}

pub fn validate_package(package: &Package, report: &mut Report) {
    // Caminhos do ZIP.
    let mut lowered: HashMap<String, &str> = HashMap::new();
    for file in &package.files {
        if file.path.contains('\\') || file.path.split('/').any(|p| p == ".." || p.is_empty()) || file.path.starts_with('/') {
            report.error("ZIP-PATH", Some(&file.path), "Caminho inválido dentro do pacote.");
        }
        if file.path.contains(' ') {
            report.warn("ZIP-NAME-SPACES", Some(&file.path), "O nome do arquivo tem espaços; alguns leitores antigos não abrem esses arquivos. Prefira nomes como capitulo-01.xhtml.");
        }
        if let Some(previous) = lowered.insert(file.path.to_ascii_lowercase(), &file.path) {
            report.error("ZIP-DUPLICATE", Some(&file.path), format!("Nome duplicado (ignorando maiúsculas/minúsculas) com {previous}."));
        }
    }
    if package.get("META-INF/encryption.xml").is_some() {
        report.warn("ENCRYPTION", Some("META-INF/encryption.xml"), "O pacote contém encryption.xml (ofuscação de fontes ou proteção de terceiros). Foi mantido: este aplicativo não remove proteções.");
    }

    // container.xml → OPF.
    let Some(container) = package.get("META-INF/container.xml") else {
        report.error("CONTAINER-MISSING", None, "META-INF/container.xml não existe.");
        return;
    };
    let container_text = String::from_utf8_lossy(&container.data).into_owned();
    let opf_path = match parse_xml(&container_text) {
        Ok(doc) => doc.descendants().find(|n| n.is_element() && n.tag_name().name() == "rootfile").and_then(|n| n.attribute("full-path")).map(str::to_string),
        Err(error) => {
            report.error("CONTAINER-PARSE", Some("META-INF/container.xml"), format!("container.xml malformado: {error}"));
            return;
        }
    };
    let Some(opf_path) = opf_path else {
        report.error("CONTAINER-ROOTFILE", Some("META-INF/container.xml"), "container.xml não declara o arquivo OPF (rootfile).");
        return;
    };
    let Some(opf_file) = package.get(&opf_path) else {
        report.error("OPF-MISSING", Some(&opf_path), "O arquivo OPF declarado em container.xml não existe no pacote.");
        return;
    };
    let opf_text = String::from_utf8_lossy(&opf_file.data).into_owned();
    let opf = match parse_opf(&opf_text) {
        Ok(opf) => opf,
        Err(error) => {
            report.error("OPF-PARSE", Some(&opf_path), format!("OPF malformado: {error}"));
            return;
        }
    };
    check_metadata(&opf, &opf_path, report);
    let by_id = check_manifest(&opf, &opf_path, package, report);
    check_spine(&opf, &opf_path, &by_id, report);
    check_navigation(&opf, &opf_path, package, report);
    check_cover(&opf, &opf_path, &by_id, report);

    // Conteúdo XHTML do manifest.
    let known: HashSet<&str> = package.files.iter().map(|f| f.path.as_str()).collect();
    for item in &opf.manifest {
        if item.media_type != "application/xhtml+xml" { continue; }
        if let Some(path) = resolve_href(&opf_path, &percent_decode(&item.href)) {
            if let Some(file) = package.get(&path) {
                check_xhtml(&path, &file.data, &known, report);
            }
        }
    }
    // Arquivos no pacote que o manifest não declara.
    let declared: HashSet<String> = opf.manifest.iter().filter_map(|i| resolve_href(&opf_path, &percent_decode(&i.href))).collect();
    for file in &package.files {
        if file.path == opf_path || file.path.starts_with("META-INF/") { continue; }
        if !declared.contains(&file.path) {
            report.warn("MANIFEST-UNDECLARED", Some(&file.path), "Arquivo presente no pacote mas não declarado no manifest.");
        }
    }
}

fn check_metadata(opf: &Opf, opf_path: &str, report: &mut Report) {
    let file = Some(opf_path);
    if opf.identifiers.is_empty() {
        report.error("OPF-IDENTIFIER", file, "O OPF não tem dc:identifier.");
    } else if !opf.identifiers.iter().any(|(id, _)| id.as_deref() == Some(opf.unique_identifier.as_str())) {
        report.error("OPF-UNIQUE-ID", file, "O atributo unique-identifier do package não aponta para nenhum dc:identifier.");
    }
    if opf.metadata.title.trim().is_empty() { report.error("OPF-TITLE", file, "O OPF não tem dc:title."); }
    if opf.metadata.language.is_empty() {
        report.error("OPF-LANGUAGE", file, "O OPF não tem dc:language.");
    } else if !valid_language(&opf.metadata.language) {
        report.error("OPF-LANGUAGE", file, format!("Idioma inválido: “{}”. Use códigos como pt-BR ou en.", opf.metadata.language));
    }
    if opf.version.starts_with('3') {
        match &opf.modified {
            None => report.error("OPF-MODIFIED", file, "EPUB 3 exige <meta property=\"dcterms:modified\">."),
            Some(value) if !valid_modified(value) => report.error("OPF-MODIFIED", file, format!("dcterms:modified inválido: “{value}” (use AAAA-MM-DDThh:mm:ssZ).")),
            _ => {}
        }
    } else if opf.version.starts_with('2') {
        report.warn("EPUB-VERSION", file, "Este é um EPUB 2. Ele funciona no Kindle, mas lojas e leitores atuais preferem EPUB 3.");
    } else {
        report.error("OPF-VERSION", file, "O OPF não declara a versão do EPUB (version=\"3.0\").");
    }
    if opf.metadata.authors.is_empty() { report.warn("OPF-CREATOR", file, "O OPF não tem autor (dc:creator)."); }
}

fn check_manifest<'a>(opf: &'a Opf, opf_path: &str, package: &Package, report: &mut Report) -> HashMap<&'a str, &'a crate::opf::ManifestItem> {
    let mut by_id: HashMap<&str, &crate::opf::ManifestItem> = HashMap::new();
    for item in &opf.manifest {
        if item.id.is_empty() || item.href.is_empty() || item.media_type.is_empty() {
            report.error("MANIFEST-ITEM", Some(opf_path), "Item do manifest sem id, href ou media-type.");
            continue;
        }
        if by_id.insert(&item.id, item).is_some() {
            report.error("MANIFEST-ID", Some(opf_path), format!("id duplicado no manifest: {}.", item.id));
        }
        if is_external(&item.href) { continue; }
        let decoded = percent_decode(item.href.split('#').next().unwrap_or(""));
        match resolve_href(opf_path, &decoded) {
            Some(path) if package.get(&path).is_some() => {
                if let Some(expected) = media_type_for(&path) {
                    let same = expected == item.media_type || (expected == "text/javascript" && item.media_type == "application/javascript");
                    if !same { report.warn("MANIFEST-MEDIA-TYPE", Some(&path), format!("media-type “{}” difere do esperado para a extensão (“{expected}”).", item.media_type)); }
                }
            }
            Some(path) => report.error("MANIFEST-FILE", Some(&path), format!("Item “{}” do manifest aponta para um arquivo que não existe.", item.id)),
            None => report.error("MANIFEST-FILE", Some(&item.href), "href do manifest escapa da raiz do pacote."),
        }
    }
    by_id
}

fn check_spine(opf: &Opf, opf_path: &str, by_id: &HashMap<&str, &crate::opf::ManifestItem>, report: &mut Report) {
    if opf.spine.is_empty() {
        report.error("SPINE-EMPTY", Some(opf_path), "A spine está vazia: o livro não tem conteúdo de leitura.");
    }
    let mut seen = HashSet::new();
    for itemref in &opf.spine {
        if !seen.insert(&itemref.idref) { report.warn("SPINE-DUPLICATE", Some(opf_path), format!("itemref repetido na spine: {}.", itemref.idref)); }
        match by_id.get(itemref.idref.as_str()) {
            None => report.error("SPINE-IDREF", Some(opf_path), format!("A spine referencia “{}”, que não existe no manifest.", itemref.idref)),
            Some(item) if item.media_type != "application/xhtml+xml" && item.media_type != "image/svg+xml" => {
                report.error("SPINE-MEDIA-TYPE", Some(opf_path), format!("“{}” está na spine mas não é XHTML (media-type {}).", itemref.idref, item.media_type));
            }
            _ => {}
        }
    }
    if !opf.spine.is_empty() && !opf.spine.iter().any(|s| s.linear) {
        report.error("SPINE-LINEAR", Some(opf_path), "Nenhum item da spine é linear.");
    }
}

fn check_navigation(opf: &Opf, opf_path: &str, package: &Package, report: &mut Report) {
    if opf.version.starts_with('3') {
        let navs: Vec<_> = opf.manifest.iter().filter(|i| i.properties.iter().any(|p| p == "nav")).collect();
        match navs.len() {
            0 => report.error("NAV-MISSING", Some(opf_path), "EPUB 3 exige um documento de navegação (item com properties=\"nav\")."),
            1 => {
                if let Some(path) = resolve_href(opf_path, &percent_decode(&navs[0].href)) {
                    if let Some(file) = package.get(&path) {
                        let text = String::from_utf8_lossy(&file.data).into_owned();
                        match parse_xml(&text) {
                            Ok(doc) => {
                                let toc = doc.descendants().find(|n| n.is_element() && n.tag_name().name() == "nav" && n.attributes().any(|a| a.name() == "type" && a.value().split_whitespace().any(|v| v == "toc")));
                                match toc {
                                    None => report.error("NAV-TOC", Some(&path), "O documento de navegação não tem <nav epub:type=\"toc\">."),
                                    Some(toc) => {
                                        let links: Vec<_> = toc.descendants().filter(|n| n.is_element() && n.tag_name().name() == "a").collect();
                                        if links.is_empty() { report.warn("NAV-EMPTY", Some(&path), "O sumário não tem nenhum link."); }
                                        for link in links {
                                            let href = link.attribute("href").unwrap_or("");
                                            if href.is_empty() || is_external(href) { continue; }
                                            let target = percent_decode(href.split('#').next().unwrap_or(""));
                                            if target.is_empty() { continue; }
                                            match resolve_href(&path, &target) {
                                                Some(resolved) if package.get(&resolved).is_some() => {}
                                                _ => report.error("NAV-LINK", Some(&path), format!("O sumário aponta para “{href}”, que não existe.")),
                                            }
                                        }
                                    }
                                }
                            }
                            Err(error) => report.error("NAV-PARSE", Some(&path), format!("Documento de navegação malformado: {error}")),
                        }
                    }
                }
            }
            _ => report.error("NAV-MULTIPLE", Some(opf_path), "Há mais de um item com properties=\"nav\"."),
        }
    }
    if let Some(toc_id) = &opf.spine_toc {
        match opf.manifest.iter().find(|i| &i.id == toc_id) {
            None => report.warn("NCX-MISSING", Some(opf_path), "A spine aponta para um NCX que não existe no manifest."),
            Some(item) if item.media_type != "application/x-dtbncx+xml" => report.warn("NCX-MEDIA-TYPE", Some(opf_path), "O item referenciado por spine@toc não é um NCX."),
            _ => {}
        }
    } else if opf.version.starts_with('2') {
        report.error("NCX-REQUIRED", Some(opf_path), "EPUB 2 exige spine@toc apontando para o NCX.");
    }
}

fn check_cover(opf: &Opf, opf_path: &str, by_id: &HashMap<&str, &crate::opf::ManifestItem>, report: &mut Report) {
    let by_property = opf.manifest.iter().find(|i| i.properties.iter().any(|p| p == "cover-image"));
    let by_meta = opf.cover_meta.as_deref().and_then(|id| by_id.get(id).copied());
    match by_property.or(by_meta) {
        None => report.warn("COVER-MISSING", Some(opf_path), "O livro não declara imagem de capa (cover-image)."),
        Some(item) if !item.media_type.starts_with("image/") => report.error("COVER-TYPE", Some(opf_path), "A capa declarada não é uma imagem."),
        _ => {}
    }
}

fn check_xhtml(path: &str, data: &[u8], known: &HashSet<&str>, report: &mut Report) {
    if data.is_empty() {
        report.error("XHTML-EMPTY", Some(path), "Arquivo vazio.");
        return;
    }
    let Ok(text) = std::str::from_utf8(data) else {
        report.error("XHTML-ENCODING", Some(path), "O arquivo não está em UTF-8.");
        return;
    };
    if data.len() < 100 { report.warn("XHTML-TINY", Some(path), "Arquivo muito curto (menos de 100 bytes): pode estar vazio ou corrompido."); }
    let doc = match parse_xml(text) {
        Ok(doc) => doc,
        Err(error) => {
            report.error("XHTML-PARSE", Some(path), format!("XHTML malformado: {error}"));
            return;
        }
    };
    let root = doc.root_element();
    if root.tag_name().name() != "html" || root.tag_name().namespace() != Some(XHTML_NS) {
        report.error("XHTML-ROOT", Some(path), "O elemento raiz deve ser <html xmlns=\"http://www.w3.org/1999/xhtml\">.");
    }
    let title_ok = doc.descendants().any(|n| n.is_element() && n.tag_name().name() == "title" && n.text().map(|t| !t.trim().is_empty()).unwrap_or(false));
    if !title_ok { report.warn("XHTML-TITLE", Some(path), "O documento não tem <title> preenchido."); }

    for node in doc.descendants().filter(|n| n.is_element()) {
        let name = node.tag_name().name();
        let (attribute, hard) = match name {
            "img" => (node.attribute("src"), true),
            "link" => (node.attribute("href"), true),
            "a" => (node.attribute("href"), false),
            "image" => (node.attributes().find(|a| a.name() == "href").map(|a| a.value()), true),
            "script" => {
                report.warn("XHTML-SCRIPT", Some(path), "O documento contém <script>: lojas e leitores podem rejeitar conteúdo com scripts.");
                continue;
            }
            _ => continue,
        };
        let Some(reference) = attribute else { continue };
        if reference.is_empty() || reference.starts_with('#') { continue; }
        if is_external(reference) {
            if hard && !reference.to_ascii_lowercase().starts_with("mailto:") {
                report.warn("XHTML-REMOTE", Some(path), format!("Recurso remoto “{reference}”: o livro deixa de funcionar offline."));
            }
            continue;
        }
        let target = percent_decode(reference.split(['#', '?']).next().unwrap_or(""));
        if target.is_empty() { continue; }
        let exists = resolve_href(path, &target).map(|resolved| known.contains(resolved.as_str())).unwrap_or(false);
        if !exists {
            if hard { report.error("XHTML-RESOURCE", Some(path), format!("Recurso “{reference}” não existe no pacote.")); }
            else { report.warn("XHTML-LINK", Some(path), format!("Link interno quebrado: “{reference}”.")); }
        }
    }
}
