mod book;
mod commands;
mod dedup;
mod coverart;
mod db;
mod error;
mod merger;
mod metadata;
mod models;
mod nfo;
mod providers;
mod renamer;
mod scanner;
mod state;
mod transcode;

use std::sync::{Arc, Mutex};

use rusqlite::Connection;
use tauri::Manager;

use providers::musicbrainz::MusicBrainzProvider;
use providers::tmdb::TmdbProvider;
use providers::tvdb::TvdbProvider;
use providers::video::VideoProviderRegistry;
use providers::ProviderRegistry;
use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("failed to resolve app data dir");
            std::fs::create_dir_all(&app_data_dir).ok();

            let db_path = app_data_dir.join("notata.db");
            let conn =
                Connection::open(&db_path).expect("failed to open database");
            db::migrations::run_migrations(&conn).expect("failed to run migrations");

            let cover_art_cache_dir = app_data_dir
                .join("covers")
                .to_string_lossy()
                .to_string();
            std::fs::create_dir_all(&cover_art_cache_dir).ok();

            let fingerprint_cache_path = app_data_dir
                .join("fingerprint-cache.json")
                .to_string_lossy()
                .to_string();

            let mut registry = ProviderRegistry::new();
            registry.register(Arc::new(MusicBrainzProvider::new()));

            let mut video_registry = VideoProviderRegistry::new();
            video_registry.register(Arc::new(TmdbProvider::new()));
            video_registry.register(Arc::new(TvdbProvider::new()));

            app.manage(AppState {
                db: Mutex::new(conn),
                providers: Mutex::new(registry),
                video_providers: Mutex::new(video_registry),
                cover_art_cache_dir,
                fingerprint_cache_path,
            });

            // Providers start unconfigured; load any saved keys.
            commands::settings::apply_stored_api_keys(&app.state::<AppState>());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::library::scan_directory,
            commands::library::get_library_roots,
            commands::library::get_files_in_directory,
            commands::library::get_files_by_root,
            commands::library::get_directory_tree,
            commands::library::get_library_stats,
            commands::library::remove_library_root,
            commands::metadata::read_metadata,
            commands::metadata::read_metadata_batch,
            commands::metadata::write_metadata,
            commands::metadata::write_metadata_batch,
            commands::metadata::get_audio_properties,
            commands::metadata::get_embedded_cover_art,
            commands::search::search_releases,
            commands::search::search_recordings,
            commands::search::search_artists,
            commands::search::get_release_details,
            commands::search::list_providers,
            commands::coverart::fetch_provider_cover_art,
            commands::coverart::download_cover_art,
            commands::coverart::embed_cover_art,
            commands::coverart::remove_cover_art,
            commands::coverart::search_cover_art,
            commands::video::list_video_providers,
            commands::video::search_movies,
            commands::video::search_series,
            commands::video::get_movie_details,
            commands::video::get_series_details,
            commands::video::get_series_episodes,
            commands::settings::set_api_key,
            commands::settings::get_api_key_status,
            commands::settings::set_preference,
            commands::settings::get_preference,
            commands::renamer::list_rename_presets,
            commands::renamer::validate_rename_template,
            commands::renamer::preview_rename,
            commands::renamer::apply_rename,
            commands::dedup::find_duplicates,
            commands::dedup::resolve_duplicates,
            commands::dedup::read_audio_preview,
            commands::batch::preview_batch_edit,
            commands::batch::apply_batch_edit,
            commands::library::set_root_media_kind,
            commands::video::read_video_metadata,
            commands::video::get_video_properties,
            commands::video::get_video_artwork,
            commands::video::write_video_metadata,
            commands::video::write_video_metadata_batch,
            commands::video::save_video_poster,
            commands::video::apply_movie_to_metadata,
            commands::video::apply_episode_to_metadata,
            commands::book::read_book_metadata,
            commands::book::get_book_properties,
            commands::book::get_book_cover,
            commands::book::write_book_metadata,
            commands::book::write_book_metadata_batch,
            commands::book::write_book_cover,
            commands::fs::read_file_bytes,
            commands::video::get_provider_artwork,
            commands::updater::check_for_update,
            commands::updater::install_update,
            commands::updater::open_releases_page,
            commands::updater::restart_app,
            commands::updater::get_app_version,
            commands::transcode::list_transcode_formats,
            commands::transcode::check_ffmpeg_available,
            commands::transcode::preview_transcode,
            commands::transcode::transcode_files,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
