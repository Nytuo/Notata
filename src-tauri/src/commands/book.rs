use tauri::State;

use crate::commands::metadata::stamp_modified;
use crate::metadata::book as book_meta;
use crate::models::book::{BookCover, BookMetadata, BookProperties};
use crate::state::AppState;

#[tauri::command]
pub fn read_book_metadata(path: String) -> Result<BookMetadata, String> {
    book_meta::read_book_metadata(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_book_properties(path: String) -> Result<BookProperties, String> {
    book_meta::read_book_properties(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_book_cover(path: String) -> Result<Option<BookCover>, String> {
    book_meta::read_book_cover(&path).map_err(|e| e.to_string())
}

/// Write metadata into the archive. Returns the entry that was rewritten.
#[tauri::command]
pub fn write_book_metadata(
    state: State<'_, AppState>,
    path: String,
    metadata: BookMetadata,
) -> Result<String, String> {
    let entry = book_meta::write_book_metadata(&path, &metadata).map_err(|e| e.to_string())?;
    stamp_modified(&state, &path);
    Ok(entry)
}

#[tauri::command]
pub fn write_book_metadata_batch(
    state: State<'_, AppState>,
    entries: Vec<(String, BookMetadata)>,
) -> Result<Vec<(String, bool, String)>, String> {
    let mut results = Vec::new();
    for (path, metadata) in entries {
        match book_meta::write_book_metadata(&path, &metadata) {
            Ok(_) => {
                stamp_modified(&state, &path);
                results.push((path, true, String::new()));
            }
            Err(e) => results.push((path, false, e.to_string())),
        }
    }
    Ok(results)
}
