use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscodeFormatInfo {
    pub id: String,
    pub label: String,
    pub extension: String,
    pub lossy: bool,
    pub codec_name: String,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TranscodeDestination {
    ReplaceInPlace,
    Folder { path: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscodeOptions {
    pub target_format: String,
    pub bitrate_kbps: Option<u32>,
    pub flac_compression: Option<u8>,
    pub prefer_stream_copy: bool,
    #[serde(default)]
    pub faststart: bool,
    pub destination: TranscodeDestination,
}
