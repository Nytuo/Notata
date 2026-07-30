use std::path::Path;

use crate::error::{NotataError, Result};
use crate::models::video::{
    VideoArtwork, VideoKind, VideoMetadata, VideoMetadataSource, VideoProperties,
};
use crate::nfo;

/// Read a video file's metadata, preferring the richest source available.
///
/// NFO sidecars win because that is what the media servers actually read and
/// what other taggers write. Embedded container tags are the fallback, and a
/// filename parse is the last resort so the editor is never completely blank.
pub fn read_video_metadata(path: &str) -> Result<VideoMetadata> {
    let video_path = Path::new(path);

    if let Some(nfo_path) = nfo::find_nfo_path(video_path) {
        match std::fs::read_to_string(&nfo_path) {
            Ok(xml) => match nfo::parser::parse_nfo(&xml) {
                Ok(mut meta) => {
                    meta.nfo_path = Some(nfo_path.to_string_lossy().to_string());
                    return Ok(meta);
                }
                Err(e) => log::warn!("Ignoring unreadable NFO at {:?}: {}", nfo_path, e),
            },
            Err(e) => log::warn!("Could not read NFO at {:?}: {}", nfo_path, e),
        }
    }

    if let Some(meta) = read_embedded(path) {
        return Ok(meta);
    }

    Ok(from_filename(video_path))
}

