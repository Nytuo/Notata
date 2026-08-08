use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::metadata::{reader, writer};
use crate::scanner::mime::is_audio_extension;

use super::ffmpeg;
use super::formats::{self, TranscodeDestination, TranscodeFormatInfo, TranscodeOptions};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscodePreviewEntry {
    pub path: String,
    pub source_extension: String,
    pub output_path: String,
    pub already_target_extension: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscodeResult {
    pub path: String,
    pub output_path: Option<String>,
    pub success: bool,
    pub skipped: bool,
    pub error: Option<String>,
}

fn source_extension(path: &str) -> String {
    Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_lowercase()
}

fn output_path_for(path: &str, fmt: &TranscodeFormatInfo, destination: &TranscodeDestination) -> String {
    let src = Path::new(path);
    let stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
    let file_name = format!("{}.{}", stem, fmt.extension);

    match destination {
        TranscodeDestination::ReplaceInPlace => {
            let parent = src.parent().unwrap_or_else(|| Path::new(""));
            parent.join(file_name).to_string_lossy().to_string()
        }
        TranscodeDestination::Folder { path: dir } => {
            Path::new(dir).join(file_name).to_string_lossy().to_string()
        }
    }
}

pub fn preview(paths: &[String], options: &TranscodeOptions) -> Result<Vec<TranscodePreviewEntry>, String> {
    let fmt = formats::find(&options.target_format)
        .ok_or_else(|| format!("Unknown target format '{}'", options.target_format))?;

    Ok(paths
        .iter()
        .map(|path| {
            let ext = source_extension(path);
            TranscodePreviewEntry {
                path: path.clone(),
                source_extension: ext.clone(),
                output_path: output_path_for(path, &fmt, &options.destination),
                already_target_extension: ext == fmt.extension,
            }
        })
        .collect())
}

fn build_args(
    src: &str,
    tmp_output: &str,
    fmt: &TranscodeFormatInfo,
    options: &TranscodeOptions,
    stream_copy: bool,
) -> Vec<String> {
    let mut args: Vec<String> = vec![
        "-y".into(),
        "-i".into(),
        src.into(),
        "-vn".into(),
        "-map_metadata".into(),
        "-1".into(),
        "-map".into(),
        "0:a:0".into(),
    ];

    if stream_copy {
        args.push("-c:a".into());
        args.push("copy".into());
    } else {
        match fmt.id.as_str() {
            "mp3" => {
                args.push("-c:a".into());
                args.push("libmp3lame".into());
                args.push("-b:a".into());
                args.push(format!("{}k", options.bitrate_kbps.or(fmt.default_bitrate_kbps).unwrap_or(256)));
            }
            "aac" => {
                args.push("-c:a".into());
                args.push("aac".into());
                args.push("-b:a".into());
                args.push(format!("{}k", options.bitrate_kbps.or(fmt.default_bitrate_kbps).unwrap_or(256)));
            }
            "ogg" => {
                args.push("-c:a".into());
                args.push("libvorbis".into());
                args.push("-b:a".into());
                args.push(format!("{}k", options.bitrate_kbps.or(fmt.default_bitrate_kbps).unwrap_or(256)));
            }
            "opus" => {
                args.push("-c:a".into());
                args.push("libopus".into());
                args.push("-b:a".into());
                args.push(format!("{}k", options.bitrate_kbps.or(fmt.default_bitrate_kbps).unwrap_or(160)));
            }
            "wma" => {
                args.push("-c:a".into());
                args.push("wmav2".into());
                args.push("-b:a".into());
                args.push(format!("{}k", options.bitrate_kbps.or(fmt.default_bitrate_kbps).unwrap_or(192)));
            }
            "flac" => {
                args.push("-c:a".into());
                args.push("flac".into());
                args.push("-compression_level".into());
                args.push(options.flac_compression.unwrap_or(5).min(8).to_string());
            }
            "alac" => {
                args.push("-c:a".into());
                args.push("alac".into());
            }
            "wav" => {
                args.push("-c:a".into());
                args.push("pcm_s16le".into());
            }
            "aiff" => {
                args.push("-c:a".into());
                args.push("pcm_s16be".into());
            }
            other => {
                // Should not happen, formats::find() already validated the id.
                args.push("-c:a".into());
                args.push(other.into());
            }
        }
    }

    args.push(tmp_output.into());
    args
}

/// Copy tags and embedded cover art from the source onto the freshly
/// transcoded output using the same lofty-backed reader/writer the rest of
/// the app uses, so tags stay consistent regardless of what ffmpeg preserved.
fn carry_over_metadata(src: &str, output: &str) -> Result<(), String> {
    let metadata = reader::read_metadata(src).map_err(|e| e.to_string())?;
    writer::write_metadata(output, &metadata).map_err(|e| e.to_string())?;

    if let Ok(Some(cover)) = reader::read_embedded_cover_art(src) {
        if let Ok(bytes) = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &cover.data) {
            let _ = crate::coverart::manager::embed_cover_art_in_file(output, &bytes, &cover.mime_type);
        }
    }

    Ok(())
}

