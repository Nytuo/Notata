use tauri::State;

use crate::db::queries;
use crate::renamer::presets::{builtin_presets, RenamePreset};
use crate::renamer::{
    apply_plan, build_plan_for_paths, prune_empty_dirs, RenameOutcome, RenamePlan,
    RenamePlanEntry,
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
///
/// With `repair_tree`, folder-building templates are anchored at each file's
/// library root instead of its own folder, so files filed at the wrong depth
/// (`Artist/Album/Artist/Album/track`) land where the preset says.
#[tauri::command]
pub fn preview_rename(
    state: State<'_, AppState>,
    paths: Vec<String>,
    template: String,
    base_dir: Option<String>,
    repair_tree: Option<bool>,
) -> Result<RenamePlan, String> {
    let roots = if repair_tree.unwrap_or(false) {
        Some(library_root_paths(&state)?)
    } else {
        None
    };
    build_plan_for_paths(&template, &paths, base_dir.as_deref(), roots.as_deref())
        .map_err(|e| e.to_string())
}

fn library_root_paths(state: &AppState) -> Result<Vec<String>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    Ok(queries::get_library_roots(&conn)
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|r| r.path)
        .collect())
}

/// What applying a plan did.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyRenameResult {
    pub outcomes: Vec<RenameOutcome>,
    /// Emptied folders removed afterwards.
    pub pruned_dirs: usize,
}

/// Execute a previously built plan and re-point the library index at the new
/// paths so the file list does not go stale.
#[tauri::command]
///
/// With `prune_empty`, folders the move left empty (or holding only OS junk
/// like `.DS_Store`) are removed, up to but never including a library root or
/// the chosen destination folder.
pub fn apply_rename(
    state: State<'_, AppState>,
    entries: Vec<RenamePlanEntry>,
    prune_empty: Option<bool>,
    base_dir: Option<String>,
) -> Result<ApplyRenameResult, String> {
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

    let pruned_dirs = if prune_empty.unwrap_or(false) {
        let mut stop_at = library_root_paths(&state)?;
        stop_at.extend(base_dir.filter(|b| !b.is_empty()));
        prune_empty_dirs(&outcomes, &stop_at)
    } else {
        0
    };

    Ok(ApplyRenameResult {
        outcomes,
        pruned_dirs,
    })
}
