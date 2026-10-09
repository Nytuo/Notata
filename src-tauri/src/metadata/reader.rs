use std::path::Path;

use lofty::prelude::*;
use lofty::probe::Probe;
use lofty::tag::{Accessor, ItemKey};

use crate::error::Result;
use crate::models::album::CoverArtData;
use crate::models::track::{AudioProperties, TrackMetadata};

pub fn read_metadata(path: &str) -> Result<TrackMetadata> {
    let tagged_file = Probe::open(path)?.read()?;

    let tag = match tagged_file
        .primary_tag()
        .or_else(|| tagged_file.first_tag())
    {
        Some(t) => t,
        None => return Ok(TrackMetadata::default()),
    };

    let genre = tag.genre().map(|g| {
        g.split(';')
            .flat_map(|s| s.split(','))
            .flat_map(|s| s.split('/'))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
    });

    let composer = get_items(tag, &ItemKey::Composer);

    let mut custom_tags = std::collections::HashMap::new();
    for item in tag.items() {
        let key_str = format!("{:?}", item.key());
        if is_standard_key(item.key()) {
            continue;
        }
        if let Some(val) = item.value().text() {
            custom_tags
                .entry(key_str)
                .or_insert_with(Vec::new)
                .push(val.to_string());
        }
    }

    Ok(TrackMetadata {
        title: tag.title().map(|s| s.to_string()),
        artist: tag.artist().map(|s| s.to_string()),
        album_artist: get_first_item(tag, &ItemKey::AlbumArtist),
        album: tag.album().map(|s| s.to_string()),
        track_number: tag.track(),
        total_tracks: tag.track_total(),
        disc_number: tag.disk(),
        total_discs: tag.disk_total(),
        year: tag.year().map(|y| y as i32),
        date: get_first_item(tag, &ItemKey::RecordingDate),
        genre,
        composer: if composer.is_empty() {
            None
        } else {
            Some(composer)
        },
        comment: tag.comment().map(|s| s.to_string()),
        lyrics: get_first_item(tag, &ItemKey::Lyrics),
        isrc: get_first_item(tag, &ItemKey::Isrc),
        musicbrainz_track_id: get_first_item(tag, &ItemKey::MusicBrainzRecordingId),
        musicbrainz_release_id: get_first_item(tag, &ItemKey::MusicBrainzReleaseId),
        musicbrainz_artist_id: get_first_item(tag, &ItemKey::MusicBrainzArtistId),
        musicbrainz_release_group_id: get_first_item(tag, &ItemKey::MusicBrainzReleaseGroupId),
        custom_tags,
    })
}

pub fn read_audio_properties(path: &str) -> Result<AudioProperties> {
    let tagged_file = Probe::open(path)?.read()?;
    let properties = tagged_file.properties();

    let format = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("unknown")
        .to_uppercase();

    Ok(AudioProperties {
        duration_ms: properties.duration().as_millis() as u64,
        bitrate_kbps: properties.audio_bitrate().unwrap_or(0),
        sample_rate_hz: properties.sample_rate().unwrap_or(0),
        channels: properties.channels().unwrap_or(0) as u8,
        bits_per_sample: properties.bit_depth().map(|b| b as u8),
        format,
    })
}

#[derive(Debug, Clone, Default)]
pub struct AudioAnalysis {
    pub properties: Option<AudioProperties>,
    pub has_cover_art: bool,
    pub issues: Vec<String>,
}

pub mod issue {
    pub const UNREADABLE: &str = "unreadable";
    pub const NO_TAGS: &str = "no_tags";
    pub const NO_DURATION: &str = "no_duration";
    pub const MISSING_TITLE: &str = "missing_title";
    pub const MISSING_ARTIST: &str = "missing_artist";
    pub const MISSING_ALBUM: &str = "missing_album";
    pub const MISSING_TRACK: &str = "missing_track";
    pub const MISSING_COVER: &str = "missing_cover";
}

fn is_blank(value: Option<&str>) -> bool {
    value.map_or(true, |v| v.trim().is_empty())
}

