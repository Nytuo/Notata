use tauri::State;

use crate::db::queries;
use crate::models::media_file::{
    DirectoryNode, LibraryRoot, LibraryStats, MediaFile, MediaKind, ScanResult,
};
use crate::scanner::walker;
use crate::state::AppState;

#[tauri::command]
pub fn scan_directory(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    path: String,
    media_kind: Option<MediaKind>,
) -> Result<ScanResult, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;

    let existing =
        queries::get_library_root_by_path(&conn, &path).map_err(|e| e.to_string())?;

    let root_id = match existing {
        // A re-scan keeps the kind already chosen unless the caller overrides it.
        Some(root) => {
            if let Some(kind) = media_kind {
                if kind != root.media_kind {
                    queries::set_root_media_kind(&conn, &root.id, kind)
                        .map_err(|e| e.to_string())?;
                }
            }
            root.id
        }
        None => {
            let new_id = uuid::Uuid::new_v4().to_string();
            let root = LibraryRoot {
                id: new_id.clone(),
                path: path.clone(),
                label: std::path::Path::new(&path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.to_string()),
                added_at: chrono::Utc::now().timestamp(),
                last_scan: None,
                previous_scan: None,
                media_kind: media_kind.unwrap_or(MediaKind::Music),
            };
            queries::insert_library_root(&conn, &root).map_err(|e| e.to_string())?;
            new_id
        }
    };

    drop(conn);

    walker::scan_directory(&app, &state, &path, &root_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_root_media_kind(
    state: State<'_, AppState>,
    root_id: String,
    media_kind: MediaKind,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    queries::set_root_media_kind(&conn, &root_id, media_kind).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_library_roots(state: State<'_, AppState>) -> Result<Vec<LibraryRoot>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    queries::get_library_roots(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_files_in_directory(
    state: State<'_, AppState>,
    path: String,
) -> Result<Vec<MediaFile>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    queries::get_files_in_directory(&conn, &path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_files_by_root(
    state: State<'_, AppState>,
    root_id: String,
) -> Result<Vec<MediaFile>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    queries::get_files_by_root(&conn, &root_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_directory_tree(root: String) -> Result<DirectoryNode, String> {
    walker::build_directory_tree(&root).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_library_stats(state: State<'_, AppState>) -> Result<LibraryStats, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let roots = queries::get_library_roots(&conn).map_err(|e| e.to_string())?;

    let total_files: usize = conn
        .query_row("SELECT COUNT(*) FROM media_files", [], |row| row.get(0))
        .unwrap_or(0);
    let total_size: u64 = conn
        .query_row(
            "SELECT COALESCE(SUM(file_size), 0) FROM media_files",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    Ok(LibraryStats {
        total_files,
        total_size,
        roots,
    })
}

#[tauri::command]
pub fn remove_library_root(
    state: State<'_, AppState>,
    root_id: String,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    queries::delete_library_root(&conn, &root_id).map_err(|e| e.to_string())
}
