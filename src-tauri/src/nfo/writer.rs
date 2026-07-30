use crate::models::video::{VideoKind, VideoMetadata};

/// Serialize metadata as a Kodi-style NFO document.
///
/// Written by hand rather than via a serializer so the element order matches
/// what Kodi and Jellyfin emit, which keeps diffs readable when a user has
/// other tools writing the same files.
pub fn to_nfo_xml(meta: &VideoMetadata) -> String {
    let root = match meta.kind {
        VideoKind::Movie => "movie",
        VideoKind::Episode => "episodedetails",
    };

    let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n");
    out.push_str(&format!("<{}>\n", root));

    // A plain fn rather than a closure so the `<uniqueid>` blocks below can
    // also borrow `out`.
    fn push(out: &mut String, tag: &str, value: &str) {
        out.push_str(&format!("  <{0}>{1}</{0}>\n", tag, escape(value)));
    }
    macro_rules! push {
        ($tag:expr, $value:expr) => {
            push(&mut out, $tag, $value)
        };
    }

    if let Some(v) = &meta.title {
        push!("title", v);
    }
    if let Some(v) = &meta.original_title {
        push!("originaltitle", v);
    }
    if let Some(v) = &meta.sort_title {
        push!("sorttitle", v);
    }

    if meta.kind == VideoKind::Episode {
        if let Some(v) = &meta.show_title {
            push!("showtitle", v);
        }
        if let Some(v) = meta.season {
            push!("season", &v.to_string());
        }
        if let Some(v) = meta.episode {
            push!("episode", &v.to_string());
        }
        if let Some(v) = &meta.aired {
            push!("aired", v);
        }
    }

    if let Some(v) = meta.year {
        push!("year", &v.to_string());
    }
    if let Some(v) = &meta.release_date {
        push!("premiered", v);
    }
    if let Some(v) = &meta.tagline {
        push!("tagline", v);
    }
    if let Some(v) = &meta.plot {
        push!("plot", v);
    }
    if let Some(v) = &meta.outline {
        push!("outline", v);
    }
    if let Some(v) = meta.runtime_minutes {
        push!("runtime", &v.to_string());
    }
    if let Some(v) = meta.rating {
        push!("rating", &v.to_string());
    }
    if let Some(v) = meta.votes {
        push!("votes", &v.to_string());
    }
    if let Some(v) = &meta.certification {
        push!("mpaa", v);
    }

    for genre in &meta.genres {
        push!("genre", genre);
    }
    for studio in &meta.studios {
        push!("studio", studio);
    }
    for country in &meta.countries {
        push!("country", country);
    }
    for director in &meta.directors {
        push!("director", director);
    }
    for writer in &meta.writers {
        push!("credits", writer);
    }
    for tag in &meta.tags {
        push!("tag", tag);
    }

    if let Some(v) = &meta.trailer {
        push!("trailer", v);
    }

    // IMDb goes in <id> for Kodi compatibility, plus explicit <uniqueid>
    // entries that Jellyfin and newer Kodi versions prefer.
    if let Some(v) = &meta.imdb_id {
        push!("id", v);
        out.push_str(&format!(
            "  <uniqueid type=\"imdb\" default=\"true\">{}</uniqueid>\n",
            escape(v)
        ));
    }
    if let Some(v) = &meta.tmdb_id {
        push!("tmdbid", v);
        out.push_str(&format!(
            "  <uniqueid type=\"tmdb\">{}</uniqueid>\n",
            escape(v)
        ));
    }
    if let Some(v) = &meta.tvdb_id {
        push!("tvdbid", v);
        out.push_str(&format!(
            "  <uniqueid type=\"tvdb\">{}</uniqueid>\n",
            escape(v)
        ));
    }

    for actor in &meta.actors {
        out.push_str("  <actor>\n");
        out.push_str(&format!("    <name>{}</name>\n", escape(&actor.name)));
        if let Some(role) = &actor.role {
            out.push_str(&format!("    <role>{}</role>\n", escape(role)));
        }
        if let Some(thumb) = &actor.thumb {
            out.push_str(&format!("    <thumb>{}</thumb>\n", escape(thumb)));
        }
        out.push_str("  </actor>\n");
    }

    out.push_str(&format!("</{}>\n", root));
    out
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::video::ActorCredit;
    use crate::nfo::parser::parse_nfo;

    fn sample() -> VideoMetadata {
        VideoMetadata {
            kind: VideoKind::Movie,
            title: Some("Blade Runner".into()),
            year: Some(1982),
            plot: Some("A blade runner must pursue replicants.".into()),
            runtime_minutes: Some(117),
            certification: Some("R".into()),
            genres: vec!["Science Fiction".into(), "Thriller".into()],
            directors: vec!["Ridley Scott".into()],
            imdb_id: Some("tt0083658".into()),
            tmdb_id: Some("78".into()),
            rating: Some(8.1),
            actors: vec![ActorCredit {
                name: "Harrison Ford".into(),
                role: Some("Rick Deckard".into()),
                thumb: None,
            }],
            ..Default::default()
        }
    }

    #[test]
    fn round_trips_through_the_parser() {
        let original = sample();
        let parsed = parse_nfo(&to_nfo_xml(&original)).unwrap();

        assert_eq!(parsed.title, original.title);
        assert_eq!(parsed.year, original.year);
        assert_eq!(parsed.runtime_minutes, original.runtime_minutes);
        assert_eq!(parsed.certification, original.certification);
        assert_eq!(parsed.genres, original.genres);
        assert_eq!(parsed.directors, original.directors);
        assert_eq!(parsed.imdb_id, original.imdb_id);
        assert_eq!(parsed.tmdb_id, original.tmdb_id);
        assert_eq!(parsed.rating, original.rating);
        assert_eq!(parsed.actors.len(), 1);
        assert_eq!(parsed.actors[0].role.as_deref(), Some("Rick Deckard"));
    }

    #[test]
    fn round_trips_an_episode() {
        let episode = VideoMetadata {
            kind: VideoKind::Episode,
            title: Some("Ozymandias".into()),
            show_title: Some("Breaking Bad".into()),
            season: Some(5),
            episode: Some(14),
            aired: Some("2013-09-15".into()),
            ..Default::default()
        };

        let parsed = parse_nfo(&to_nfo_xml(&episode)).unwrap();

        assert_eq!(parsed.kind, VideoKind::Episode);
        assert_eq!(parsed.show_title.as_deref(), Some("Breaking Bad"));
        assert_eq!(parsed.season, Some(5));
        assert_eq!(parsed.episode, Some(14));
    }

    #[test]
    fn escapes_markup_in_values() {
        let meta = VideoMetadata {
            title: Some("Fish & <Chips>".into()),
            ..Default::default()
        };
        let xml = to_nfo_xml(&meta);

        assert!(xml.contains("Fish &amp; &lt;Chips&gt;"));
        // And it must survive a parse back.
        assert_eq!(
            parse_nfo(&xml).unwrap().title.as_deref(),
            Some("Fish & <Chips>")
        );
    }

    #[test]
    fn omits_fields_that_are_unset() {
        let meta = VideoMetadata {
            title: Some("X".into()),
            ..Default::default()
        };
        let xml = to_nfo_xml(&meta);

        assert!(!xml.contains("<tagline>"));
        assert!(!xml.contains("<runtime>"));
        assert!(!xml.contains("<actor>"));
    }
}
