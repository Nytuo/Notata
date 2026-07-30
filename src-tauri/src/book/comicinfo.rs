use crate::book::xml::{escape, read_elements};
use crate::error::Result;
use crate::models::book::{BookKind, BookMetadata, BookMetadataSource};

/// Parse a ComicRack `ComicInfo.xml` document.
///
/// The schema is flat — every field is a direct child of `<ComicInfo>` — and
/// multi-value fields are comma-separated strings rather than repeated
/// elements.
pub fn parse_comic_info(xml: &str) -> Result<BookMetadata> {
    let elements = read_elements(xml)?;

    let mut meta = BookMetadata {
        kind: BookKind::Comic,
        source: BookMetadataSource::ComicInfo,
        ..Default::default()
    };

    for (name, value) in &elements {
        let text = value.trim();
        if text.is_empty() {
            continue;
        }

        match name.to_lowercase().as_str() {
            "title" => meta.title = Some(text.to_string()),
            "series" => meta.series = Some(text.to_string()),
            "number" => meta.number = Some(text.to_string()),
            "count" => meta.count = text.parse().ok(),
            "volume" => meta.volume = text.parse().ok(),
            "summary" | "notes" if meta.summary.is_none() => {
                meta.summary = Some(text.to_string())
            }
            "year" => meta.year = text.parse().ok(),
            "month" => meta.month = text.parse().ok(),
            "day" => meta.day = text.parse().ok(),
            "writer" => meta.authors = split_list(text),
            "penciller" => meta.pencillers = split_list(text),
            "inker" => meta.inkers = split_list(text),
            "colorist" => meta.colorists = split_list(text),
            "letterer" => meta.letterers = split_list(text),
            "coverartist" => meta.cover_artists = split_list(text),
            "editor" => meta.editors = split_list(text),
            "translator" => meta.translators = split_list(text),
            "publisher" => meta.publisher = Some(text.to_string()),
            "imprint" => meta.imprint = Some(text.to_string()),
            "genre" => meta.genres = split_list(text),
            "characters" => meta.characters = split_list(text),
            "storyarc" => meta.story_arc = Some(text.to_string()),
            "languageiso" => meta.language = Some(text.to_string()),
            "gtin" | "isbn" => meta.isbn = Some(text.to_string()),
            "pagecount" => meta.page_count = text.parse().ok(),
            "agerating" => meta.age_rating = Some(text.to_string()),
            "web" => meta.web = Some(text.to_string()),
            _ => {}
        }
    }

    Ok(meta)
}

