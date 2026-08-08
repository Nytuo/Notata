/// Read a local file's raw bytes, e.g. so the frontend can preview or embed
/// a picture the user picked from disk via the native file dialog.
#[tauri::command]
pub fn read_file_bytes(path: String) -> Result<tauri::ipc::Response, String> {
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    Ok(tauri::ipc::Response::new(bytes))
}
