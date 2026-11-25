//! `.effector` project files.
//!
//! The macOS app saved projects as a *package* (a folder shown as one file by Finder). Windows has
//! no packages, so projects are now a single zip file with the same contents (document.json,
//! original.<ext>, thumbnail.png). Both layouts are read, so old Mac projects still open.

use std::io::{Read, Write};
use std::path::Path;

pub fn write(path: &Path, document_json: &[u8], original_name: &str, original: &[u8], thumbnail_png: Option<&[u8]>) -> Result<(), String> {
    let tmp = path.with_extension("effector.tmp~");
    {
        let f = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
        let mut z = zip::ZipWriter::new(f);
        let deflate = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        let stored = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        z.start_file("document.json", deflate).map_err(|e| e.to_string())?;
        z.write_all(document_json).map_err(|e| e.to_string())?;
        z.start_file(original_name, stored).map_err(|e| e.to_string())?;
        z.write_all(original).map_err(|e| e.to_string())?;
        if let Some(t) = thumbnail_png {
            z.start_file("thumbnail.png", stored).map_err(|e| e.to_string())?;
            z.write_all(t).map_err(|e| e.to_string())?;
        }
        z.finish().map_err(|e| e.to_string())?;
    }
    // replace an old project (a macOS package folder or a file)
    if path.is_dir() {
        let _ = std::fs::remove_dir_all(path);
    }
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())
}

/// Returns (document.json, original file name, original bytes).
pub fn read(path: &Path) -> Result<(Vec<u8>, String, Vec<u8>), String> {
    let bad = || crate::l10n::tr("This image could not be opened.");
    if path.is_dir() {
        // macOS package
        let json = std::fs::read(path.join("document.json")).map_err(|_| bad())?;
        let orig = std::fs::read_dir(path)
            .map_err(|e| e.to_string())?
            .flatten()
            .find(|e| e.file_name().to_string_lossy().starts_with("original."))
            .ok_or_else(bad)?;
        let data = std::fs::read(orig.path()).map_err(|e| e.to_string())?;
        return Ok((json, orig.file_name().to_string_lossy().to_string(), data));
    }
    let f = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut z = zip::ZipArchive::new(f).map_err(|_| bad())?;
    let mut json = vec![];
    z.by_name("document.json").map_err(|_| bad())?.read_to_end(&mut json).map_err(|e| e.to_string())?;
    let name = (0..z.len())
        .filter_map(|i| z.by_index(i).ok().and_then(|e| e.name().ok().map(|n| n.to_string())))
        .find(|n| n.starts_with("original."))
        .ok_or_else(bad)?;
    let mut data = vec![];
    z.by_name(&name).map_err(|_| bad())?.read_to_end(&mut data).map_err(|e| e.to_string())?;
    Ok((json, name, data))
}