/// ComicInfo packs multiple credits into one comma-separated element.
fn split_list(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn join_list(values: &[String]) -> String {
    values.join(", ")
}

/// Serialize back to ComicInfo.xml, preserving the schema's element order.
pub fn to_comic_info_xml(meta: &BookMetadata) -> String {
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n");
    out.push_str(
        "<ComicInfo xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" \
         xmlns:xsd=\"http://www.w3.org/2001/XMLSchema\">\n",
    );

    fn push(out: &mut String, tag: &str, value: &str) {
        if !value.trim().is_empty() {
            out.push_str(&format!("  <{0}>{1}</{0}>\n", tag, escape(value)));
        }
    }
    macro_rules! push {
        ($tag:expr, $value:expr) => {
            push(&mut out, $tag, $value)
        };
    }

    if let Some(v) = &meta.title {
        push!("Title", v);
    }
    if let Some(v) = &meta.series {
        push!("Series", v);
    }
    if let Some(v) = &meta.number {
        push!("Number", v);
    }
    if let Some(v) = meta.count {
        push!("Count", &v.to_string());
    }
    if let Some(v) = meta.volume {
        push!("Volume", &v.to_string());
    }
    if let Some(v) = &meta.summary {
        push!("Summary", v);
    }
    if let Some(v) = meta.year {
        push!("Year", &v.to_string());
    }
    if let Some(v) = meta.month {
        push!("Month", &v.to_string());
    }
    if let Some(v) = meta.day {
        push!("Day", &v.to_string());
    }

    push!("Writer", &join_list(&meta.authors));
    push!("Penciller", &join_list(&meta.pencillers));
    push!("Inker", &join_list(&meta.inkers));
    push!("Colorist", &join_list(&meta.colorists));
    push!("Letterer", &join_list(&meta.letterers));
    push!("CoverArtist", &join_list(&meta.cover_artists));
    push!("Editor", &join_list(&meta.editors));
    push!("Translator", &join_list(&meta.translators));

    if let Some(v) = &meta.publisher {
        push!("Publisher", v);
    }
    if let Some(v) = &meta.imprint {
        push!("Imprint", v);
    }
    push!("Genre", &join_list(&meta.genres));
    if let Some(v) = &meta.web {
        push!("Web", v);
    }
    if let Some(v) = meta.page_count {
        push!("PageCount", &v.to_string());
    }
    if let Some(v) = &meta.language {
        push!("LanguageISO", v);
    }
    if let Some(v) = &meta.age_rating {
        push!("AgeRating", v);
    }
    push!("Characters", &join_list(&meta.characters));
    if let Some(v) = &meta.story_arc {
        push!("StoryArc", v);
    }
    if let Some(v) = &meta.isbn {
        push!("GTIN", v);
    }

    out.push_str("</ComicInfo>\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_comicinfo_document() {
        let xml = r#"<?xml version="1.0"?>
<ComicInfo>
  <Title>The Vigil</Title>
  <Series>Sandman</Series>
  <Number>8</Number>
  <Count>75</Count>
  <Volume>1</Volume>
  <Summary>Death visits.</Summary>
  <Year>1989</Year>
  <Month>8</Month>
  <Writer>Neil Gaiman</Writer>
  <Penciller>Mike Dringenberg, Malcolm Jones III</Penciller>
  <Publisher>DC Comics</Publisher>
  <Genre>Fantasy, Horror</Genre>
  <PageCount>24</PageCount>
  <LanguageISO>en</LanguageISO>
  <AgeRating>Mature 17+</AgeRating>
</ComicInfo>"#;

        let m = parse_comic_info(xml).unwrap();

        assert_eq!(m.title.as_deref(), Some("The Vigil"));
        assert_eq!(m.series.as_deref(), Some("Sandman"));
        assert_eq!(m.number.as_deref(), Some("8"));
        assert_eq!(m.count, Some(75));
        assert_eq!(m.year, Some(1989));
        assert_eq!(m.month, Some(8));
        assert_eq!(m.authors, vec!["Neil Gaiman"]);
        assert_eq!(
            m.pencillers,
            vec!["Mike Dringenberg", "Malcolm Jones III"]
        );
        assert_eq!(m.genres, vec!["Fantasy", "Horror"]);
        assert_eq!(m.page_count, Some(24));
        assert_eq!(m.language.as_deref(), Some("en"));
        assert_eq!(m.source, BookMetadataSource::ComicInfo);
    }

    #[test]
    fn round_trips_through_the_writer() {
        let original = BookMetadata {
            kind: BookKind::Comic,
            title: Some("The Vigil".into()),
            series: Some("Sandman".into()),
            number: Some("8".into()),
            year: Some(1989),
            authors: vec!["Neil Gaiman".into()],
            pencillers: vec!["Mike Dringenberg".into(), "Malcolm Jones III".into()],
            genres: vec!["Fantasy".into(), "Horror".into()],
            publisher: Some("DC Comics".into()),
            page_count: Some(24),
            ..Default::default()
        };

        let parsed = parse_comic_info(&to_comic_info_xml(&original)).unwrap();

        assert_eq!(parsed.title, original.title);
        assert_eq!(parsed.series, original.series);
        assert_eq!(parsed.number, original.number);
        assert_eq!(parsed.year, original.year);
        assert_eq!(parsed.authors, original.authors);
        assert_eq!(parsed.pencillers, original.pencillers);
        assert_eq!(parsed.genres, original.genres);
        assert_eq!(parsed.page_count, original.page_count);
    }

    #[test]
    fn escapes_markup_in_values() {
        let meta = BookMetadata {
            title: Some("Tom & Jerry <Special>".into()),
            ..Default::default()
        };
        let parsed = parse_comic_info(&to_comic_info_xml(&meta)).unwrap();
        assert_eq!(parsed.title.as_deref(), Some("Tom & Jerry <Special>"));
    }

    #[test]
    fn omits_empty_credit_lists() {
        let meta = BookMetadata {
            title: Some("X".into()),
            ..Default::default()
        };
        let xml = to_comic_info_xml(&meta);
        assert!(!xml.contains("<Writer>"));
        assert!(!xml.contains("<Genre>"));
    }
}
