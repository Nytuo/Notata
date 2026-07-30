use std::sync::Arc;

use tauri::State;

use crate::commands::metadata::stamp_modified;
use crate::metadata::video as video_meta;
use crate::models::video::{
    ActorCredit, EpisodeMetadata, MovieMetadata, RemoteArtwork, SeriesMetadata, VideoArtwork,
    VideoKind, VideoMetadata, VideoProperties, VideoProviderInfo, VideoSearchResult,
};
use crate::providers::video::VideoMetadataProvider;
use crate::state::AppState;

/// Resolve a provider and drop the registry lock before any `.await`.
fn provider_arc(
    state: &State<'_, AppState>,
    id: &str,
) -> Result<Arc<dyn VideoMetadataProvider>, String> {
    let registry = state.video_providers.lock().map_err(|e| e.to_string())?;
    registry
        .get_arc(id)
        .ok_or_else(|| format!("Video provider '{}' not found", id))
}

#[tauri::command]
pub fn list_video_providers(state: State<'_, AppState>) -> Result<Vec<VideoProviderInfo>, String> {
    let registry = state.video_providers.lock().map_err(|e| e.to_string())?;
    Ok(registry.list())
}

#[tauri::command]
pub async fn search_movies(
    state: State<'_, AppState>,
    provider: String,
    query: String,
    year: Option<i32>,
) -> Result<Vec<VideoSearchResult>, String> {
    let p = provider_arc(&state, &provider)?;
    p.search_movie(&query, year).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn search_series(
    state: State<'_, AppState>,
    provider: String,
    query: String,
    year: Option<i32>,
) -> Result<Vec<VideoSearchResult>, String> {
    let p = provider_arc(&state, &provider)?;
    p.search_series(&query, year).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_movie_details(
    state: State<'_, AppState>,
    provider: String,
    movie_id: String,
) -> Result<MovieMetadata, String> {
    let p = provider_arc(&state, &provider)?;
    p.get_movie(&movie_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_series_details(
    state: State<'_, AppState>,
    provider: String,
    series_id: String,
) -> Result<SeriesMetadata, String> {
    let p = provider_arc(&state, &provider)?;
    p.get_series(&series_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_series_episodes(
    state: State<'_, AppState>,
    provider: String,
    series_id: String,
    season: Option<u32>,
) -> Result<Vec<EpisodeMetadata>, String> {
    let p = provider_arc(&state, &provider)?;
    p.get_episodes(&series_id, season)
        .await
        .map_err(|e| e.to_string())
}

/// Every poster and backdrop a provider holds for this title, for the picker.
#[tauri::command]
pub async fn get_provider_artwork(
    state: State<'_, AppState>,
    provider: String,
    id: String,
    is_series: bool,
) -> Result<Vec<RemoteArtwork>, String> {
    let p = provider_arc(&state, &provider)?;
    p.get_artwork(&id, is_series)
        .await
        .map_err(|e| e.to_string())
}

// -------------------------------------------------------- file-level ------

#[tauri::command]
pub fn read_video_metadata(path: String) -> Result<VideoMetadata, String> {
    video_meta::read_video_metadata(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_video_properties(path: String) -> Result<VideoProperties, String> {
    video_meta::read_video_properties(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_video_artwork(path: String) -> Result<Vec<VideoArtwork>, String> {
    Ok(video_meta::find_local_artwork(&path))
}

/// Write metadata to the file's NFO sidecar. Returns the path written.
#[tauri::command]
pub fn write_video_metadata(
    state: State<'_, AppState>,
    path: String,
    metadata: VideoMetadata,
) -> Result<String, String> {
    let written = video_meta::write_video_metadata(&path, &metadata).map_err(|e| e.to_string())?;
    stamp_modified(&state, &path);
    Ok(written)
}

#[tauri::command]
pub fn save_video_poster(
    path: String,
    image_data: Vec<u8>,
    mime_type: String,
) -> Result<String, String> {
    video_meta::write_poster(&path, &image_data, &mime_type).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn write_video_metadata_batch(
    state: State<'_, AppState>,
    entries: Vec<(String, VideoMetadata)>,
) -> Result<Vec<(String, bool, String)>, String> {
    let mut results = Vec::new();
    for (path, metadata) in entries {
        match video_meta::write_video_metadata(&path, &metadata) {
            Ok(_) => {
                stamp_modified(&state, &path);
                results.push((path, true, String::new()));
            }
            Err(e) => results.push((path, false, e.to_string())),
        }
    }
    Ok(results)
}

/// Fold a provider movie record into a file's metadata.
///
/// Runs on the Rust side so the same merge rules apply everywhere: provider
/// values win for the fields it supplies, and everything else is preserved.
#[tauri::command]
pub fn apply_movie_to_metadata(
    current: VideoMetadata,
    movie: MovieMetadata,
) -> Result<VideoMetadata, String> {
    let mut next = current;

    next.kind = VideoKind::Movie;
    next.title = Some(movie.title);
    next.original_title = movie.original_title.or(next.original_title);
    next.year = movie.year.or(next.year);
    next.release_date = movie.release_date.or(next.release_date);
    next.tagline = movie.tagline.or(next.tagline);
    next.plot = movie.overview.or(next.plot);
    next.runtime_minutes = movie.runtime_minutes.or(next.runtime_minutes);
    next.rating = movie.rating.or(next.rating);

    if !movie.genres.is_empty() {
        next.genres = movie.genres;
    }
    if !movie.studios.is_empty() {
        next.studios = movie.studios;
    }
    if let Some(country) = movie.country {
        next.countries = vec![country];
    }
    if !movie.directors.is_empty() {
        next.directors = movie.directors;
    }
    if !movie.writers.is_empty() {
        next.writers = movie.writers;
    }
    if !movie.cast.is_empty() {
        next.actors = movie
            .cast
            .into_iter()
            .map(|name| ActorCredit {
                name,
                role: None,
                thumb: None,
            })
            .collect();
    }

    next.tmdb_id = movie.tmdb_id.or(next.tmdb_id);
    next.imdb_id = movie.imdb_id.or(next.imdb_id);

    Ok(next)
}

/// Fold a provider episode (plus its series) into a file's metadata.
#[tauri::command]
pub fn apply_episode_to_metadata(
    current: VideoMetadata,
    series: SeriesMetadata,
    episode: EpisodeMetadata,
) -> Result<VideoMetadata, String> {
    let mut next = current;

    next.kind = VideoKind::Episode;
    next.title = Some(episode.title);
    next.show_title = Some(series.title);
    next.season = Some(episode.season);
    next.episode = Some(episode.episode);
    next.aired = episode.air_date.clone().or(next.aired);
    next.release_date = episode.air_date.or(next.release_date);
    next.plot = episode.overview.or(next.plot);
    next.runtime_minutes = episode.runtime_minutes.or(next.runtime_minutes);
    next.rating = episode.rating.or(next.rating);

    if !series.genres.is_empty() {
        next.genres = series.genres;
    }
    if let Some(network) = series.network {
        next.studios = vec![network];
    }

    next.tmdb_id = episode.tmdb_id.or(next.tmdb_id);
    next.tvdb_id = episode.tvdb_id.or(series.tvdb_id).or(next.tvdb_id);
    next.imdb_id = series.imdb_id.or(next.imdb_id);

    Ok(next)
}
