use std::collections::HashMap;
use std::path::Path;

use crate::models::media_file::MediaType;
use crate::models::video::VideoKind;
use crate::scanner::mime::classify_extension;

/// Collect the template values for one file, choosing the metadata source
/// from the file's own type.
///
/// Reading music tags from a video file yields nothing, which is what made
/// video renames collapse to their template's punctuation.
pub fn fields_for_path(path: &str) -> HashMap<String, String> {
    let source = Path::new(path);
    let media_type = source
        .extension()
        .and_then(|e| e.to_str())
        .map(|ext| classify_extension(ext).0)
        .unwrap_or(MediaType::Unknown);

    let mut values = match media_type {
        MediaType::Video => video_fields(path),
        MediaType::Book | MediaType::Comic => book_fields(path),
        _ => music_fields(path),
    };

    // Always available, regardless of what the metadata reader found.
    put(
        &mut values,
        "filename",
        source
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string()),
    );
    put(
        &mut values,
        "ext",
        source
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_lowercase()),
    );

    values
}

fn put(values: &mut HashMap<String, String>, key: &str, value: Option<String>) {
    if let Some(v) = value {
        if !v.trim().is_empty() {
            values.insert(key.to_string(), v);
        }
    }
}

fn music_fields(path: &str) -> HashMap<String, String> {
    let metadata = crate::metadata::reader::read_metadata(path).unwrap_or_default();
    let mut v = HashMap::new();

    put(&mut v, "title", metadata.title.clone());
    put(&mut v, "artist", metadata.artist.clone());
    put(
        &mut v,
        "albumartist",
        metadata
            .album_artist
            .clone()
            .or_else(|| metadata.artist.clone()),
    );
    put(&mut v, "album", metadata.album.clone());
    put(&mut v, "track", metadata.track_number.map(|n| n.to_string()));
    put(
        &mut v,
        "totaltracks",
        metadata.total_tracks.map(|n| n.to_string()),
    );
    put(&mut v, "disc", metadata.disc_number.map(|n| n.to_string()));
    put(
        &mut v,
        "totaldiscs",
        metadata.total_discs.map(|n| n.to_string()),
    );
    put(&mut v, "year", metadata.year.map(|y| y.to_string()));
    put(&mut v, "date", metadata.date.clone());
    put(&mut v, "isrc", metadata.isrc.clone());
    put(
        &mut v,
        "genre",
        metadata.genre.as_ref().and_then(|g| g.first().cloned()),
    );
    put(
        &mut v,
        "composer",
        metadata.composer.as_ref().and_then(|c| c.first().cloned()),
    );
    put(
        &mut v,
        "musicbrainzreleaseid",
        metadata.musicbrainz_release_id.clone(),
    );
    put(
        &mut v,
        "musicbrainztrackid",
        metadata.musicbrainz_track_id.clone(),
    );

    v
}

fn video_fields(path: &str) -> HashMap<String, String> {
    let metadata = crate::metadata::video::read_video_metadata(path).unwrap_or_default();
    let mut v = HashMap::new();

    put(&mut v, "title", metadata.title.clone());
    put(&mut v, "episodetitle", metadata.title.clone());
    put(&mut v, "originaltitle", metadata.original_title.clone());
    put(&mut v, "year", metadata.year.map(|y| y.to_string()));
    put(&mut v, "date", metadata.release_date.clone());
    put(&mut v, "aired", metadata.aired.clone());
    put(
        &mut v,
        "runtime",
        metadata.runtime_minutes.map(|r| r.to_string()),
    );
    put(&mut v, "certification", metadata.certification.clone());
    put(&mut v, "genre", metadata.genres.first().cloned());
    put(&mut v, "studio", metadata.studios.first().cloned());
    put(&mut v, "director", metadata.directors.first().cloned());
    put(&mut v, "imdbid", metadata.imdb_id.clone());
    put(&mut v, "tmdbid", metadata.tmdb_id.clone());
    put(&mut v, "tvdbid", metadata.tvdb_id.clone());

    if metadata.kind == VideoKind::Episode {
        put(&mut v, "seriestitle", metadata.show_title.clone());
        put(&mut v, "season", metadata.season.map(|s| s.to_string()));
        put(&mut v, "episode", metadata.episode.map(|e| e.to_string()));
        // Series templates address the show through {seriestitle}; keep
        // {title} pointing at the episode.
    } else {
        put(&mut v, "seriestitle", metadata.title.clone());
    }

    v
}

fn book_fields(path: &str) -> HashMap<String, String> {
    let metadata = crate::metadata::book::read_book_metadata(path).unwrap_or_default();
    let mut v = HashMap::new();

    put(&mut v, "title", metadata.title.clone());
    put(&mut v, "series", metadata.series.clone());
    put(&mut v, "number", metadata.number.clone());
    put(&mut v, "volume", metadata.volume.map(|n| n.to_string()));
    put(&mut v, "author", metadata.authors.first().cloned());
    put(&mut v, "writer", metadata.authors.first().cloned());
    put(&mut v, "publisher", metadata.publisher.clone());
    put(&mut v, "year", metadata.year.map(|y| y.to_string()));
    put(&mut v, "language", metadata.language.clone());
    put(&mut v, "isbn", metadata.isbn.clone());
    put(&mut v, "genre", metadata.genres.first().cloned());

    v
}
