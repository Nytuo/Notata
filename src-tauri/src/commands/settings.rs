use serde::{Deserialize, Serialize};
use tauri::State;

use crate::db::queries;
use crate::state::{AppState, PREF_TMDB_API_KEY, PREF_TVDB_API_KEY};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeyStatus {
    pub tmdb_configured: bool,
    pub tvdb_configured: bool,
}

/// Push stored keys into the live provider instances. Called at startup and
/// after any key change.
pub fn apply_stored_api_keys(state: &AppState) {
    let (tmdb, tvdb) = {
        let conn = match state.db.lock() {
            Ok(c) => c,
            Err(e) => {
                log::warn!("Could not read API keys: {}", e);
                return;
            }
        };
        (
            queries::get_preference(&conn, PREF_TMDB_API_KEY).ok().flatten(),
            queries::get_preference(&conn, PREF_TVDB_API_KEY).ok().flatten(),
        )
    };

    if let Ok(registry) = state.video_providers.lock() {
        if let Some(p) = registry.get_arc("tmdb") {
            p.configure(tmdb);
        }
        if let Some(p) = registry.get_arc("tvdb") {
            p.configure(tvdb);
        }
    }
}

#[tauri::command]
pub fn set_api_key(
    state: State<'_, AppState>,
    provider: String,
    api_key: String,
) -> Result<(), String> {
    let key = match provider.as_str() {
        "tmdb" => PREF_TMDB_API_KEY,
        "tvdb" => PREF_TVDB_API_KEY,
        other => return Err(format!("Unknown provider '{}'", other)),
    };

    {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        queries::set_preference(&conn, key, api_key.trim()).map_err(|e| e.to_string())?;
    }

    apply_stored_api_keys(&state);
    Ok(())
}

/// Reports only whether a key is present — the values themselves are never
/// returned to the frontend.
#[tauri::command]
pub fn get_api_key_status(state: State<'_, AppState>) -> Result<ApiKeyStatus, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let has = |k: &str| {
        queries::get_preference(&conn, k)
            .ok()
            .flatten()
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false)
    };

    Ok(ApiKeyStatus {
        tmdb_configured: has(PREF_TMDB_API_KEY),
        tvdb_configured: has(PREF_TVDB_API_KEY),
    })
}

#[tauri::command]
pub fn set_preference(
    state: State<'_, AppState>,
    key: String,
    value: String,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    queries::set_preference(&conn, &key, &value).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_preference(
    state: State<'_, AppState>,
    key: String,
) -> Result<Option<String>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    queries::get_preference(&conn, &key).map_err(|e| e.to_string())
}
