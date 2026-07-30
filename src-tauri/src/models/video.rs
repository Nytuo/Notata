use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VideoResultType {
    Movie,
    Series,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoSearchResult {
    pub provider: String,
    pub result_type: VideoResultType,
    pub id: String,
    pub title: String,
    pub year: Option<i32>,
    pub overview: Option<String>,
    pub poster_url: Option<String>,
    pub rating: Option<f32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MovieMetadata {
    pub title: String,
    pub original_title: Option<String>,
    pub year: Option<i32>,
    pub release_date: Option<String>,
    pub tagline: Option<String>,
    pub overview: Option<String>,
    pub runtime_minutes: Option<u32>,
    pub genres: Vec<String>,
    pub rating: Option<f32>,
    pub country: Option<String>,
    pub studios: Vec<String>,
    pub directors: Vec<String>,
    pub writers: Vec<String>,
    pub cast: Vec<String>,
    pub tmdb_id: Option<String>,
    pub imdb_id: Option<String>,
    pub poster_url: Option<String>,
    pub backdrop_url: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeriesMetadata {
    pub title: String,
    pub year: Option<i32>,
    pub first_aired: Option<String>,
    pub overview: Option<String>,
    pub genres: Vec<String>,
    pub rating: Option<f32>,
    pub network: Option<String>,
    pub status: Option<String>,
    pub total_seasons: Option<u32>,
    pub tmdb_id: Option<String>,
    pub tvdb_id: Option<String>,
    pub imdb_id: Option<String>,
    pub poster_url: Option<String>,
    pub backdrop_url: Option<String>,
    pub episodes: Vec<EpisodeMetadata>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EpisodeMetadata {
    pub title: String,
    pub season: u32,
    pub episode: u32,
    pub air_date: Option<String>,
    pub overview: Option<String>,
    pub rating: Option<f32>,
    pub runtime_minutes: Option<u32>,
    pub still_url: Option<String>,
    pub tmdb_id: Option<String>,
    pub tvdb_id: Option<String>,
}

/// Whether a video file is a standalone movie or an episode of a series.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VideoKind {
    Movie,
    Episode,
}

impl Default for VideoKind {
    fn default() -> Self {
        Self::Movie
    }
}

/// Where a file's metadata was loaded from.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VideoMetadataSource {
    Nfo,
    Embedded,
    /// Nothing found; fields were derived from the filename.
    Filename,
    None,
}

/// A single actor credit, kept structured so NFO round-trips keep roles.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActorCredit {
    pub name: String,
    pub role: Option<String>,
    pub thumb: Option<String>,
}

/// The editable metadata for one video file.
///
/// Mirrors what Kodi/Jellyfin/Plex store in NFO sidecars, which is the
/// interchange format those servers actually read.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoMetadata {
    #[serde(default)]
    pub kind: VideoKind,

    pub title: Option<String>,
    pub original_title: Option<String>,
    pub sort_title: Option<String>,
    pub year: Option<i32>,
    pub release_date: Option<String>,
    pub tagline: Option<String>,
    pub plot: Option<String>,
    pub outline: Option<String>,
    pub runtime_minutes: Option<u32>,
    pub rating: Option<f32>,
    pub votes: Option<u32>,
    /// Age certification, e.g. "PG-13".
    pub certification: Option<String>,

    #[serde(default)]
    pub genres: Vec<String>,
    #[serde(default)]
    pub studios: Vec<String>,
    #[serde(default)]
    pub countries: Vec<String>,
    #[serde(default)]
    pub directors: Vec<String>,
    #[serde(default)]
    pub writers: Vec<String>,
    #[serde(default)]
    pub actors: Vec<ActorCredit>,
    #[serde(default)]
    pub tags: Vec<String>,

    // Episode-specific.
    pub show_title: Option<String>,
    pub season: Option<u32>,
    pub episode: Option<u32>,
    pub aired: Option<String>,

    pub imdb_id: Option<String>,
    pub tmdb_id: Option<String>,
    pub tvdb_id: Option<String>,

    pub trailer: Option<String>,

    /// Where these values came from. Not written back to disk.
    #[serde(default = "default_source")]
    pub source: VideoMetadataSource,
    /// Path of the NFO backing this file, when one exists.
    pub nfo_path: Option<String>,
}

fn default_source() -> VideoMetadataSource {
    VideoMetadataSource::None
}

impl Default for VideoMetadataSource {
    fn default() -> Self {
        Self::None
    }
}

/// Container-level technical details, the video analogue of `AudioProperties`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoProperties {
    pub duration_ms: Option<u64>,
    pub container: String,
    pub file_size: u64,
    pub overall_bitrate_kbps: Option<u32>,
}

/// A candidate image offered by a provider during poster rematch.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteArtwork {
    pub provider: String,
    /// "poster" or "backdrop".
    pub art_type: String,
    /// Full-size image, saved when the user picks it.
    pub url: String,
    /// Smaller variant used for the grid.
    pub thumb_url: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    /// ISO-639-1 code, when the provider labels the image by language.
    pub language: Option<String>,
    pub rating: Option<f32>,
}

/// A poster or fanart image found next to a video file.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoArtwork {
    /// "poster", "fanart", "banner", "thumb"
    pub art_type: String,
    pub path: String,
    pub data: String,
    pub mime_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoProviderInfo {
    pub id: String,
    pub display_name: String,
    pub requires_api_key: bool,
    pub configured: bool,
}
