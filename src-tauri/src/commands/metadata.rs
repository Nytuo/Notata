use tauri::State;

use crate::db::queries;
use crate::metadata::{reader, writer};
use crate::models::album::CoverArtData;
use crate::models::track::{AudioProperties, TrackMetadata};
use crate::state::AppState;

/// Best-effort: a failure to stamp the "modified" marker must not fail the
/// write that already succeeded on disk.
pub(crate) fn stamp_modified(state: &AppState, path: &str) {
    let now = chrono::Utc::now().timestamp();
    if let Ok(conn) = state.db.lock() {
        if let Err(e) = queries::mark_file_modified(&conn, path, now) {
            log::warn!("Failed to mark {} as modified: {}", path, e);
        }
    }
}

#[tauri::command]
pub fn read_metadata(path: String) -> Result<TrackMetadata, String> {
    reader::read_metadata(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn read_metadata_batch(
    paths: Vec<String>,
) -> Result<Vec<(String, TrackMetadata)>, String> {
    let mut results = Vec::new();
    for path in paths {
        match reader::read_metadata(&path) {
            Ok(meta) => results.push((path, meta)),
            Err(e) => {
                log::warn!("Failed to read metadata for {}: {}", path, e);
                results.push((path, TrackMetadata::default()));
            }
        }
    }
    Ok(results)
}

#[tauri::command]
pub fn write_metadata(
    state: State<'_, AppState>,
    path: String,
    metadata: TrackMetadata,
) -> Result<(), String> {
    writer::write_metadata(&path, &metadata).map_err(|e| e.to_string())?;
    stamp_modified(&state, &path);
    Ok(())
}

#[tauri::command]
pub fn write_metadata_batch(
    state: State<'_, AppState>,
    entries: Vec<(String, TrackMetadata)>,
) -> Result<Vec<(String, bool, String)>, String> {
    let mut results = Vec::new();
    for (path, metadata) in entries {
        match writer::write_metadata(&path, &metadata) {
            Ok(()) => {
                stamp_modified(&state, &path);
                results.push((path, true, String::new()));
            }
            Err(e) => results.push((path, false, e.to_string())),
        }
    }
    Ok(results)
}

#[tauri::command]
pub fn get_audio_properties(path: String) -> Result<AudioProperties, String> {
    reader::read_audio_properties(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_embedded_cover_art(path: String) -> Result<Option<CoverArtData>, String> {
    reader::read_embedded_cover_art(&path).map_err(|e| e.to_string())
}