/// Open the file once and report its properties, cover presence, and the
/// metadata gaps that break media-server matching or playback.
pub fn analyze_audio_file(path: &str) -> AudioAnalysis {
    let tagged_file = match Probe::open(path).and_then(|p| p.read()) {
        Ok(f) => f,
        Err(_) => {
            return AudioAnalysis {
                properties: None,
                has_cover_art: false,
                issues: vec![issue::UNREADABLE.to_string()],
            }
        }
    };

    let mut issues = Vec::new();

    let props = tagged_file.properties();
    let properties = AudioProperties {
        duration_ms: props.duration().as_millis() as u64,
        bitrate_kbps: props.audio_bitrate().unwrap_or(0),
        sample_rate_hz: props.sample_rate().unwrap_or(0),
        channels: props.channels().unwrap_or(0) as u8,
        bits_per_sample: props.bit_depth().map(|b| b as u8),
        format: Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("unknown")
            .to_uppercase(),
    };

    if properties.duration_ms == 0 {
        issues.push(issue::NO_DURATION.to_string());
    }

    let tag = tagged_file
        .primary_tag()
        .or_else(|| tagged_file.first_tag());
    let has_cover_art = tag.is_some_and(|t| !t.pictures().is_empty());

    match tag {
        None => issues.push(issue::NO_TAGS.to_string()),
        Some(t) => {
            if is_blank(t.title().as_deref()) {
                issues.push(issue::MISSING_TITLE.to_string());
            }
            if is_blank(t.artist().as_deref())
                && is_blank(get_first_item(t, &ItemKey::AlbumArtist).as_deref())
            {
                issues.push(issue::MISSING_ARTIST.to_string());
            }
            if is_blank(t.album().as_deref()) {
                issues.push(issue::MISSING_ALBUM.to_string());
            }
            if t.track().is_none() {
                issues.push(issue::MISSING_TRACK.to_string());
            }
        }
    }

    if !has_cover_art {
        issues.push(issue::MISSING_COVER.to_string());
    }

    AudioAnalysis {
        properties: Some(properties),
        has_cover_art,
        issues,
    }
}

pub fn read_embedded_cover_art(path: &str) -> Result<Option<CoverArtData>> {
    use crate::models::album::CoverArtType;

    let tagged_file = Probe::open(path)?.read()?;
    let tag = match tagged_file
        .primary_tag()
        .or_else(|| tagged_file.first_tag())
    {
        Some(t) => t,
        None => return Ok(None),
    };

    let pictures = tag.pictures();
    if pictures.is_empty() {
        return Ok(None);
    }

    let pic = &pictures[0];
    let data = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, pic.data());
    let mime_type = pic
        .mime_type()
        .map(|m| m.to_string())
        .unwrap_or_else(|| "image/jpeg".to_string());

    Ok(Some(CoverArtData {
        data,
        mime_type,
        art_type: CoverArtType::Front,
        width: None,
        height: None,
    }))
}

fn get_first_item(tag: &lofty::tag::Tag, key: &ItemKey) -> Option<String> {
    tag.get_string(key).map(|s| s.to_string())
}

fn get_items(tag: &lofty::tag::Tag, key: &ItemKey) -> Vec<String> {
    tag.get_strings(key).map(|s| s.to_string()).collect()
}

fn is_standard_key(key: &ItemKey) -> bool {
    matches!(
        key,
        ItemKey::TrackTitle
            | ItemKey::TrackArtist
            | ItemKey::AlbumArtist
            | ItemKey::AlbumTitle
            | ItemKey::TrackNumber
            | ItemKey::TrackTotal
            | ItemKey::DiscNumber
            | ItemKey::DiscTotal
            | ItemKey::Year
            | ItemKey::RecordingDate
            | ItemKey::Genre
            | ItemKey::Composer
            | ItemKey::Comment
            | ItemKey::Lyrics
            | ItemKey::Isrc
            | ItemKey::MusicBrainzRecordingId
            | ItemKey::MusicBrainzReleaseId
            | ItemKey::MusicBrainzArtistId
            | ItemKey::MusicBrainzReleaseGroupId
    )
}

#[cfg(test)]
mod analysis_tests {
    use super::*;

    #[test]
    fn unreadable_file_is_reported_not_panicked() {
        let analysis = analyze_audio_file("/definitely/not/a/real/file.mp3");

        assert_eq!(analysis.issues, vec![issue::UNREADABLE.to_string()]);
        assert!(analysis.properties.is_none());
        assert!(!analysis.has_cover_art);
    }

    #[test]
    fn a_non_audio_file_with_an_audio_extension_is_unreadable() {
        let dir = std::env::temp_dir().join("notata-analysis-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("fake.flac");
        std::fs::write(&path, b"not audio").unwrap();

        let analysis = analyze_audio_file(path.to_str().unwrap());
        assert!(analysis.issues.contains(&issue::UNREADABLE.to_string()));

        let _ = std::fs::remove_file(path);
    }
}
