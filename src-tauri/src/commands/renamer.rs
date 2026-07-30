use tauri::State;

use crate::db::queries;
use crate::renamer::presets::{builtin_presets, RenamePreset};
use crate::renamer::{
    apply_plan, build_plan_for_paths, RenameOutcome, RenamePlan, RenamePlanEntry,
};
use crate::state::AppState;

#[tauri::command]
pub fn list_rename_presets() -> Vec<RenamePreset> {
    builtin_presets()
}

/// Validate a template without needing any files.
#[tauri::command]
pub fn validate_rename_template(template: String) -> Result<(), String> {
    crate::renamer::template::parse(&template).map(|_| ())
}

/// Build a dry-run rename plan for the given files.
///
/// Each file's metadata is read with the reader that matches its type, so
/// video files resolve from their NFO rather than from audio tags.
#[tauri::command]
pub fn preview_rename(
    paths: Vec<String>,
    template: String,
    base_dir: Option<String>,
) -> Result<RenamePlan, String> {
    build_plan_for_paths(&template, &paths, base_dir.as_deref()).map_err(|e| e.to_string())
}

/// Execute a previously built plan and re-point the library index at the new
/// paths so the file list does not go stale.
#[tauri::command]
pub fn apply_rename(
    state: State<'_, AppState>,
    entries: Vec<RenamePlanEntry>,
) -> Result<Vec<RenameOutcome>, String> {
    let outcomes = apply_plan(&entries);

    if let Ok(conn) = state.db.lock() {
        for outcome in outcomes.iter().filter(|o| o.success) {
            if let Err(e) = queries::update_file_path(&conn, &outcome.source_path, &outcome.target_path)
            {
                log::warn!(
                    "Renamed {} but could not update the index: {}",
                    outcome.source_path,
                    e
                );
            }
        }
    }

    Ok(outcomes)
}
