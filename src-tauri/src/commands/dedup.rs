use std::collections::HashMap;
use std::path::PathBuf;

use tauri::State;

use crate::db::queries;
use crate::dedup::detector;
use crate::dedup::{DuplicateMode, DuplicateReport, ResolveOutcome};
use crate::metadata::reader;
use crate::models::track::TrackMetadata;
use crate::state::AppState;

/// Scan the indexed library for duplicates.
///
/// Runs on a blocking thread: exact mode hashes file contents, which would
/// otherwise stall the async runtime.
#[tauri::command]
pub async fn find_duplicates(
    state: State<'_, AppState>,
    mode: DuplicateMode,
    threshold: Option<f64>,
) -> Result<DuplicateReport, String> {
    let files = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        queries::get_all_audio_files(&conn).map_err(|e| e.to_string())?
    };

    let threshold = threshold.unwrap_or(0.90);

    let groups = tauri::async_runtime::spawn_blocking(move || match mode {
        DuplicateMode::Exact => detector::find_exact_duplicates(&files),
        DuplicateMode::Fuzzy => {
            let mut metadata: HashMap<String, TrackMetadata> = HashMap::new();
            for file in &files {
                if let Ok(meta) = reader::read_metadata(&file.path) {
                    metadata.insert(file.path.clone(), meta);
                }
            }
            detector::find_fuzzy_duplicates(&files, &metadata, threshold)
        }
    })
    .await
    .map_err(|e| e.to_string())?;

    Ok(detector::build_report(groups))
}

/// Move the selected duplicates into a dated quarantine folder and drop them
/// from the index. Files are not unlinked, so the action can be undone.
#[tauri::command]
pub fn resolve_duplicates(
    state: State<'_, AppState>,
    paths: Vec<String>,
) -> Result<Vec<ResolveOutcome>, String> {
    let trash_dir = PathBuf::from(&state.cover_art_cache_dir)
        .parent()
        .map(|p| p.join("quarantine"))
        .ok_or_else(|| "Could not resolve the quarantine directory".to_string())?;

    let outcomes = detector::quarantine_files(&paths, &trash_dir).map_err(|e| e.to_string())?;

    if let Ok(conn) = state.db.lock() {
        for outcome in outcomes.iter().filter(|o| o.success) {
            if let Err(e) = queries::delete_media_file(&conn, &outcome.path) {
                log::warn!("Quarantined {} but could not update the index: {}", outcome.path, e);
            }
        }
    }

    Ok(outcomes)
}
