use crate::book::xml::{escape, read_all};
use crate::error::Result;
use crate::models::book::{BookKind, BookMetadata, BookMetadataSource};

/// Parse the Dublin Core block of an EPUB OPF package document.
///
/// EPUB stores metadata as `dc:` elements inside `<metadata>`; roles such as
/// author vs. translator live in an `opf:role` attribute on `dc:creator`.
pub fn parse_opf(xml: &str) -> Result<BookMetadata> {
    let elements = read_all(xml)?;

    let mut meta = BookMetadata {
        kind: BookKind::Ebook,
        source: BookMetadataSource::Opf,
        ..Default::default()
    };

    for element in &elements {
        let text = element.text.trim();
        let attr = |key: &str| {
            element
                .attributes
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v.as_str())
        };

        match element.name.to_lowercase().as_str() {
            "title" if !text.is_empty() && meta.title.is_none() => {
                meta.title = Some(text.to_string())
            }
            "creator" if !text.is_empty() => {
                // marc relator codes: aut = author, trl = translator,
                // ill = illustrator, edt = editor.
                match attr("role") {
                    Some("trl") => meta.translators.push(text.to_string()),
                    Some("edt") => meta.editors.push(text.to_string()),
                    Some("ill") => meta.pencillers.push(text.to_string()),
                    _ => meta.authors.push(text.to_string()),
                }
            }
            "contributor" if !text.is_empty() => match attr("role") {
                Some("trl") => meta.translators.push(text.to_string()),
                Some("edt") => meta.editors.push(text.to_string()),
                _ => {}
            },
            "publisher" if !text.is_empty() => meta.publisher = Some(text.to_string()),
            "language" if !text.is_empty() => meta.language = Some(text.to_string()),
            "description" if !text.is_empty() => meta.summary = Some(text.to_string()),
            "subject" if !text.is_empty() => meta.genres.push(text.to_string()),
            "rights" if !text.is_empty() => meta.rights = Some(text.to_string()),
            "date" if !text.is_empty() => {
                if meta.year.is_none() {
                    meta.year = text.get(0..4).and_then(|y| y.parse().ok());
                }
            }
            "identifier" if !text.is_empty() => {
                let scheme = attr("scheme").unwrap_or_default().to_lowercase();
                let is_isbn = scheme.contains("isbn")
                    || text.to_lowercase().starts_with("urn:isbn:");
                if is_isbn && meta.isbn.is_none() {
                    meta.isbn = Some(
                        text.trim_start_matches("urn:isbn:")
                            .trim_start_matches("URN:ISBN:")
                            .to_string(),
                    );
                }
            }
            "meta" => {
                // EPUB 3 series info rides on <meta property="belongs-to-collection">.
                match attr("property") {
                    Some("belongs-to-collection") if !text.is_empty() => {
                        meta.series = Some(text.to_string())
                    }
                    Some("group-position") if !text.is_empty() => {
                        meta.number = Some(text.to_string())
                    }
                    _ => {
                        // EPUB 2 Calibre convention: <meta name=... content=...>
                        if let (Some(name), Some(content)) = (attr("name"), attr("content")) {
                            match name {
                                "calibre:series" => meta.series = Some(content.to_string()),
                                "calibre:series_index" => {
                                    meta.number = Some(content.to_string())
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    Ok(meta)
}

/// Rewrite an OPF document's `<metadata>` block with updated values.
///
/// The rest of the package — manifest, spine, guide — is copied verbatim,
/// because rebuilding it would break the book's reading order.
pub fn update_opf_metadata(original: &str, meta: &BookMetadata) -> Result<String> {
    let (start, end) = match find_metadata_block(original) {
        Some(range) => range,
        None => {
            return Err(crate::error::NotataError::Custom(
                "OPF has no <metadata> block to update".to_string(),
            ))
        }
    };

    let existing = &original[start..end];
    // Keep elements the editor does not model, so identifiers and custom
    // Calibre fields survive a save.
    let preserved: String = existing
        .lines()
        .filter(|line| {
            let l = line.trim().to_lowercase();
            l.starts_with("<dc:identifier")
                || l.starts_with("<identifier")
                || (l.starts_with("<meta") && !l.contains("belongs-to-collection")
                    && !l.contains("calibre:series"))
        })
        .map(|l| format!("    {}\n", l.trim()))
        .collect();

    let mut block = String::from("\n");
    // A plain fn rather than a closure so the blocks below can also borrow
    // `block` for the elements that carry attributes.
    fn push(block: &mut String, tag: &str, value: &str) {
        if !value.trim().is_empty() {
            block.push_str(&format!("    <dc:{0}>{1}</dc:{0}>\n", tag, escape(value)));
        }
    }
    macro_rules! push {
        ($tag:expr, $value:expr) => {
            push(&mut block, $tag, $value)
        };
    }

    if let Some(v) = &meta.title {
        push!("title", v);
    }
    for author in &meta.authors {
        block.push_str(&format!(
            "    <dc:creator opf:role=\"aut\">{}</dc:creator>\n",
            escape(author)
        ));
    }
    for translator in &meta.translators {
        block.push_str(&format!(
            "    <dc:contributor opf:role=\"trl\">{}</dc:contributor>\n",
            escape(translator)
        ));
    }
    if let Some(v) = &meta.publisher {
        push!("publisher", v);
    }
    if let Some(v) = &meta.language {
        push!("language", v);
    }
    if let Some(v) = &meta.summary {
        push!("description", v);
    }
    for genre in &meta.genres {
        push!("subject", genre);
    }
    if let Some(v) = &meta.rights {
        push!("rights", v);
    }
    if let Some(v) = meta.year {
        push!("date", &v.to_string());
    }
    if let Some(v) = &meta.isbn {
        block.push_str(&format!(
            "    <dc:identifier opf:scheme=\"ISBN\">{}</dc:identifier>\n",
            escape(v)
        ));
    }
    if let Some(v) = &meta.series {
        block.push_str(&format!(
            "    <meta name=\"calibre:series\" content=\"{}\"/>\n",
            escape(v)
        ));
    }
    if let Some(v) = &meta.number {
        block.push_str(&format!(
            "    <meta name=\"calibre:series_index\" content=\"{}\"/>\n",
            escape(v)
        ));
    }

    block.push_str(&preserved);
    block.push_str("  ");

    Ok(format!(
        "{}{}{}",
        &original[..start],
        block,
        &original[end..]
    ))
}

/// Byte range of the content between `<metadata ...>` and `</metadata>`.
fn find_metadata_block(xml: &str) -> Option<(usize, usize)> {
    let lower = xml.to_lowercase();
    let open_tag = lower.find("<metadata")?;
    let open_end = lower[open_tag..].find('>')? + open_tag + 1;
    let close = lower.find("</metadata>")?;
    if close < open_end {
        return None;
    }
    Some((open_end, close))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"<?xml version="1.0"?>
<package xmlns:opf="http://www.idpf.org/2007/opf" version="2.0">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Dune</dc:title>
    <dc:creator opf:role="aut">Frank Herbert</dc:creator>
    <dc:contributor opf:role="trl">A Translator</dc:contributor>
    <dc:publisher>Chilton Books</dc:publisher>
    <dc:language>en</dc:language>
    <dc:date>1965-08-01</dc:date>
    <dc:subject>Science Fiction</dc:subject>
    <dc:identifier opf:scheme="ISBN">9780441013593</dc:identifier>
    <dc:identifier id="uuid">urn:uuid:12345</dc:identifier>
    <meta name="calibre:series" content="Dune Chronicles"/>
    <meta name="calibre:series_index" content="1"/>
  </metadata>
  <manifest><item id="c" href="c.xhtml"/></manifest>
  <spine><itemref idref="c"/></spine>
</package>"#;

    #[test]
    fn parses_dublin_core_metadata() {
        let m = parse_opf(SAMPLE).unwrap();

        assert_eq!(m.title.as_deref(), Some("Dune"));
        assert_eq!(m.authors, vec!["Frank Herbert"]);
        assert_eq!(m.translators, vec!["A Translator"]);
        assert_eq!(m.publisher.as_deref(), Some("Chilton Books"));
        assert_eq!(m.language.as_deref(), Some("en"));
        assert_eq!(m.year, Some(1965));
        assert_eq!(m.genres, vec!["Science Fiction"]);
        assert_eq!(m.isbn.as_deref(), Some("9780441013593"));
        assert_eq!(m.series.as_deref(), Some("Dune Chronicles"));
        assert_eq!(m.number.as_deref(), Some("1"));
        assert_eq!(m.source, BookMetadataSource::Opf);
    }

    #[test]
    fn update_preserves_manifest_and_spine() {
        let meta = parse_opf(SAMPLE).unwrap();
        let updated = update_opf_metadata(SAMPLE, &meta).unwrap();

        assert!(updated.contains("<manifest>"));
        assert!(updated.contains("<itemref idref=\"c\"/>"));
        assert!(updated.contains("</package>"));
    }

    #[test]
    fn update_round_trips_edited_values() {
        let mut meta = parse_opf(SAMPLE).unwrap();
        meta.title = Some("Dune Messiah".into());
        meta.authors = vec!["Frank Herbert".into()];

        let reparsed = parse_opf(&update_opf_metadata(SAMPLE, &meta).unwrap()).unwrap();

        assert_eq!(reparsed.title.as_deref(), Some("Dune Messiah"));
        assert_eq!(reparsed.authors, vec!["Frank Herbert"]);
        assert_eq!(reparsed.series.as_deref(), Some("Dune Chronicles"));
    }

    #[test]
    fn update_keeps_the_uuid_identifier() {
        let meta = parse_opf(SAMPLE).unwrap();
        let updated = update_opf_metadata(SAMPLE, &meta).unwrap();
        assert!(updated.contains("urn:uuid:12345"));
    }

    #[test]
    fn rejects_a_package_without_metadata() {
        assert!(update_opf_metadata("<package></package>", &BookMetadata::default()).is_err());
    }
}
