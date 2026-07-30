use rusqlite::Connection;
use std::sync::Mutex;

use crate::providers::video::VideoProviderRegistry;
use crate::providers::ProviderRegistry;

pub struct AppState {
    pub db: Mutex<Connection>,
    pub providers: Mutex<ProviderRegistry>,
    pub video_providers: Mutex<VideoProviderRegistry>,
    pub cover_art_cache_dir: String,
}

/// Preference keys for provider credentials.
pub const PREF_TMDB_API_KEY: &str = "tmdb_api_key";
pub const PREF_TVDB_API_KEY: &str = "tvdb_api_key";
