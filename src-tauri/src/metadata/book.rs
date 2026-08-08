use std::path::Path;

use crate::book::{archive, comicinfo, opf, xml};
use crate::error::{NotataError, Result};
use crate::models::book::{
    BookCover, BookKind, BookMetadata, BookMetadataSource, BookProperties,
};

const COMIC_INFO_ENTRY: &str = "ComicInfo.xml";
const CONTAINER_ENTRY: &str = "META-INF/container.xml";
const IMAGE_EXTS: &[&str] = &["jpg", "jpeg", "png", "webp", "gif", "bmp"];

fn kind_for(path: &str) -> BookKind {
    match Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .as_deref()
    {
        Some("epub") => BookKind::Ebook,
        _ => BookKind::Comic,
    }
}

/// Read metadata from inside a comic archive or EPUB.
///
/// Falls back to a filename parse so the editor still opens for archives with
/// no embedded metadata — or for `.cbr`, which needs a RAR decoder this build
/// does not ship.
pub fn read_book_metadata(path: &str) -> Result<BookMetadata> {
    let kind = kind_for(path);

    if archive::is_supported_archive(path) {
        let found = match kind {
            BookKind::Comic => read_comic_info(path),
            BookKind::Ebook => read_epub_opf(path),
        };

        match found {
            Ok(Some(mut meta)) => {
                // A file with no title is less useful than the filename guess.
                if meta.title.is_none() {
                    meta.title = from_filename(path).title;
                }
                return Ok(meta);
            }
            Ok(None) => {}
            Err(e) => log::warn!("Could not read metadata from {}: {}", path, e),
        }
    }

    Ok(from_filename(path))
}

fn read_comic_info(path: &str) -> Result<Option<BookMetadata>> {
    let Some(bytes) = archive::read_entry(path, COMIC_INFO_ENTRY)? else {
        return Ok(None);
    };
    let text = String::from_utf8_lossy(&bytes);
    let mut meta = comicinfo::parse_comic_info(&text)?;
    meta.entry_path = Some(COMIC_INFO_ENTRY.to_string());
    Ok(Some(meta))
}

/// Resolve the OPF package document an EPUB's container points at.
fn find_opf_path(path: &str) -> Result<Option<String>> {
    let Some(bytes) = archive::read_entry(path, CONTAINER_ENTRY)? else {
        return Ok(None);
    };
    let text = String::from_utf8_lossy(&bytes);

    let elements = xml::read_all(&text)?;
    Ok(elements
        .iter()
        .find(|e| e.name.eq_ignore_ascii_case("rootfile"))
        .and_then(|e| {
            e.attributes
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("full-path"))
                .map(|(_, v)| v.clone())
        }))
}

fn read_epub_opf(path: &str) -> Result<Option<BookMetadata>> {
    let Some(opf_path) = find_opf_path(path)? else {
        return Ok(None);
    };
    let Some(bytes) = archive::read_entry(path, &opf_path)? else {
        return Ok(None);
    };

    let text = String::from_utf8_lossy(&bytes);
    let mut meta = opf::parse_opf(&text)?;
    meta.entry_path = Some(opf_path);
    Ok(Some(meta))
}

/// Derive a title (and series/issue for comics) from the filename.
fn from_filename(path: &str) -> BookMetadata {
    let stem = Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default();

    let cleaned: String = stem
        .chars()
        .map(|c| if c == '_' || c == '.' { ' ' } else { c })
        .collect();
    let cleaned = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");

    // "Series 012 (2011)" — a bare number is the issue, parentheses the year.
    let (series, number) = split_series_and_issue(&cleaned);
    let year = find_year(&cleaned);

    BookMetadata {
        kind: kind_for(path),
        title: Some(cleaned.clone()),
        series,
        number,
        year,
        source: BookMetadataSource::Filename,
        ..Default::default()
    }
}

fn split_series_and_issue(name: &str) -> (Option<String>, Option<String>) {
    let tokens: Vec<&str> = name.split_whitespace().collect();

    // Scan for a standalone number that is not a plausible year.
    for (i, token) in tokens.iter().enumerate() {
        let digits = token.trim_start_matches('#');
        if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        let value: i32 = match digits.parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        if (1900..=2100).contains(&value) && digits.len() == 4 {
            continue;
        }
        if i == 0 {
            continue;
        }
        let series = tokens[..i].join(" ");
        if series.is_empty() {
            continue;
        }
        return (Some(series), Some(digits.to_string()));
    }

    (None, None)
}

fn find_year(name: &str) -> Option<i32> {
    let chars: Vec<char> = name.chars().collect();
    let mut found = None;
    for i in 0..chars.len().saturating_sub(3) {
        let slice: String = chars[i..i + 4].iter().collect();
        if slice.chars().all(|c| c.is_ascii_digit()) {
            if let Ok(year) = slice.parse::<i32>() {
                if (1900..=2100).contains(&year) {
                    found = Some(year);
                }
            }
        }
    }
    found
}

pub fn read_book_properties(path: &str) -> Result<BookProperties> {
    let metadata = std::fs::metadata(path)?;
    let container = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("unknown")
        .to_uppercase();

    let readable = archive::is_supported_archive(path);
    let page_count = if readable {
        archive::list_entries(path).ok().map(|entries| {
            entries
                .iter()
                .filter(|name| is_image_entry(name))
                .count() as u32
        })
    } else {
        None
    };

    Ok(BookProperties {
        container,
        file_size: metadata.len(),
        page_count,
        readable,
    })
}

