//! Gravação do ZIP no formato exigido pelo EPUB (OCF).

use crate::model::Package;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

/// Grava o pacote em `output` de forma atômica (arquivo temporário + rename).
/// `mimetype` é o primeiro item e não é comprimido, como exige a especificação.
pub fn write_package(package: &Package, output: &Path) -> Result<u64, String> {
    let parent = output.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(|error| format!("Não foi possível criar a pasta de destino: {error}"))?;
    let temp = output.with_extension(format!("epub.tmp-{}", std::process::id()));
    let result = write_zip(package, &temp);
    if let Err(error) = result {
        let _ = fs::remove_file(&temp);
        return Err(error);
    }
    fs::rename(&temp, output).map_err(|error| {
        let _ = fs::remove_file(&temp);
        format!("Não foi possível gravar o arquivo final: {error}")
    })?;
    fs::metadata(output).map(|m| m.len()).map_err(|error| error.to_string())
}

fn write_zip(package: &Package, path: &Path) -> Result<(), String> {
    let file = File::create(path).map_err(|error| format!("Não foi possível criar o arquivo: {error}"))?;
    let mut zip = ZipWriter::new(file);
    let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    zip.start_file("mimetype", stored).map_err(|e| e.to_string())?;
    zip.write_all(b"application/epub+zip").map_err(|e| e.to_string())?;

    let mut ordered: Vec<&crate::model::PackageFile> = package.files.iter().filter(|f| f.path != "mimetype").collect();
    // container.xml logo após o mimetype; o restante em ordem estável.
    ordered.sort_by_key(|f| (f.path != "META-INF/container.xml", f.path.clone()));
    for entry in ordered {
        zip.start_file(entry.path.as_str(), deflated).map_err(|e| format!("{}: {e}", entry.path))?;
        zip.write_all(&entry.data).map_err(|e| format!("{}: {e}", entry.path))?;
    }
    zip.finish().map_err(|e| e.to_string())?;
    Ok(())
}
