use std::sync::Arc;
use tauri::State;

use crate::models::album::{CoverArt, CoverArtData};
use crate::state::AppState;

#[tauri::command]
pub async fn fetch_provider_cover_art(
    state: State<'_, AppState>,
    provider: String,
    release_id: String,
) -> Result<Vec<CoverArt>, String> {
    let provider_ref = {
        let registry = state.providers.lock().map_err(|e| e.to_string())?;
        registry
            .get_arc(&provider)
            .ok_or_else(|| format!("Provider '{}' not found", provider))?
    };

    provider_ref
        .get_cover_art(&release_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn download_cover_art(url: String) -> Result<CoverArtData, String> {
    crate::coverart::manager::download_image(&url)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn embed_cover_art(
    path: String,
    image_data: Vec<u8>,
    mime_type: String,
) -> Result<(), String> {
    crate::coverart::manager::embed_cover_art_in_file(&path, &image_data, &mime_type)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remove_cover_art(path: String) -> Result<(), String> {
    crate::coverart::manager::remove_cover_art_from_file(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn search_cover_art(query: String) -> Result<Vec<CoverArt>, String> {
    let (itunes, deezer) = tokio::join!(
        crate::coverart::search::search_itunes_cover_art(&query),
        crate::coverart::search::search_deezer_cover_art(&query),
    );

    let mut results = Vec::new();
    if let Ok(mut v) = itunes {
        results.append(&mut v);
    }
    if let Ok(mut v) = deezer {
        results.append(&mut v);
    }
    Ok(results)
}
