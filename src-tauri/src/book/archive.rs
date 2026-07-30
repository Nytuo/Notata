use std::io::{Read, Seek, Write};
use std::path::Path;

use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

use crate::error::{NotataError, Result};

/// Read one entry's bytes from a zip archive, matched case-insensitively.
pub fn read_entry(path: &str, wanted: &str) -> Result<Option<Vec<u8>>> {
    let file = std::fs::File::open(path)?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| NotataError::Custom(format!("Not a readable archive: {}", e)))?;

    let name = match find_entry_name(&mut archive, wanted) {
        Some(n) => n,
        None => return Ok(None),
    };

    let mut entry = archive
        .by_name(&name)
        .map_err(|e| NotataError::Custom(e.to_string()))?;
    let mut buf = Vec::new();
    entry.read_to_end(&mut buf)?;
    Ok(Some(buf))
}

/// Locate an entry by exact name or basename, ignoring case.
fn find_entry_name<R: Read + Seek>(archive: &mut ZipArchive<R>, wanted: &str) -> Option<String> {
    let target = wanted.to_lowercase();
    let names: Vec<String> = archive.file_names().map(|n| n.to_string()).collect();

    names
        .iter()
        .find(|n| n.to_lowercase() == target)
        .or_else(|| {
            names.iter().find(|n| {
                n.rsplit('/')
                    .next()
                    .map(|base| base.to_lowercase() == target)
                    .unwrap_or(false)
            })
        })
        .cloned()
}

/// List entry names in archive order.
pub fn list_entries(path: &str) -> Result<Vec<String>> {
    let file = std::fs::File::open(path)?;
    let archive = ZipArchive::new(file)
        .map_err(|e| NotataError::Custom(format!("Not a readable archive: {}", e)))?;
    Ok(archive.file_names().map(|n| n.to_string()).collect())
}

/// Replace (or add) one entry, leaving every other entry byte-identical.
///
/// The new archive is built beside the original and swapped in only after it
/// is fully written, so an interrupted save cannot destroy the book.
pub fn replace_entry(path: &str, entry_name: &str, contents: &[u8]) -> Result<()> {
    let source = Path::new(path);
    let temp_path = source.with_extension(format!(
        "{}.notata-tmp",
        source
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("zip")
    ));

    {
        let input = std::fs::File::open(path)?;
        let mut archive = ZipArchive::new(input)
            .map_err(|e| NotataError::Custom(format!("Not a readable archive: {}", e)))?;

        let existing = find_entry_name(&mut archive, entry_name);
        let target_name = existing.clone().unwrap_or_else(|| entry_name.to_string());

        let output = std::fs::File::create(&temp_path)?;
        let mut writer = ZipWriter::new(output);

        for i in 0..archive.len() {
            let mut entry = archive
                .by_index(i)
                .map_err(|e| NotataError::Custom(e.to_string()))?;
            let name = entry.name().to_string();

            if name == target_name {
                continue; // rewritten below
            }

            let options = SimpleFileOptions::default()
                .compression_method(entry.compression());

            if entry.is_dir() {
                writer
                    .add_directory(name, options)
                    .map_err(|e| NotataError::Custom(e.to_string()))?;
            } else {
                let mut buf = Vec::new();
                entry.read_to_end(&mut buf)?;
                writer
                    .start_file(name, options)
                    .map_err(|e| NotataError::Custom(e.to_string()))?;
                writer.write_all(&buf)?;
            }
        }

        writer
            .start_file(target_name, SimpleFileOptions::default())
            .map_err(|e| NotataError::Custom(e.to_string()))?;
        writer.write_all(contents)?;
        writer
            .finish()
            .map_err(|e| NotataError::Custom(e.to_string()))?;
    }

    // Swap only after the replacement is complete and closed.
    std::fs::rename(&temp_path, path).map_err(|e| {
        let _ = std::fs::remove_file(&temp_path);
        NotataError::Custom(format!("Could not replace the archive: {}", e))
    })?;

    Ok(())
}

/// True for archive formats this build can open. RAR-based comics (`.cbr`)
/// are read-only placeholders — the format needs a proprietary decoder.
pub fn is_supported_archive(path: &str) -> bool {
    matches!(
        Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .as_deref(),
        Some("cbz") | Some("cbt") | Some("epub") | Some("zip")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    fn make_zip(dir: &Path, name: &str, entries: &[(&str, &[u8])]) -> String {
        let path = dir.join(name);
        let file = std::fs::File::create(&path).unwrap();
        let mut writer = ZipWriter::new(file);
        for (entry, data) in entries {
            writer
                .start_file(*entry, SimpleFileOptions::default())
                .unwrap();
            writer.write_all(data).unwrap();
        }
        writer.finish().unwrap();
        path.to_string_lossy().to_string()
    }

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("notata-archive-{}-{}", tag, std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn reads_an_entry_by_name() {
        let dir = temp_dir("read");
        let path = make_zip(
            &dir,
            "a.cbz",
            &[("ComicInfo.xml", b"<ComicInfo/>"), ("001.jpg", b"img")],
        );

        let found = read_entry(&path, "ComicInfo.xml").unwrap().unwrap();
        assert_eq!(found, b"<ComicInfo/>");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn matches_entry_names_case_insensitively() {
        let dir = temp_dir("case");
        let path = make_zip(&dir, "b.cbz", &[("comicinfo.xml", b"<ComicInfo/>")]);

        assert!(read_entry(&path, "ComicInfo.xml").unwrap().is_some());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn returns_none_for_a_missing_entry() {
        let dir = temp_dir("missing");
        let path = make_zip(&dir, "c.cbz", &[("001.jpg", b"img")]);

        assert!(read_entry(&path, "ComicInfo.xml").unwrap().is_none());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn replace_entry_updates_only_the_target() {
        let dir = temp_dir("replace");
        let path = make_zip(
            &dir,
            "d.cbz",
            &[
                ("ComicInfo.xml", b"<old/>"),
                ("001.jpg", b"page-one"),
                ("002.jpg", b"page-two"),
            ],
        );

        replace_entry(&path, "ComicInfo.xml", b"<new/>").unwrap();

        assert_eq!(read_entry(&path, "ComicInfo.xml").unwrap().unwrap(), b"<new/>");
        assert_eq!(read_entry(&path, "001.jpg").unwrap().unwrap(), b"page-one");
        assert_eq!(read_entry(&path, "002.jpg").unwrap().unwrap(), b"page-two");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn replace_entry_adds_the_entry_when_absent() {
        let dir = temp_dir("add");
        let path = make_zip(&dir, "e.cbz", &[("001.jpg", b"page")]);

        replace_entry(&path, "ComicInfo.xml", b"<new/>").unwrap();

        assert_eq!(read_entry(&path, "ComicInfo.xml").unwrap().unwrap(), b"<new/>");
        assert_eq!(read_entry(&path, "001.jpg").unwrap().unwrap(), b"page");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn replace_entry_leaves_no_temporary_file_behind() {
        let dir = temp_dir("temp");
        let path = make_zip(&dir, "f.cbz", &[("001.jpg", b"page")]);

        replace_entry(&path, "ComicInfo.xml", b"<new/>").unwrap();

        let leftovers: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains("notata-tmp"))
            .collect();
        assert!(leftovers.is_empty());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn rar_comics_are_reported_as_unsupported() {
        assert!(!is_supported_archive("/x/book.cbr"));
        assert!(is_supported_archive("/x/book.cbz"));
        assert!(is_supported_archive("/x/book.epub"));
    }
}