/// Read tags stored inside the container itself (MP4 `ilst`, Matroska tags).
///
/// Many containers carry no tags at all, and some that lofty cannot open are
/// still perfectly valid video — so every failure here is soft.
fn read_embedded(path: &str) -> Option<VideoMetadata> {
    use lofty::prelude::*;
    use lofty::probe::Probe;

    let tagged = Probe::open(path).ok()?.read().ok()?;
    let tag = tagged.primary_tag().or_else(|| tagged.first_tag())?;

    let title = tag.title().map(|s| s.to_string());
    let plot = tag.comment().map(|s| s.to_string());
    let year = tag.year().map(|y| y as i32);
    let genres: Vec<String> = tag
        .genre()
        .map(|g| {
            g.split([';', '/'])
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();

    // A container with no title and no year tells us nothing the filename
    // parse would not do better.
    if title.is_none() && year.is_none() && genres.is_empty() {
        return None;
    }

    Some(VideoMetadata {
        kind: VideoKind::Movie,
        title,
        year,
        plot,
        genres,
        source: VideoMetadataSource::Embedded,
        ..Default::default()
    })
}

/// Derive what we can from the filename: `S01E02` markers and a trailing year.
fn from_filename(path: &Path) -> VideoMetadata {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_string();

    let (season, episode) = parse_season_episode(&stem);
    let year = parse_year(&stem);

    let kind = if season.is_some() {
        VideoKind::Episode
    } else {
        VideoKind::Movie
    };

    VideoMetadata {
        kind,
        title: Some(clean_title(&stem)),
        year,
        season,
        episode,
        source: VideoMetadataSource::Filename,
        ..Default::default()
    }
}

/// Find an `S01E02` / `1x02` style marker.
fn parse_season_episode(name: &str) -> (Option<u32>, Option<u32>) {
    let lower = name.to_lowercase();
    let chars: Vec<char> = lower.chars().collect();

    // SxxExx
    for i in 0..chars.len() {
        if chars[i] != 's' {
            continue;
        }
        let (season, next) = take_number(&chars, i + 1);
        let Some(season) = season else { continue };
        if next < chars.len() && chars[next] == 'e' {
            let (episode, _) = take_number(&chars, next + 1);
            if let Some(episode) = episode {
                return (Some(season), Some(episode));
            }
        }
    }

    // 1x02
    for i in 0..chars.len() {
        if chars[i] != 'x' || i == 0 {
            continue;
        }
        let start = chars[..i]
            .iter()
            .rposition(|c| !c.is_ascii_digit())
            .map(|p| p + 1)
            .unwrap_or(0);
        if start == i {
            continue;
        }
        let season: Option<u32> = lower[start..i].parse().ok();
        let (episode, _) = take_number(&chars, i + 1);
        if let (Some(s), Some(e)) = (season, episode) {
            return (Some(s), Some(e));
        }
    }

    (None, None)
}

fn take_number(chars: &[char], from: usize) -> (Option<u32>, usize) {
    let mut end = from;
    while end < chars.len() && chars[end].is_ascii_digit() {
        end += 1;
    }
    if end == from {
        return (None, from);
    }
    let value: String = chars[from..end].iter().collect();
    (value.parse().ok(), end)
}

/// A four-digit year in a plausible range, taken from the end so a title like
/// "2012" is not mistaken for the release year of "2012 (2009)".
fn parse_year(name: &str) -> Option<i32> {
    let chars: Vec<char> = name.chars().collect();
    let mut found = None;

    for i in 0..chars.len().saturating_sub(3) {
        if !chars[i].is_ascii_digit() {
            continue;
        }
        let slice: String = chars[i..i + 4].iter().collect();
        if slice.len() == 4 && slice.chars().all(|c| c.is_ascii_digit()) {
            if let Ok(year) = slice.parse::<i32>() {
                if (1900..=2100).contains(&year) {
                    found = Some(year);
                }
            }
        }
    }

    found
}

/// Turn separators into spaces and drop the release-group noise after the year.
fn clean_title(stem: &str) -> String {
    let replaced: String = stem
        .chars()
        .map(|c| if c == '.' || c == '_' { ' ' } else { c })
        .collect();

    let cleaned = replaced.split_whitespace().collect::<Vec<_>>().join(" ");

    // Cut at the year marker when present — everything after is usually
    // resolution, codec, and group tags.
    if let Some(year) = parse_year(&cleaned) {
        for marker in [format!("({})", year), year.to_string()] {
            if let Some(idx) = cleaned.find(&marker) {
                let title = cleaned[..idx].trim().trim_end_matches(['-', '(']).trim();
                if !title.is_empty() {
                    return title.to_string();
                }
            }
        }
    }

    cleaned
}

pub fn read_video_properties(path: &str) -> Result<VideoProperties> {
    let metadata = std::fs::metadata(path)?;
    let container = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("unknown")
        .to_uppercase();

    // Duration is only available when lofty can parse the container.
    let duration_ms = lofty::probe::Probe::open(path)
        .ok()
        .and_then(|p| p.read().ok())
        .map(|f| {
            use lofty::file::AudioFile;
            f.properties().duration().as_millis() as u64
        })
        .filter(|d| *d > 0);

    Ok(VideoProperties {
        duration_ms,
        container,
        file_size: metadata.len(),
        overall_bitrate_kbps: None,
    })
}

/// Artwork filenames the media servers look for, in priority order.
const POSTER_NAMES: &[&str] = &["poster", "folder", "cover", "default"];
const FANART_NAMES: &[&str] = &["fanart", "backdrop"];
const IMAGE_EXTS: &[&str] = &["jpg", "jpeg", "png", "webp"];

/// Find poster and fanart images sitting next to a video file.
///
/// Checks both the per-file form (`Movie (2001)-poster.jpg`) and the
/// folder-level form (`poster.jpg`).
pub fn find_local_artwork(path: &str) -> Vec<VideoArtwork> {
    let video_path = Path::new(path);
    let Some(parent) = video_path.parent() else {
        return Vec::new();
    };
    let stem = video_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default();

    let mut found = Vec::new();

    for (art_type, names) in [("poster", POSTER_NAMES), ("fanart", FANART_NAMES)] {
        let mut candidates: Vec<std::path::PathBuf> = Vec::new();

        for name in names {
            for ext in IMAGE_EXTS {
                candidates.push(parent.join(format!("{}-{}.{}", stem, name, ext)));
                candidates.push(parent.join(format!("{}.{}", name, ext)));
            }
        }

        if let Some(hit) = candidates.into_iter().find(|p| p.is_file()) {
            if let Ok(bytes) = std::fs::read(&hit) {
                let mime = match hit.extension().and_then(|e| e.to_str()) {
                    Some("png") => "image/png",
                    Some("webp") => "image/webp",
                    _ => "image/jpeg",
                };
                found.push(VideoArtwork {
                    art_type: art_type.to_string(),
                    path: hit.to_string_lossy().to_string(),
                    data: base64::Engine::encode(
                        &base64::engine::general_purpose::STANDARD,
                        &bytes,
                    ),
                    mime_type: mime.to_string(),
                });
            }
        }
    }

    found
}

/// Write metadata back to the NFO sidecar.
///
/// Returns the path written. Embedded container tags are deliberately not
/// touched: rewriting an MKV or MP4 in place risks the media file itself, and
/// every target media server reads the NFO.
pub fn write_video_metadata(path: &str, meta: &VideoMetadata) -> Result<String> {
    let video_path = Path::new(path);

    let target = match &meta.nfo_path {
        Some(existing) => std::path::PathBuf::from(existing),
        None => nfo::default_nfo_path(video_path),
    };

    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let xml = nfo::writer::to_nfo_xml(meta);
    std::fs::write(&target, xml)
        .map_err(|e| NotataError::Custom(format!("Could not write {:?}: {}", target, e)))?;

    Ok(target.to_string_lossy().to_string())
}

/// Save a poster fetched from a provider next to the video file.
pub fn write_poster(path: &str, image_data: &[u8], mime_type: &str) -> Result<String> {
    let video_path = Path::new(path);
    let parent = video_path
        .parent()
        .ok_or_else(|| NotataError::Custom("Video file has no parent directory".into()))?;
    let stem = video_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| NotataError::Custom("Video file has no name".into()))?;

    let ext = match mime_type {
        "image/png" => "png",
        "image/webp" => "webp",
        _ => "jpg",
    };

    let target = parent.join(format!("{}-poster.{}", stem, ext));
    std::fs::write(&target, image_data)?;

    Ok(target.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_season_and_episode_from_common_patterns() {
        assert_eq!(
            parse_season_episode("Breaking.Bad.S05E14.1080p"),
            (Some(5), Some(14))
        );
        assert_eq!(parse_season_episode("Show 1x02 title"), (Some(1), Some(2)));
        assert_eq!(parse_season_episode("s01e01"), (Some(1), Some(1)));
        assert_eq!(parse_season_episode("A Movie 2019"), (None, None));
    }

    #[test]
    fn takes_a_plausible_year() {
        assert_eq!(parse_year("Blade Runner (1982)"), Some(1982));
        assert_eq!(parse_year("Movie.2019.1080p"), Some(2019));
        assert_eq!(parse_year("No year here"), None);
        // 9999 is out of range and must not be mistaken for a year.
        assert_eq!(parse_year("Thing 9999"), None);
    }

    #[test]
    fn cleans_release_noise_off_the_title() {
        assert_eq!(clean_title("Blade.Runner.1982.1080p.BluRay"), "Blade Runner");
        assert_eq!(clean_title("Blade Runner (1982)"), "Blade Runner");
        assert_eq!(clean_title("Some_Movie_Name"), "Some Movie Name");
    }

    #[test]
    fn filename_fallback_classifies_episodes() {
        let meta = from_filename(Path::new("/tv/Breaking.Bad.S05E14.mkv"));
        assert_eq!(meta.kind, VideoKind::Episode);
        assert_eq!(meta.season, Some(5));
        assert_eq!(meta.episode, Some(14));
        assert_eq!(meta.source, VideoMetadataSource::Filename);
    }

    #[test]
    fn filename_fallback_classifies_movies() {
        let meta = from_filename(Path::new("/movies/Blade.Runner.1982.mkv"));
        assert_eq!(meta.kind, VideoKind::Movie);
        assert_eq!(meta.title.as_deref(), Some("Blade Runner"));
        assert_eq!(meta.year, Some(1982));
    }
}
