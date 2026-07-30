use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TrackMetadata {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album_artist: Option<String>,
    pub album: Option<String>,
    pub track_number: Option<u32>,
    pub total_tracks: Option<u32>,
    pub disc_number: Option<u32>,
    pub total_discs: Option<u32>,
    pub year: Option<i32>,
    pub date: Option<String>,
    pub genre: Option<Vec<String>>,
    pub composer: Option<Vec<String>>,
    pub comment: Option<String>,
    pub lyrics: Option<String>,
    pub isrc: Option<String>,
    pub musicbrainz_track_id: Option<String>,
    pub musicbrainz_release_id: Option<String>,
    pub musicbrainz_artist_id: Option<String>,
    pub musicbrainz_release_group_id: Option<String>,
    #[serde(default)]
    pub custom_tags: HashMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioProperties {
    pub duration_ms: u64,
    pub bitrate_kbps: u32,
    pub sample_rate_hz: u32,
    pub channels: u8,
    pub bits_per_sample: Option<u8>,
    pub format: String,
}