fn is_image_entry(name: &str) -> bool {
    Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| IMAGE_EXTS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// Locate the entry a cover image lives at — the one whose name mentions
/// "cover", falling back to the first page in archive order.
fn find_cover_entry_name(path: &str) -> Result<Option<String>> {
    if !archive::is_supported_archive(path) {
        return Ok(None);
    }

    let entries = archive::list_entries(path)?;
    let mut images: Vec<&String> = entries.iter().filter(|n| is_image_entry(n)).collect();
    images.sort();

    Ok(images
        .iter()
        .find(|n| n.to_lowercase().contains("cover"))
        .or_else(|| images.first())
        .map(|n| (*n).clone()))
}

/// Extract a cover image — the first page of a comic, or the EPUB's declared
/// cover with a first-image fallback.
pub fn read_book_cover(path: &str) -> Result<Option<BookCover>> {
    let Some(entry_name) = find_cover_entry_name(path)? else {
        return Ok(None);
    };

    let Some(bytes) = archive::read_entry(path, &entry_name)? else {
        return Ok(None);
    };

    let mime = match Path::new(&entry_name)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .as_deref()
    {
        Some("png") => "image/png",
        Some("webp") => "image/webp",
        Some("gif") => "image/gif",
        _ => "image/jpeg",
    };

    Ok(Some(BookCover {
        data: base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes),
        mime_type: mime.to_string(),
        entry_path: entry_name,
    }))
}

/// Write metadata back into the archive.
///
/// Comics get a regenerated `ComicInfo.xml`; EPUBs get their existing OPF
/// rewritten in place so the manifest and spine survive untouched.
pub fn write_book_metadata(path: &str, meta: &BookMetadata) -> Result<String> {
    if !archive::is_supported_archive(path) {
        return Err(NotataError::Custom(format!(
            "{} archives cannot be written by Notata — convert to CBZ first",
            Path::new(path)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("These")
                .to_uppercase()
        )));
    }

    match kind_for(path) {
        BookKind::Comic => {
            let xml = comicinfo::to_comic_info_xml(meta);
            archive::replace_entry(path, COMIC_INFO_ENTRY, xml.as_bytes())?;
            Ok(COMIC_INFO_ENTRY.to_string())
        }
        BookKind::Ebook => {
            let opf_path = find_opf_path(path)?.ok_or_else(|| {
                NotataError::Custom("EPUB has no container.xml pointing at a package".into())
            })?;
            let original = archive::read_entry(path, &opf_path)?.ok_or_else(|| {
                NotataError::Custom(format!("EPUB is missing its package document {}", opf_path))
            })?;

            let updated =
                opf::update_opf_metadata(&String::from_utf8_lossy(&original), meta)?;
            archive::replace_entry(path, &opf_path, updated.as_bytes())?;
            Ok(opf_path)
        }
    }
}

fn ext_for_mime(mime_type: &str) -> &'static str {
    match mime_type {
        "image/png" => "png",
        "image/webp" => "webp",
        "image/gif" => "gif",
        _ => "jpg",
    }
}

/// Replace the book's cover image with a manually chosen picture.
///
/// Overwrites the existing cover entry in place if there is one (comics
/// pick their cover by content, not by the manifest, so a mismatched
/// extension is harmless). If the archive has no image at all yet, a new
/// entry is added — but only for comics, since an EPUB's cover must be
/// declared in its manifest, which this does not rewrite.
pub fn write_book_cover(path: &str, image_data: &[u8], mime_type: &str) -> Result<String> {
    if !archive::is_supported_archive(path) {
        return Err(NotataError::Custom(format!(
            "{} archives cannot be written by Notata — convert to CBZ first",
            Path::new(path)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("These")
                .to_uppercase()
        )));
    }

    let entry_name = match find_cover_entry_name(path)? {
        Some(name) => name,
        None if kind_for(path) == BookKind::Comic => {
            format!("000-cover.{}", ext_for_mime(mime_type))
        }
        None => {
            return Err(NotataError::Custom(
                "This EPUB has no existing cover image to replace".to_string(),
            ))
        }
    };

    archive::replace_entry(path, &entry_name, image_data)?;
    Ok(entry_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_series_and_issue_number() {
        assert_eq!(
            split_series_and_issue("Sandman 008"),
            (Some("Sandman".into()), Some("008".into()))
        );
        assert_eq!(
            split_series_and_issue("Saga #12"),
            (Some("Saga".into()), Some("12".into()))
        );
    }

    #[test]
    fn does_not_mistake_a_year_for_an_issue_number() {
        assert_eq!(split_series_and_issue("Watchmen 1986"), (None, None));
    }

    #[test]
    fn reads_year_from_the_name() {
        assert_eq!(find_year("Sandman 008 (1989)"), Some(1989));
        assert_eq!(find_year("Sandman 008"), None);
    }

    #[test]
    fn filename_fallback_populates_a_title() {
        let meta = from_filename("/comics/Sandman_008_(1989).cbz");
        assert_eq!(meta.source, BookMetadataSource::Filename);
        assert_eq!(meta.series.as_deref(), Some("Sandman"));
        assert_eq!(meta.number.as_deref(), Some("008"));
        assert_eq!(meta.year, Some(1989));
        assert_eq!(meta.kind, BookKind::Comic);
    }

    #[test]
    fn epub_extension_selects_the_ebook_kind() {
        assert_eq!(kind_for("/books/dune.epub"), BookKind::Ebook);
        assert_eq!(kind_for("/comics/x.cbz"), BookKind::Comic);
    }

    #[test]
    fn writing_a_cbr_reports_it_as_unsupported() {
        let err = write_book_metadata("/comics/x.cbr", &BookMetadata::default())
            .unwrap_err()
            .to_string();
        assert!(err.contains("CBR"), "unexpected message: {}", err);
    }
}
