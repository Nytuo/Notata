use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use crate::commands::metadata::stamp_modified;
use crate::db::queries;
use crate::state::AppState;
use crate::transcode::engine::{self, TranscodePreviewEntry, TranscodeResult};
use crate::transcode::ffmpeg::{self, FfmpegStatus};
use crate::transcode::formats::{self, TranscodeDestination, TranscodeFormatInfo, TranscodeOptions};

#[tauri::command]
pub fn list_transcode_formats() -> Vec<TranscodeFormatInfo> {
    formats::catalog()
}

#[tauri::command]
pub async fn check_ffmpeg_available(state: State<'_, AppState>) -> Result<FfmpegStatus, String> {
    let ffmpeg_path = ffmpeg::resolve_ffmpeg_path(&state);
    Ok(ffmpeg::check_ffmpeg(&ffmpeg_path).await)
}

#[tauri::command]
pub fn preview_transcode(
    paths: Vec<String>,
    options: TranscodeOptions,
) -> Result<Vec<TranscodePreviewEntry>, String> {
    engine::preview(&paths, &options)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TranscodeProgress {
    path: String,
    index: usize,
    total: usize,
    percent: f64,
}

#[tauri::command]
pub async fn transcode_files(
    app: AppHandle,
    state: State<'_, AppState>,
    paths: Vec<String>,
    options: TranscodeOptions,
) -> Result<Vec<TranscodeResult>, String> {
    let fmt = formats::find(&options.target_format)
        .ok_or_else(|| format!("Unknown target format '{}'", options.target_format))?;
    let ffmpeg_path = ffmpeg::resolve_ffmpeg_path(&state);
    let ffprobe_path = ffmpeg::resolve_ffprobe_path(&ffmpeg_path);

    let total = paths.len();
    let mut results = Vec::with_capacity(total);

    for (index, path) in paths.iter().enumerate() {
        let app_for_progress = app.clone();
        let path_for_progress = path.clone();
        let result = engine::transcode_one(
            &ffmpeg_path,
            &ffprobe_path,
            path,
            &fmt,
            &options,
            move |percent| {
                let _ = app_for_progress.emit(
                    "transcode:progress",
                    TranscodeProgress {
                        path: path_for_progress.clone(),
                        index,
                        total,
                        percent,
                    },
                );
            },
        )
        .await;

        // Replacing in place retires the old indexed path in favor of the
        // new one; writing to a separate folder leaves the indexed source
        // untouched, so the new file only appears in the library on rescan.
        if result.success && !result.skipped && matches!(options.destination, TranscodeDestination::ReplaceInPlace) {
            if let Some(output_path) = &result.output_path {
                if output_path != path {
                    if let Ok(conn) = state.db.lock() {
                        let _ = queries::update_file_path(&conn, path, output_path);
                    }
                }
                stamp_modified(&state, output_path);
            }
        }

        results.push(result);
    }

    Ok(results)
}
