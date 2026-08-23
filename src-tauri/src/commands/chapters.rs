use tauri::{AppHandle, Emitter, State};

use crate::chapters::detect::allin1::{self, Allin1Status};
use crate::chapters::detect::{ssm, DetectProgress};
use crate::chapters::{self, Chapter};
use crate::commands::metadata::stamp_modified;
use crate::state::AppState;

#[tauri::command]
pub async fn read_chapters(state: State<'_, AppState>, path: String) -> Result<Vec<Chapter>, String> {
    Ok(chapters::read_chapters_best_effort(&state, &path).await)
}

#[tauri::command]
pub fn write_chapters(
    state: State<'_, AppState>,
    path: String,
    chapters: Vec<Chapter>,
) -> Result<(), String> {
    chapters::write_chapters(&path, &chapters).map_err(|e| e.to_string())?;
    stamp_modified(&state, &path);
    Ok(())
}

fn emit_progress(app: &AppHandle, stage: &str, percent: f64) {
    let _ = app.emit(
        "chapters:detect-progress",
        DetectProgress {
            stage: stage.to_string(),
            percent,
        },
    );
}

#[tauri::command]
pub async fn detect_chapters_dsp(app: AppHandle, path: String) -> Result<Vec<Chapter>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        ssm::detect(&path, |stage, percent| emit_progress(&app, stage, percent))
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn check_allin1_available(state: State<'_, AppState>) -> Result<Allin1Status, String> {
    let allin1_path = allin1::resolve_allin1_path(&state);
    Ok(allin1::check_available(&allin1_path).await)
}

#[tauri::command]
pub async fn detect_chapters_ai(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<Vec<Chapter>, String> {
    let allin1_path = allin1::resolve_allin1_path(&state);
    allin1::detect(&app, &allin1_path, &path, |stage, percent| {
        emit_progress(&app, stage, percent)
    })
    .await
    .map_err(|e| e.to_string())
}
