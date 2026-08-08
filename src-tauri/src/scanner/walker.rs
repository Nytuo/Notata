use std::path::Path;
use std::time::Instant;

use tauri::{AppHandle, Emitter};
use walkdir::WalkDir;

use crate::db::queries;
use crate::error::Result;
use crate::metadata::reader::read_audio_properties;
use crate::models::media_file::{DirectoryNode, MediaFile, MediaType, ScanProgress, ScanResult};
use crate::scanner::mime::classify_extension;
use crate::state::AppState;

pub fn scan_directory(
    app: &AppHandle,
    state: &AppState,
    root_path: &str,
    root_id: &str,
) -> Result<ScanResult> {
    let start = Instant::now();
    let mut total_files = 0usize;
    let mut audio_files = 0usize;
    let mut skipped = 0usize;

    let conn = state.db.lock().map_err(|e| crate::error::NotataError::Custom(e.to_string()))?;

    // Rotate scan history before indexing: files first seen after
    // `previous_scan` are what the UI reports as new.
    let scan_started_at = chrono::Utc::now().timestamp();
    conn.execute(
        "UPDATE library_roots SET previous_scan = last_scan WHERE id = ?1",
        rusqlite::params![root_id],
    )?;

    for entry in WalkDir::new(root_path)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();

        // macOS AppleDouble sidecar files (e.g. "._Song.mp3") mirror the real
        // file's extension, so they'd otherwise be indexed as media.
        if path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with("._"))
        {
            skipped += 1;
            continue;
        }

        let ext = match path.extension().and_then(|e| e.to_str()) {
            Some(e) => e,
            None => {
                skipped += 1;
                continue;
            }
        };

        let (media_type, audio_format) = classify_extension(ext);

        // Sidecars (artwork, NFO, cue) are found on demand from disk; indexing
        // them would inflate counts and list rows with nothing to edit.
        if !crate::scanner::mime::is_indexable(&media_type) {
            skipped += 1;
            continue;
        }

        total_files += 1;

        if matches!(media_type, crate::models::media_file::MediaType::Audio) {
            audio_files += 1;
        }

        let metadata = entry.metadata().ok();
        let file_size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
        let modified_at = metadata
            .as_ref()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        let parent_dir = path
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or("")
            .to_string();

        let path_str = path.to_string_lossy().to_string();

        // Reading properties opens and parses the container header (not a
        // full decode), so this is cheap enough to do for every audio file
        // during a scan. A read failure just leaves these unset rather than
        // aborting the file's indexing.
        let audio_properties = if matches!(media_type, MediaType::Audio) {
            read_audio_properties(&path_str).ok()
        } else {
            None
        };

        let media_file = MediaFile {
            id: uuid::Uuid::new_v4().to_string(),
            path: path_str,
            file_name,
            parent_dir,
            media_type,
            audio_format,
            file_size,
            modified_at,
            scanned_at: scan_started_at,
            has_cover_art: false,
            duration_ms: audio_properties.as_ref().map(|p| p.duration_ms),
            bitrate_kbps: audio_properties.as_ref().map(|p| p.bitrate_kbps),
            sample_rate_hz: audio_properties.as_ref().map(|p| p.sample_rate_hz),
            channels: audio_properties.as_ref().map(|p| p.channels),
            // Only applied when the row is new; the upsert keeps the original
            // value for paths already indexed.
            first_seen_at: scan_started_at,
            last_modified_by_app: None,
            is_new: false,
        };

        queries::upsert_media_file(&conn, &media_file, root_id)?;

        if total_files % 50 == 0 {
            let _ = app.emit(
                "scan:progress",
                ScanProgress {
                    scanned: total_files,
                    current_file: media_file.path.clone(),
                },
            );
        }
    }

    queries::delete_files_not_seen_in_scan(&conn, root_id, scan_started_at)?;

    conn.execute(
        "UPDATE library_roots SET last_scan = ?1 WHERE id = ?2",
        rusqlite::params![scan_started_at, root_id],
    )?;

    let duration_ms = start.elapsed().as_millis() as u64;

    Ok(ScanResult {
        total_files,
        audio_files,
        skipped,
        duration_ms,
    })
}

pub fn build_directory_tree(root_path: &str) -> Result<DirectoryNode> {
    let root = Path::new(root_path);
    let name = root
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(root_path)
        .to_string();

    fn build_node(path: &Path) -> DirectoryNode {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        let mut children = Vec::new();
        let mut file_count = 0;

        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.filter_map(|e| e.ok()) {
                let entry_path = entry.path();
                if entry_path.is_dir() {
                    children.push(build_node(&entry_path));
                } else if entry_path.is_file() {
                    let is_apple_double = entry_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .is_some_and(|n| n.starts_with("._"));
                    if !is_apple_double {
                        file_count += 1;
                    }
                }
            }
        }

        children.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

        DirectoryNode {
            path: path.to_string_lossy().to_string(),
            name,
            children,
            file_count,
        }
    }

    Ok(build_node(root))
}
