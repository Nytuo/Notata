use quick_xml::events::Event;
use quick_xml::Reader;

use crate::error::{NotataError, Result};
use crate::models::video::{ActorCredit, VideoKind, VideoMetadata, VideoMetadataSource};

/// Parse a Kodi-style NFO document.
///
/// Handles the three root elements the media servers write — `<movie>`,
/// `<episodedetails>`, and `<tvshow>` — and ignores unknown elements so a
/// richer NFO from another tool still round-trips its known fields.
pub fn parse_nfo(xml: &str) -> Result<VideoMetadata> {
    let mut reader = Reader::from_str(xml);
    // Text is trimmed once per element after the fragments are joined; trimming
    // each fragment would swallow the spaces around entity references.
    reader.config_mut().trim_text(false);

    let mut meta = VideoMetadata {
        source: VideoMetadataSource::Nfo,
        ..Default::default()
    };

    // Path of open elements, so nested fields like <actor><name> are
    // attributed to the right parent.
    let mut stack: Vec<String> = Vec::new();
    let mut actor = ActorCredit::default();
    let mut in_actor = false;
    let mut saw_root = false;
    let mut buf = Vec::new();
    // Text arrives in fragments — entity references are separate events — so
    // an element's content is accumulated and applied when it closes.
    let mut text_buf = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_lowercase();

                if !saw_root {
                    match name.as_str() {
                        "movie" => {
                            meta.kind = VideoKind::Movie;
                            saw_root = true;
                        }
                        "episodedetails" => {
                            meta.kind = VideoKind::Episode;
                            saw_root = true;
                        }
                        "tvshow" => {
                            // Series-level data; treat as a movie-shaped record
                            // since it describes the show, not an episode.
                            meta.kind = VideoKind::Movie;
                            saw_root = true;
                        }
                        _ => {}
                    }
                }

                if name == "actor" {
                    in_actor = true;
                    actor = ActorCredit::default();
                }
                stack.push(name);
                text_buf.clear();
            }
            Ok(Event::End(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_lowercase();

                let text = text_buf.trim().to_string();
                if !text.is_empty() {
                    apply_field(&mut meta, &name, &text, in_actor, &mut actor, &stack);
                }
                text_buf.clear();

                if name == "actor" {
                    if !actor.name.trim().is_empty() {
                        meta.actors.push(std::mem::take(&mut actor));
                    }
                    in_actor = false;
                }
                stack.pop();
            }
            Ok(Event::Text(e)) => {
                text_buf.push_str(
                    &e.xml_content(quick_xml::XmlVersion::Implicit1_0)
                        .unwrap_or_default(),
                );
            }
            Ok(Event::GeneralRef(e)) => {
                // Named entity references arrive as their own event.
                let name = String::from_utf8_lossy(e.as_ref()).to_string();
                if let Some(ch) = resolve_entity(&name) {
                    text_buf.push(ch);
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(NotataError::Custom(format!(
                    "Malformed NFO at position {}: {}",
                    reader.buffer_position(),
                    e
                )))
            }
            _ => {}
        }
        buf.clear();
    }

    if !saw_root {
        return Err(NotataError::Custom(
            "Not a recognised NFO — expected <movie>, <episodedetails>, or <tvshow>".to_string(),
        ));
    }

    Ok(meta)
}

fn apply_field(
    meta: &mut VideoMetadata,
    tag: &str,
    text: &str,
    in_actor: bool,
    actor: &mut ActorCredit,
    stack: &[String],
) {
    if in_actor {
        match tag {
            "name" => actor.name = text.to_string(),
            "role" => actor.role = Some(text.to_string()),
            "thumb" => actor.thumb = Some(text.to_string()),
            _ => {}
        }
        return;
    }

    // <rating><value> nests one level deeper than the flat <rating> form.
    let parent = stack.len().checked_sub(2).and_then(|i| stack.get(i));

    match tag {
        "title" => meta.title = Some(text.to_string()),
        "originaltitle" => meta.original_title = Some(text.to_string()),
        "sorttitle" => meta.sort_title = Some(text.to_string()),
        "year" => meta.year = text.parse().ok(),
        "premiered" | "releasedate" => meta.release_date = Some(text.to_string()),
        "tagline" => meta.tagline = Some(text.to_string()),
        "plot" => meta.plot = Some(text.to_string()),
        "outline" => meta.outline = Some(text.to_string()),
        "runtime" => meta.runtime_minutes = text.parse().ok(),
        "mpaa" | "certification" => meta.certification = Some(text.to_string()),
        "genre" => push_unique(&mut meta.genres, text),
        "studio" => push_unique(&mut meta.studios, text),
        "country" => push_unique(&mut meta.countries, text),
        "director" => push_unique(&mut meta.directors, text),
        "credits" | "writer" => push_unique(&mut meta.writers, text),
        "tag" => push_unique(&mut meta.tags, text),
        "showtitle" => meta.show_title = Some(text.to_string()),
        "season" => meta.season = text.parse().ok(),
        "episode" => meta.episode = text.parse().ok(),
        "aired" => meta.aired = Some(text.to_string()),
        "trailer" => meta.trailer = Some(text.to_string()),
        "votes" => meta.votes = text.replace([',', '.'], "").parse().ok(),
        "rating" => meta.rating = text.parse().ok(),
        "value" if parent.map(|p| p == "rating").unwrap_or(false) => {
            meta.rating = text.parse().ok()
        }
        "id" | "imdbid" => {
            // Kodi writes the IMDb id in a bare <id> for movies.
            if text.starts_with("tt") {
                meta.imdb_id = Some(text.to_string());
            } else if meta.tmdb_id.is_none() && tag == "id" {
                meta.tmdb_id = Some(text.to_string());
            }
        }
        "tmdbid" => meta.tmdb_id = Some(text.to_string()),
        "tvdbid" => meta.tvdb_id = Some(text.to_string()),
        "uniqueid" => {
            // Value only; the type attribute is handled by callers that need it.
            if text.starts_with("tt") && meta.imdb_id.is_none() {
                meta.imdb_id = Some(text.to_string());
            }
        }
        _ => {}
    }
}