/// Transcode a single file. `on_progress` receives 0.0..=1.0 while ffmpeg
/// runs; it is not called at all when duration can't be probed.
pub async fn transcode_one<F: FnMut(f64)>(
    ffmpeg_path: &str,
    ffprobe_path: &str,
    path: &str,
    fmt: &TranscodeFormatInfo,
    options: &TranscodeOptions,
    on_progress: F,
) -> TranscodeResult {
    if !Path::new(path).is_file() {
        return TranscodeResult {
            path: path.to_string(),
            output_path: None,
            success: false,
            skipped: false,
            error: Some("File not found".into()),
        };
    }

    let ext = source_extension(path);
    if !is_audio_extension(&ext) {
        return TranscodeResult {
            path: path.to_string(),
            output_path: None,
            success: false,
            skipped: true,
            error: Some("Not a recognized audio file".into()),
        };
    }

    let final_output = output_path_for(path, fmt, &options.destination);
    let replacing_in_place = matches!(options.destination, TranscodeDestination::ReplaceInPlace);
    let same_path = replacing_in_place && final_output == path;

    let source_codec = ffmpeg::probe_audio_codec(ffprobe_path, path).await;
    let stream_copy = options.prefer_stream_copy
        && source_codec.as_deref() == Some(fmt.codec_name.as_str());

    if same_path && stream_copy {
        return TranscodeResult {
            path: path.to_string(),
            output_path: Some(final_output),
            success: true,
            skipped: true,
            error: None,
        };
    }

    if let TranscodeDestination::Folder { path: dir } = &options.destination {
        if let Err(e) = std::fs::create_dir_all(dir) {
            return TranscodeResult {
                path: path.to_string(),
                output_path: None,
                success: false,
                skipped: false,
                error: Some(format!("Could not create destination folder: {}", e)),
            };
        }
    }

    // Write to a temp file first so a same-path replace never reads and
    // writes the same file at once, and a failed run never clobbers the
    // original.
    let tmp_output: PathBuf = {
        let out = Path::new(&final_output);
        let dir = out.parent().unwrap_or_else(|| Path::new(""));
        let file_name = out
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("output");
        dir.join(format!(".notata-transcode-{}", file_name))
    };
    let tmp_output_str = tmp_output.to_string_lossy().to_string();

    let total_secs = ffmpeg::probe_duration_secs(ffprobe_path, path).await;
    let args = build_args(path, &tmp_output_str, fmt, options, stream_copy);

    let status = ffmpeg::run_with_progress(ffmpeg_path, &args, total_secs, on_progress).await;

    let status = match status {
        Ok(s) => s,
        Err(e) => {
            let _ = std::fs::remove_file(&tmp_output);
            return TranscodeResult {
                path: path.to_string(),
                output_path: None,
                success: false,
                skipped: false,
                error: Some(format!("Could not run ffmpeg: {}", e)),
            };
        }
    };

    if !status.success() {
        let _ = std::fs::remove_file(&tmp_output);
        return TranscodeResult {
            path: path.to_string(),
            output_path: None,
            success: false,
            skipped: false,
            error: Some("ffmpeg exited with an error".into()),
        };
    }

    if let Err(e) = carry_over_metadata(path, &tmp_output_str) {
        log::warn!("Transcoded {} but could not carry over tags: {}", path, e);
    }

    if replacing_in_place {
        if let Err(e) = std::fs::remove_file(path) {
            let _ = std::fs::remove_file(&tmp_output);
            return TranscodeResult {
                path: path.to_string(),
                output_path: None,
                success: false,
                skipped: false,
                error: Some(format!("Could not remove original file: {}", e)),
            };
        }
    }

    if let Err(e) = std::fs::rename(&tmp_output, &final_output) {
        return TranscodeResult {
            path: path.to_string(),
            output_path: None,
            success: false,
            skipped: false,
            error: Some(format!("Could not finalize output file: {}", e)),
        };
    }

    TranscodeResult {
        path: path.to_string(),
        output_path: Some(final_output),
        success: true,
        skipped: false,
        error: None,
    }
}
