use std::sync::Arc;
use tauri::State;

use crate::models::provider_result::{ProviderInfo, ProviderRelease, ProviderSearchResult};
use crate::state::AppState;

#[tauri::command]
pub async fn search_releases(
    state: State<'_, AppState>,
    provider: String,
    query: String,
    artist: Option<String>,
) -> Result<Vec<ProviderSearchResult>, String> {
    let provider_ref = {
        let registry = state.providers.lock().map_err(|e| e.to_string())?;
        registry
            .get_arc(&provider)
            .ok_or_else(|| format!("Provider '{}' not found", provider))?
    };

    provider_ref
        .search_release(&query, artist.as_deref())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn search_recordings(
    state: State<'_, AppState>,
    provider: String,
    query: String,
    artist: Option<String>,
) -> Result<Vec<ProviderSearchResult>, String> {
    let provider_ref = {
        let registry = state.providers.lock().map_err(|e| e.to_string())?;
        registry
            .get_arc(&provider)
            .ok_or_else(|| format!("Provider '{}' not found", provider))?
    };

    provider_ref
        .search_recording(&query, artist.as_deref())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn search_artists(
    state: State<'_, AppState>,
    provider: String,
    query: String,
) -> Result<Vec<ProviderSearchResult>, String> {
    let provider_ref = {
        let registry = state.providers.lock().map_err(|e| e.to_string())?;
        registry
            .get_arc(&provider)
            .ok_or_else(|| format!("Provider '{}' not found", provider))?
    };

    provider_ref
        .search_artist(&query)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_release_details(
    state: State<'_, AppState>,
    provider: String,
    release_id: String,
) -> Result<ProviderRelease, String> {
    let provider_ref = {
        let registry = state.providers.lock().map_err(|e| e.to_string())?;
        registry
            .get_arc(&provider)
            .ok_or_else(|| format!("Provider '{}' not found", provider))?
    };

    provider_ref
        .get_release(&release_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_providers(state: State<'_, AppState>) -> Result<Vec<ProviderInfo>, String> {
    let registry = state.providers.lock().map_err(|e| e.to_string())?;
    Ok(registry.list())
}
