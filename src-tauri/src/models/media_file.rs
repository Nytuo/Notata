use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaType {
    Audio,
    Video,
    Comic,
    Book,
    Image,
    Cue,
    Nfo,
    Unknown,
}

impl MediaType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Audio => "audio",
            Self::Video => "video",
            Self::Comic => "comic",
            Self::Book => "book",
            Self::Image => "image",
            Self::Cue => "cue",
            Self::Nfo => "nfo",
            Self::Unknown => "unknown",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "audio" => Self::Audio,
            "video" => Self::Video,
            "comic" => Self::Comic,
            "book" => Self::Book,
            "image" => Self::Image,
            "cue" => Self::Cue,
            "nfo" => Self::Nfo,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioFormat {
    Mp3,
    Flac,
    Ogg,
    Opus,
    Aac,
    Mp4a,
    Wma,
    Ape,
    Wav,
    Aiff,
    Dsf,
    WavPack,
    Unknown(String),
}

impl AudioFormat {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Mp3 => "mp3",
            Self::Flac => "flac",
            Self::Ogg => "ogg",
            Self::Opus => "opus",
            Self::Aac => "aac",
            Self::Mp4a => "mp4a",
            Self::Wma => "wma",
            Self::Ape => "ape",
            Self::Wav => "wav",
            Self::Aiff => "aiff",
            Self::Dsf => "dsf",
            Self::WavPack => "wavpack",
            Self::Unknown(s) => s.as_str(),
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "mp3" => Self::Mp3,
            "flac" => Self::Flac,
            "ogg" => Self::Ogg,
            "opus" => Self::Opus,
            "aac" => Self::Aac,
            "mp4a" => Self::Mp4a,
            "wma" => Self::Wma,
            "ape" => Self::Ape,
            "wav" => Self::Wav,
            "aiff" => Self::Aiff,
            "dsf" => Self::Dsf,
            "wavpack" => Self::WavPack,
            other => Self::Unknown(other.to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaFile {
    pub id: String,
    pub path: String,
    pub file_name: String,
    pub parent_dir: String,
    pub media_type: MediaType,
    pub audio_format: Option<AudioFormat>,
    pub file_size: u64,
    pub modified_at: i64,
    pub scanned_at: i64,
    pub has_cover_art: bool,
    pub duration_ms: Option<u64>,
    pub bitrate_kbps: Option<u32>,
    pub sample_rate_hz: Option<u32>,
    pub channels: Option<u8>,
    /// When this path was first indexed. Survives re-scans.
    #[serde(default)]
    pub first_seen_at: i64,
    /// Last time Notata itself wrote tags to this file.
    #[serde(default)]
    pub last_modified_by_app: Option<i64>,
    /// First seen during the most recent scan.
    #[serde(default)]
    pub is_new: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryNode {
    pub path: String,
    pub name: String,
    pub children: Vec<DirectoryNode>,
    pub file_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub total_files: usize,
    pub audio_files: usize,
    pub skipped: usize,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanProgress {
    pub scanned: usize,
    pub current_file: String,
}

/// What kind of media a library root holds. Drives which metadata workflow
/// the UI presents for files under it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaKind {
    Music,
    Movies,
    Series,
    Books,
}

impl MediaKind {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Music => "music",
            Self::Movies => "movies",
            Self::Series => "series",
            Self::Books => "books",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "movies" => Self::Movies,
            "series" => Self::Series,
            "books" => Self::Books,
            _ => Self::Music,
        }
    }

    pub fn is_video(&self) -> bool {
        matches!(self, Self::Movies | Self::Series)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryRoot {
    pub id: String,
    pub path: String,
    pub label: Option<String>,
    pub added_at: i64,
    pub last_scan: Option<i64>,
    #[serde(default)]
    pub previous_scan: Option<i64>,
    #[serde(default = "default_media_kind")]
    pub media_kind: MediaKind,
}

fn default_media_kind() -> MediaKind {
    MediaKind::Music
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryStats {
    pub total_files: usize,
    pub total_size: u64,
    pub roots: Vec<LibraryRoot>,
}
