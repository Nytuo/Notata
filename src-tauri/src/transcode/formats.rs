use serde::{Deserialize, Serialize};

/// One entry in the catalog of formats Notata can transcode audio into.
///
/// `codec_name` is the ffmpeg/ffprobe codec identifier the format encodes
/// to — it doubles as the check for whether a source file can be
/// stream-copied into the target container without re-encoding.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscodeFormatInfo {
    pub id: String,
    pub label: String,
    pub extension: String,
    pub lossy: bool,
    pub codec_name: String,
    /// Default bitrate for lossy formats, in kbps. Ignored for lossless ones.
    pub default_bitrate_kbps: Option<u32>,
}

pub fn catalog() -> Vec<TranscodeFormatInfo> {
    vec![
        TranscodeFormatInfo {
            id: "mp3".into(),
            label: "MP3".into(),
            extension: "mp3".into(),
            lossy: true,
            codec_name: "mp3".into(),
            default_bitrate_kbps: Some(256),
        },
        TranscodeFormatInfo {
            id: "aac".into(),
            label: "AAC (.m4a)".into(),
            extension: "m4a".into(),
            lossy: true,
            codec_name: "aac".into(),
            default_bitrate_kbps: Some(256),
        },
        TranscodeFormatInfo {
            id: "ogg".into(),
            label: "Ogg Vorbis".into(),
            extension: "ogg".into(),
            lossy: true,
            codec_name: "vorbis".into(),
            default_bitrate_kbps: Some(256),
        },
        TranscodeFormatInfo {
            id: "opus".into(),
            label: "Opus".into(),
            extension: "opus".into(),
            lossy: true,
            codec_name: "opus".into(),
            default_bitrate_kbps: Some(160),
        },
        TranscodeFormatInfo {
            id: "wma".into(),
            label: "WMA".into(),
            extension: "wma".into(),
            lossy: true,
            codec_name: "wmav2".into(),
            default_bitrate_kbps: Some(192),
        },
        TranscodeFormatInfo {
            id: "flac".into(),
            label: "FLAC".into(),
            extension: "flac".into(),
            lossy: false,
            codec_name: "flac".into(),
            default_bitrate_kbps: None,
        },
        TranscodeFormatInfo {
            id: "alac".into(),
            label: "ALAC (.m4a)".into(),
            extension: "m4a".into(),
            lossy: false,
            codec_name: "alac".into(),
            default_bitrate_kbps: None,
        },
        TranscodeFormatInfo {
            id: "wav".into(),
            label: "WAV (PCM)".into(),
            extension: "wav".into(),
            lossy: false,
            codec_name: "pcm_s16le".into(),
            default_bitrate_kbps: None,
        },
        TranscodeFormatInfo {
            id: "aiff".into(),
            label: "AIFF (PCM)".into(),
            extension: "aiff".into(),
            lossy: false,
            codec_name: "pcm_s16be".into(),
            default_bitrate_kbps: None,
        },
    ]
}

pub fn find(id: &str) -> Option<TranscodeFormatInfo> {
    catalog().into_iter().find(|f| f.id == id)
}

/// Where the transcoded file should end up.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TranscodeDestination {
    /// Write next to the source, then remove the source once the new file
    /// is verified on disk.
    ReplaceInPlace,
    /// Write into a separate folder, leaving the source untouched.
    Folder { path: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscodeOptions {
    /// Catalog id, e.g. "mp3", "flac".
    pub target_format: String,
    /// Lossy formats only; falls back to the format's default when omitted.
    pub bitrate_kbps: Option<u32>,
    /// FLAC only, 0 (fastest) to 8 (smallest). Defaults to 5.
    pub flac_compression: Option<u8>,
    /// When the source audio codec already matches the target, remux with
    /// `-c:a copy` instead of re-encoding.
    pub prefer_stream_copy: bool,
    pub destination: TranscodeDestination,
}
