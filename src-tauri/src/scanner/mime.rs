use crate::models::media_file::{AudioFormat, MediaType};

pub fn classify_extension(ext: &str) -> (MediaType, Option<AudioFormat>) {
    match ext.to_lowercase().as_str() {
        "mp3" => (MediaType::Audio, Some(AudioFormat::Mp3)),
        "flac" => (MediaType::Audio, Some(AudioFormat::Flac)),
        "ogg" | "oga" => (MediaType::Audio, Some(AudioFormat::Ogg)),
        "opus" => (MediaType::Audio, Some(AudioFormat::Opus)),
        "aac" => (MediaType::Audio, Some(AudioFormat::Aac)),
        "m4a" | "m4b" | "m4p" => (MediaType::Audio, Some(AudioFormat::Mp4a)),
        "wma" => (MediaType::Audio, Some(AudioFormat::Wma)),
        "ape" => (MediaType::Audio, Some(AudioFormat::Ape)),
        "wav" => (MediaType::Audio, Some(AudioFormat::Wav)),
        "aif" | "aiff" => (MediaType::Audio, Some(AudioFormat::Aiff)),
        "dsf" | "dff" => (MediaType::Audio, Some(AudioFormat::Dsf)),
        "wv" => (MediaType::Audio, Some(AudioFormat::WavPack)),

        "mp4" | "mkv" | "avi" | "mov" | "wmv" | "flv" | "webm" | "m4v" | "ts" | "m2ts" => {
            (MediaType::Video, None)
        }

        "cbz" | "cbr" | "cb7" | "cbt" => (MediaType::Comic, None),
        "epub" => (MediaType::Book, None),

        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp" | "tiff" => (MediaType::Image, None),

        "cue" => (MediaType::Cue, None),
        "nfo" => (MediaType::Nfo, None),

        _ => (MediaType::Unknown, None),
    }
}

/// Whether a file is a library entry in its own right.
///
/// Images and NFO files are sidecars that describe other media — they are
/// read from disk where needed, but indexing them would pad file counts and
/// clutter the list with entries that have nothing to edit.
pub fn is_indexable(media_type: &MediaType) -> bool {
    matches!(
        media_type,
        MediaType::Audio | MediaType::Video | MediaType::Comic | MediaType::Book
    )
}

pub fn is_audio_extension(ext: &str) -> bool {
    matches!(
        ext.to_lowercase().as_str(),
        "mp3" | "flac" | "ogg" | "oga" | "opus" | "aac" | "m4a" | "m4b" | "m4p" | "wma"
            | "ape" | "wav" | "aif" | "aiff" | "dsf" | "dff" | "wv"
    )
}
