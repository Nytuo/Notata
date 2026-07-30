pub mod parser;
pub mod writer;

use std::path::{Path, PathBuf};

/// Locate the NFO sidecar for a video file, following the conventions Kodi,
/// Jellyfin, and Plex all read.
///
/// Checked in order:
/// 1. `<video basename>.nfo` — the per-file form, used for movies and episodes
/// 2. `movie.nfo` in the same folder — the "one movie per folder" form
/// 3. `tvshow.nfo` in the same folder — series-level data
pub fn find_nfo_path(video_path: &Path) -> Option<PathBuf> {
    let sidecar = video_path.with_extension("nfo");
    if sidecar.is_file() {
        return Some(sidecar);
    }

    let parent = video_path.parent()?;

    for name in ["movie.nfo", "tvshow.nfo"] {
        let candidate = parent.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    None
}

/// Where a new NFO should be written for this video file.
pub fn default_nfo_path(video_path: &Path) -> PathBuf {
    video_path.with_extension("nfo")
}
