use std::collections::HashMap;
use std::path::{Path, PathBuf};

use tauri::{AppHandle, Emitter, State};

use crate::db::queries;
use crate::dedup::{cache as fingerprint_cache, detector};
use crate::dedup::{DedupProgress, DuplicateMode, DuplicateReport, ResolveOutcome};
use crate::metadata::reader;
use crate::models::track::TrackMetadata;
use crate::state::AppState;

/// Scan the indexed library for duplicates.
///
/// Runs on a blocking thread: exact mode hashes file contents, which would
/// otherwise stall the async runtime.
#[tauri::command]
pub async fn find_duplicates(
    app: AppHandle,
    state: State<'_, AppState>,
    mode: DuplicateMode,
    threshold: Option<f64>,
) -> Result<DuplicateReport, String> {
    let files = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        queries::get_all_audio_files(&conn).map_err(|e| e.to_string())?
    };
    let fingerprint_cache_path = state.fingerprint_cache_path.clone();

    // Fuzzy's threshold is a Jaro-Winkler string score, which sits close to
    // 1.0 for genuine matches. Acoustic's is a coverage x tightness score
    // over matched fingerprint segments, which rarely gets that high even
    // for identical audio (fades, encoder padding, and frame alignment all
    // eat into coverage) - so it needs its own, much looser, default.
    let threshold = threshold.unwrap_or(match mode {
        DuplicateMode::Acoustic => 0.55,
        _ => 0.90,
    });

    let (groups, warnings) = tauri::async_runtime::spawn_blocking(move || match mode {
        DuplicateMode::Exact => (detector::find_exact_duplicates(&files), Vec::new()),
        DuplicateMode::Fuzzy => {
            let mut metadata: HashMap<String, TrackMetadata> = HashMap::new();
            for file in &files {
                if let Ok(meta) = reader::read_metadata(&file.path) {
                    metadata.insert(file.path.clone(), meta);
                }
            }
            (
                detector::find_fuzzy_duplicates(&files, &metadata, threshold),
                Vec::new(),
            )
        }
        DuplicateMode::Acoustic => {
            let cache_path = Path::new(&fingerprint_cache_path);
            let mut cache = fingerprint_cache::load(cache_path);
            let result =
                detector::find_acoustic_duplicates(&files, threshold, &mut cache, |scanned, total| {
                    let _ = app.emit("dedup:progress", DedupProgress { scanned, total });
                });
            if let Err(e) = fingerprint_cache::save(cache_path, &cache) {
                log::warn!("Could not save the fingerprint cache: {}", e);
            }
            result
        }
    })
    .await
    .map_err(|e| e.to_string())?;

    Ok(detector::build_report(groups, warnings))
}

/// Read a media file's raw bytes so the frontend can build a blob URL and
/// preview it in an `<audio>` element while deciding which duplicate to keep.
#[tauri::command]
pub fn read_audio_preview(path: String) -> Result<tauri::ipc::Response, String> {
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    Ok(tauri::ipc::Response::new(bytes))
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
