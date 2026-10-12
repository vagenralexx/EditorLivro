use book_epub::{build, inspect, validate, BuildError, BuildRequest, Metadata, ProjectMode};
use std::fs;
use std::io::Read;
use std::path::Path;

const HEAD: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE html>\n<html xmlns=\"http://www.w3.org/1999/xhtml\" lang=\"pt-BR\"><head><meta charset=\"UTF-8\" /><title>T</title></head><body>";

fn chapter(dir: &Path, name: &str, body: &str) {
    fs::create_dir_all(dir).unwrap();
    fs::write(dir.join(name), format!("{HEAD}{body}</body></html>")).unwrap();
}

fn request(root: &Path, title: &str) -> BuildRequest {
    BuildRequest {
        root: root.to_string_lossy().into_owned(),
        metadata: Metadata { title: title.into(), authors: vec!["Autora".into()], language: "pt-BR".into(), ..Metadata::default() },
        ..BuildRequest::default()
    }
}

fn zip_entries(path: &Path) -> Vec<(String, bool)> {
    let mut archive = zip::ZipArchive::new(fs::File::open(path).unwrap()).unwrap();
    (0..archive.len()).map(|i| { let f = archive.by_index(i).unwrap(); (f.name().to_string(), f.compression() == zip::CompressionMethod::Stored) }).collect()
}

fn read_entry(path: &Path, name: &str) -> String {
    let mut archive = zip::ZipArchive::new(fs::File::open(path).unwrap()).unwrap();
    let mut text = String::new();
    archive.by_name(name).unwrap().read_to_string(&mut text).unwrap();
    text
}

#[test]
fn loose_folder_becomes_a_valid_ordered_package() {
    let dir = tempfile::tempdir().unwrap();
    chapter(&dir.path().join("text"), "cap10.xhtml", "<h1>Dez</h1><p>a&nbsp;b</p>");
    chapter(&dir.path().join("text"), "cap2.xhtml", "<h1>Dois</h1><p>texto</p>");
    fs::write(dir.path().join("text/cap2.xhtml.backup-123"), "lixo").unwrap();
    assert_eq!(inspect(&dir.path().to_string_lossy()).unwrap().mode, ProjectMode::Loose);

    let out = dir.path().join("saida/livro.epub");
    let outcome = build(&request(dir.path(), "Livro"), &out.to_string_lossy()).expect("deve gerar");
    assert!(outcome.identifier.starts_with("urn:uuid:"));
    assert_eq!(outcome.report.errors, 0);

    let entries = zip_entries(&out);
    assert_eq!(entries[0], ("mimetype".to_string(), true), "mimetype deve ser o primeiro e sem compressão");
    assert!(entries.iter().all(|(name, _)| !name.contains("backup")));
    assert_eq!(read_entry(&out, "mimetype"), "application/epub+zip");

    let opf = read_entry(&out, "OEBPS/content.opf");
    let two = opf.find("text/cap2.xhtml").unwrap();
    let ten = opf.find("text/cap10.xhtml").unwrap();
    assert!(two < ten, "ordem natural: cap2 antes de cap10");
    assert!(read_entry(&out, "OEBPS/text/cap10.xhtml").contains("a&#160;b"), "&nbsp; vira referência numérica");
    assert!(read_entry(&out, "OEBPS/nav.xhtml").contains("Dois"));
    assert!(!fs::read_dir(dir.path().join("saida")).unwrap().any(|e| e.unwrap().file_name().to_string_lossy().contains("tmp")));
}

#[test]
fn explicit_chapter_order_is_respected() {
    let dir = tempfile::tempdir().unwrap();
    chapter(dir.path(), "a.xhtml", "<h1>A</h1>");
    chapter(dir.path(), "b.xhtml", "<h1>B</h1>");
    let mut req = request(dir.path(), "Livro");
    req.chapters = vec![dir.path().join("b.xhtml").to_string_lossy().into_owned(), "a.xhtml".into()];
    let out = dir.path().join("o.epub");
    build(&req, &out.to_string_lossy()).unwrap();
    let opf = read_entry(&out, "OEBPS/content.opf");
    assert!(opf.find("b.xhtml").unwrap() < opf.find("a.xhtml").unwrap());
}

