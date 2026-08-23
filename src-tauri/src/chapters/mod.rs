pub mod audio;
pub mod detect;
mod ffprobe;
pub mod mp3;
pub mod mp4;
pub mod vorbis;

use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::error::{NotataError, Result};
use crate::state::AppState;
use crate::transcode::ffmpeg::{resolve_ffmpeg_path, resolve_ffprobe_path};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Chapter {
    pub id: String,
    pub title: String,
    pub start_ms: u64,
    pub end_ms: u64,
}

fn extension_of(path: &str) -> String {
    Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase()
}

pub fn read_chapters(path: &str) -> Result<Vec<Chapter>> {
    match extension_of(path).as_str() {
        "mp3" => mp3::read_chapters(path),
        "flac" | "ogg" | "opus" => vorbis::read_chapters(path),
        "m4a" | "m4b" | "mp4" => mp4::read_chapters(path),
        ext => Err(NotataError::Custom(format!(
            "Chapters are not supported for .{ext} files"
        ))),
    }
}

pub async fn read_chapters_best_effort(state: &AppState, path: &str) -> Vec<Chapter> {
    if let Ok(chapters) = read_chapters(path) {
        if !chapters.is_empty() {
            return chapters;
        }
    }

    let ffmpeg_path = resolve_ffmpeg_path(state);
    let ffprobe_path = resolve_ffprobe_path(&ffmpeg_path);
    ffprobe::read_chapters(&ffprobe_path, path).await.unwrap_or_default()
}

pub fn write_chapters(path: &str, chapters: &[Chapter]) -> Result<()> {
    match extension_of(path).as_str() {
        "mp3" => mp3::write_chapters(path, chapters),
        "flac" | "ogg" | "opus" => vorbis::write_chapters(path, chapters),
        "m4a" | "m4b" | "mp4" => mp4::write_chapters(path, chapters),
        ext => Err(NotataError::Custom(format!(
            "Chapters are not supported for .{ext} files"
        ))),
    }
}