/// Resolve the five predefined XML entities plus numeric character refs.
fn resolve_entity(name: &str) -> Option<char> {
    match name {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        other => {
            let digits = other.strip_prefix('#')?;
            let code = match digits.strip_prefix(['x', 'X']) {
                Some(hex) => u32::from_str_radix(hex, 16).ok()?,
                None => digits.parse().ok()?,
            };
            char::from_u32(code)
        }
    }
}

fn push_unique(list: &mut Vec<String>, value: &str) {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return;
    }
    // NFOs often repeat a genre across <genre> and a comma list.
    for part in trimmed.split(" / ") {
        let part = part.trim();
        if !part.is_empty() && !list.iter().any(|v| v == part) {
            list.push(part.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_movie_nfo() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<movie>
  <title>Blade Runner</title>
  <originaltitle>Blade Runner</originaltitle>
  <year>1982</year>
  <premiered>1982-06-25</premiered>
  <tagline>Man has made his match... now it's his problem.</tagline>
  <plot>A blade runner must pursue replicants.</plot>
  <runtime>117</runtime>
  <mpaa>R</mpaa>
  <genre>Science Fiction</genre>
  <genre>Thriller</genre>
  <studio>Warner Bros.</studio>
  <country>USA</country>
  <director>Ridley Scott</director>
  <credits>Hampton Fancher</credits>
  <id>tt0083658</id>
  <tmdbid>78</tmdbid>
  <rating>8.1</rating>
  <votes>12,345</votes>
  <actor>
    <name>Harrison Ford</name>
    <role>Rick Deckard</role>
  </actor>
  <actor>
    <name>Rutger Hauer</name>
    <role>Roy Batty</role>
  </actor>
</movie>"#;

        let m = parse_nfo(xml).unwrap();

        assert_eq!(m.kind, VideoKind::Movie);
        assert_eq!(m.title.as_deref(), Some("Blade Runner"));
        assert_eq!(m.year, Some(1982));
        assert_eq!(m.runtime_minutes, Some(117));
        assert_eq!(m.certification.as_deref(), Some("R"));
        assert_eq!(m.genres, vec!["Science Fiction", "Thriller"]);
        assert_eq!(m.directors, vec!["Ridley Scott"]);
        assert_eq!(m.writers, vec!["Hampton Fancher"]);
        assert_eq!(m.imdb_id.as_deref(), Some("tt0083658"));
        assert_eq!(m.tmdb_id.as_deref(), Some("78"));
        assert_eq!(m.rating, Some(8.1));
        assert_eq!(m.votes, Some(12345));
        assert_eq!(m.actors.len(), 2);
        assert_eq!(m.actors[0].name, "Harrison Ford");
        assert_eq!(m.actors[0].role.as_deref(), Some("Rick Deckard"));
        assert_eq!(m.source, VideoMetadataSource::Nfo);
    }

    #[test]
    fn parses_an_episode_nfo() {
        let xml = r#"<episodedetails>
  <title>Ozymandias</title>
  <showtitle>Breaking Bad</showtitle>
  <season>5</season>
  <episode>14</episode>
  <aired>2013-09-15</aired>
  <plot>Everything comes apart.</plot>
</episodedetails>"#;

        let m = parse_nfo(xml).unwrap();

        assert_eq!(m.kind, VideoKind::Episode);
        assert_eq!(m.show_title.as_deref(), Some("Breaking Bad"));
        assert_eq!(m.season, Some(5));
        assert_eq!(m.episode, Some(14));
        assert_eq!(m.aired.as_deref(), Some("2013-09-15"));
    }

    #[test]
    fn reads_the_nested_rating_form() {
        let xml = r#"<movie>
  <title>X</title>
  <ratings><rating name="themoviedb"><value>7.4</value><votes>900</votes></rating></ratings>
</movie>"#;

        let m = parse_nfo(xml).unwrap();
        assert_eq!(m.rating, Some(7.4));
    }

    #[test]
    fn rejects_documents_that_are_not_nfos() {
        assert!(parse_nfo("<html><body>hi</body></html>").is_err());
    }

    #[test]
    fn reports_malformed_xml() {
        assert!(parse_nfo("<movie><title>Unclosed</movie>").is_err());
    }

    #[test]
    fn does_not_duplicate_repeated_genres() {
        let xml = r#"<movie><title>X</title><genre>Drama</genre><genre>Drama</genre>
        <genre>Crime / Drama</genre></movie>"#;
        let m = parse_nfo(xml).unwrap();
        assert_eq!(m.genres, vec!["Drama", "Crime"]);
    }
}
