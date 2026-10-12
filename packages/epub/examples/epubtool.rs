//! Ferramenta de linha de comando para testar o núcleo EPUB fora do aplicativo.
//!
//!   cargo run --example epubtool -- inspect  <pasta>
//!   cargo run --example epubtool -- validate <pasta> [titulo] [autor]
//!   cargo run --example epubtool -- build    <pasta> <saida.epub> [titulo] [autor] [capa.png]

use book_epub::{BuildError, BuildRequest, Metadata};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let command = args.first().map(String::as_str).unwrap_or("");
    let root = args.get(1).cloned().unwrap_or_default();
    match command {
        "inspect" => match book_epub::inspect(&root) {
            Ok(info) => println!("{info:#?}"),
            Err(error) => { eprintln!("erro: {error}"); std::process::exit(2); }
        },
        "validate" | "build" => {
            let offset = if command == "build" { 3 } else { 2 };
            let metadata = Metadata {
                title: args.get(offset).cloned().unwrap_or_default(),
                authors: args.get(offset + 1).map(|a| vec![a.clone()]).unwrap_or_default(),
                language: "pt-BR".into(),
                ..Metadata::default()
            };
            let request = BuildRequest { root, metadata, chapters: vec![], cover: args.get(offset + 2).cloned() };
            if command == "validate" {
                match book_epub::validate(&request) {
                    Ok(report) => print_report(&report),
                    Err(error) => { eprintln!("erro: {error}"); std::process::exit(2); }
                }
            } else {
                match book_epub::build(&request, &args[2]) {
                    Ok(outcome) => { print_report(&outcome.report); println!("OK {} ({} bytes, {} arquivos)", outcome.path, outcome.size_bytes, outcome.file_count); }
                    Err(BuildError::Invalid(report)) => { print_report(&report); eprintln!("bloqueado: há erros"); std::process::exit(1); }
                    Err(BuildError::Failed(error)) => { eprintln!("erro: {error}"); std::process::exit(2); }
                }
            }
        }
        _ => eprintln!("uso: epubtool inspect|validate|build <pasta> ..."),
    }
}

fn print_report(report: &book_epub::Report) {
    for issue in &report.issues {
        println!("[{:?}] {} {} — {}", issue.severity, issue.code, issue.file.as_deref().unwrap_or("-"), issue.message);
    }
    println!("erros={} avisos={}", report.errors, report.warnings);
}
