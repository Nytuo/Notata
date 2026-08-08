use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use crate::db::queries;
use crate::state::{AppState, PREF_FFMPEG_PATH};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FfmpegStatus {
    pub available: bool,
    pub version: Option<String>,
    pub path: String,
}

/// The configured ffmpeg path, or the bare command name to resolve via PATH.
pub fn resolve_ffmpeg_path(state: &AppState) -> String {
    let custom = state
        .db
        .lock()
        .ok()
        .and_then(|conn| queries::get_preference(&conn, PREF_FFMPEG_PATH).ok().flatten())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    custom.unwrap_or_else(|| "ffmpeg".to_string())
}

/// ffprobe is assumed to live alongside ffmpeg. If a custom ffmpeg path was
/// given, swap the file name; otherwise resolve "ffprobe" from PATH.
pub fn resolve_ffprobe_path(ffmpeg_path: &str) -> String {
    let path = Path::new(ffmpeg_path);
    if path.file_name().is_none() || ffmpeg_path == "ffmpeg" {
        return "ffprobe".to_string();
    }

    let ext = path.extension().and_then(|e| e.to_str());
    let file_name = match ext {
        Some(e) => format!("ffprobe.{}", e),
        None => "ffprobe".to_string(),
    };

    let sibling: PathBuf = path.with_file_name(file_name);
    sibling.to_string_lossy().to_string()
}

pub async fn check_ffmpeg(ffmpeg_path: &str) -> FfmpegStatus {
    let output = Command::new(ffmpeg_path).arg("-version").output().await;

    match output {
        Ok(out) if out.status.success() => {
            let text = String::from_utf8_lossy(&out.stdout);
            let version = text.lines().next().map(|l| l.to_string());
            FfmpegStatus {
                available: true,
                version,
                path: ffmpeg_path.to_string(),
            }
        }
        _ => FfmpegStatus {
            available: false,
            version: None,
            path: ffmpeg_path.to_string(),
        },
    }
}

/// Duration of the primary audio stream, in seconds. Used to turn ffmpeg's
/// `time=` progress lines into a percentage.
pub async fn probe_duration_secs(ffprobe_path: &str, path: &str) -> Option<f64> {
    let output = Command::new(ffprobe_path)
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "csv=p=0",
            path,
        ])
        .output()
        .await
        .ok()?;

    if !output.status.success() {
        return None;
    }

    String::from_utf8_lossy(&output.stdout).trim().parse().ok()
}

/// Codec name of the first audio stream, e.g. "mp3", "flac", "aac".
pub async fn probe_audio_codec(ffprobe_path: &str, path: &str) -> Option<String> {
    let output = Command::new(ffprobe_path)
        .args([
            "-v",
            "error",
            "-select_streams",
            "a:0",
            "-show_entries",
            "stream=codec_name",
            "-of",
            "csv=p=0",
            path,
        ])
        .output()
        .await
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let codec = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if codec.is_empty() {
        None
    } else {
        Some(codec)
    }
}

/// Parse an ffmpeg stderr progress line like
/// `size=... time=00:01:23.45 bitrate=...` into seconds.
fn parse_time_seconds(line: &str) -> Option<f64> {
    let idx = line.find("time=")?;
    let rest = &line[idx + 5..];
    let token = rest.split_whitespace().next()?;
    let mut parts = token.split(':');
    let h: f64 = parts.next()?.parse().ok()?;
    let m: f64 = parts.next()?.parse().ok()?;
    let s: f64 = parts.next()?.parse().ok()?;
    Some(h * 3600.0 + m * 60.0 + s)
}

/// Run ffmpeg to completion, invoking `on_progress` with 0.0..=1.0 as
/// `time=` lines arrive on stderr. `total_secs` of `None`/`0` means progress
/// can't be computed, so `on_progress` is simply never called.
pub async fn run_with_progress<F: FnMut(f64)>(
    ffmpeg_path: &str,
    args: &[String],
    total_secs: Option<f64>,
    mut on_progress: F,
) -> std::io::Result<std::process::ExitStatus> {
    let mut child = Command::new(ffmpeg_path)
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    if let Some(stderr) = child.stderr.take() {
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if let (Some(total), Some(elapsed)) = (total_secs, parse_time_seconds(&line)) {
                if total > 0.0 {
                    on_progress((elapsed / total).clamp(0.0, 1.0));
                }
            }
        }
    }

    child.wait().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_time_from_progress_line() {
        let line = "frame=  100 fps= 25 q=-1.0 size=    1024kB time=00:01:23.45 bitrate= 100.0kbits/s speed=2x";
        assert_eq!(parse_time_seconds(line), Some(83.45));
    }

    #[test]
    fn missing_time_yields_none() {
        assert_eq!(parse_time_seconds("no progress info here"), None);
    }

    #[test]
    fn ffprobe_sibling_swaps_file_name() {
        assert_eq!(resolve_ffprobe_path("/opt/tools/ffmpeg"), "/opt/tools/ffprobe");
        assert_eq!(resolve_ffprobe_path("ffmpeg"), "ffprobe");
    }
}