#[test]
fn malformed_chapter_blocks_the_build_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    chapter(dir.path(), "ok.xhtml", "<p>ok</p>");
    fs::write(dir.path().join("quebrado.xhtml"), "<html><body><p>aberto").unwrap();
    let out = dir.path().join("o.epub");
    match build(&request(dir.path(), "Livro"), &out.to_string_lossy()) {
        Err(BuildError::Invalid(report)) => {
            assert_eq!(report.errors, 1, "o erro de parse deve aparecer uma única vez");
            assert!(report.issues.iter().any(|i| i.code == "XHTML-PARSE" && i.file.as_deref() == Some("OEBPS/quebrado.xhtml")));
        }
        other => panic!("esperava bloqueio, veio {other:?}"),
    }
    assert!(!out.exists());
}

#[test]
fn missing_title_is_an_error_and_missing_cover_a_warning() {
    let dir = tempfile::tempdir().unwrap();
    chapter(dir.path(), "a.xhtml", "<p>x</p>");
    let report = validate(&request(dir.path(), "")).unwrap();
    assert!(report.issues.iter().any(|i| i.code == "META-TITLE"));
    assert!(report.issues.iter().any(|i| i.code == "COVER-MISSING"));
    assert!(!report.can_build());
}

#[test]
fn extracted_epub_is_repacked_keeping_original_content() {
    let dir = tempfile::tempdir().unwrap();
    chapter(&dir.path().join("src"), "a.xhtml", "<h1>A</h1><p>texto original</p>");
    let first = dir.path().join("src/first.epub");
    let mut req = request(&dir.path().join("src"), "Original");
    req.metadata.publisher = "Editora".into();
    build(&req, &first.to_string_lossy()).unwrap();

    // Extrai e reabre como projeto "extraído".
    let extracted = dir.path().join("extraido");
    let mut archive = zip::ZipArchive::new(fs::File::open(&first).unwrap()).unwrap();
    archive.extract(&extracted).unwrap();
    fs::write(extracted.join("OEBPS/solto.txt"), "fora do manifest").unwrap();
    let info = inspect(&extracted.to_string_lossy()).unwrap();
    assert_eq!(info.mode, ProjectMode::Extracted);
    assert_eq!(info.metadata.title, "Original");
    assert_eq!(info.metadata.publisher, "Editora");

    let mut second = BuildRequest { root: extracted.to_string_lossy().into_owned(), ..BuildRequest::default() };
    second.metadata.title = "Novo Título".into();
    let out = dir.path().join("novo.epub");
    let outcome = build(&second, &out.to_string_lossy()).unwrap();
    assert_eq!(outcome.mode, ProjectMode::Extracted);
    let opf = read_entry(&out, "OEBPS/content.opf");
    assert!(opf.contains("<dc:title>Novo Título</dc:title>"));
    assert!(opf.contains("<dc:publisher>Editora</dc:publisher>"), "campos não informados permanecem");
    assert!(read_entry(&out, "OEBPS/a.xhtml").contains("texto original"));
    assert!(zip_entries(&out).iter().all(|(name, _)| name != "OEBPS/solto.txt"));
    assert_eq!(zip_entries(&out)[0].0, "mimetype");
}

#[test]
fn output_must_be_an_epub_file() {
    let dir = tempfile::tempdir().unwrap();
    chapter(dir.path(), "a.xhtml", "<p>x</p>");
    let out = dir.path().join("livro.zip");
    assert!(matches!(build(&request(dir.path(), "L"), &out.to_string_lossy()), Err(BuildError::Failed(_))));
}
