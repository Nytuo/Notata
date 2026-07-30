use rusqlite::{params, Connection};

use crate::error::Result;
use crate::models::media_file::{LibraryRoot, MediaFile, MediaKind};
use crate::models::track::TrackMetadata;

pub fn insert_library_root(conn: &Connection, root: &LibraryRoot) -> Result<()> {
    // Keep scan history if the root already exists — re-adding a folder should
    // not make every file look new again.
    conn.execute(
        "INSERT INTO library_roots (id, path, label, added_at, last_scan, previous_scan, media_kind)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(path) DO UPDATE SET
            label = excluded.label,
            media_kind = excluded.media_kind",
        params![
            root.id,
            root.path,
            root.label,
            root.added_at,
            root.last_scan,
            root.previous_scan,
            root.media_kind.as_str()
        ],
    )?;
    Ok(())
}

pub fn get_library_root_by_path(conn: &Connection, path: &str) -> Result<Option<LibraryRoot>> {
    let mut stmt = conn.prepare(
        "SELECT id, path, label, added_at, last_scan, previous_scan, media_kind FROM library_roots WHERE path = ?1",
    )?;
    let mut rows = stmt.query(params![path])?;
    if let Some(row) = rows.next()? {
        Ok(Some(LibraryRoot {
            id: row.get(0)?,
            path: row.get(1)?,
            label: row.get(2)?,
            added_at: row.get(3)?,
            last_scan: row.get(4)?,
            previous_scan: row.get(5)?,
            media_kind: MediaKind::from_str(&row.get::<_, String>(6)?),
        }))
    } else {
        Ok(None)
    }
}

pub fn get_library_roots(conn: &Connection) -> Result<Vec<LibraryRoot>> {
    let mut stmt = conn.prepare(
        "SELECT id, path, label, added_at, last_scan, previous_scan, media_kind FROM library_roots ORDER BY path",
    )?;
    let roots = stmt
        .query_map([], |row| {
            Ok(LibraryRoot {
                id: row.get(0)?,
                path: row.get(1)?,
                label: row.get(2)?,
                added_at: row.get(3)?,
                last_scan: row.get(4)?,
                previous_scan: row.get(5)?,
                media_kind: MediaKind::from_str(&row.get::<_, String>(6)?),
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(roots)
}

pub fn set_preference(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO preferences (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

pub fn get_preference(conn: &Connection, key: &str) -> Result<Option<String>> {
    let mut stmt = conn.prepare("SELECT value FROM preferences WHERE key = ?1")?;
    let mut rows = stmt.query(params![key])?;
    if let Some(row) = rows.next()? {
        Ok(Some(row.get(0)?))
    } else {
        Ok(None)
    }
}

pub fn set_root_media_kind(conn: &Connection, id: &str, kind: MediaKind) -> Result<()> {
    conn.execute(
        "UPDATE library_roots SET media_kind = ?1 WHERE id = ?2",
        params![kind.as_str(), id],
    )?;
    Ok(())
}

pub fn delete_library_root(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM library_roots WHERE id = ?1", params![id])?;
    Ok(())
}

/// Insert a freshly-scanned file, or refresh an existing row for the same path.
///
/// On conflict the original `id` and `first_seen_at` are kept so cached
/// metadata stays attached and the "new since last scan" flag remains accurate.
pub fn upsert_media_file(conn: &Connection, file: &MediaFile, library_root_id: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO media_files (id, library_root_id, path, file_name, parent_dir, media_type, audio_format, file_size, modified_at, scanned_at, has_cover_art, duration_ms, bitrate_kbps, sample_rate_hz, channels, first_seen_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
         ON CONFLICT(path) DO UPDATE SET
            library_root_id = excluded.library_root_id,
            file_name       = excluded.file_name,
            parent_dir      = excluded.parent_dir,
            media_type      = excluded.media_type,
            audio_format    = excluded.audio_format,
            file_size       = excluded.file_size,
            modified_at     = excluded.modified_at,
            scanned_at      = excluded.scanned_at",
        params![
            file.id,
            library_root_id,
            file.path,
            file.file_name,
            file.parent_dir,
            file.media_type.as_str(),
            file.audio_format.as_ref().map(|f| f.as_str().to_string()),
            file.file_size,
            file.modified_at,
            file.scanned_at,
            file.has_cover_art as i32,
            file.duration_ms.map(|d| d as i64),
            file.bitrate_kbps,
            file.sample_rate_hz,
            file.channels,
            file.first_seen_at,
        ],
    )?;
    Ok(())
}

/// Drop rows for files that disappeared — anything in this root the current
/// scan did not touch.
pub fn delete_files_not_seen_in_scan(
    conn: &Connection,
    root_id: &str,
    scan_started_at: i64,
) -> Result<usize> {
    let count = conn.execute(
        "DELETE FROM media_files WHERE library_root_id = ?1 AND scanned_at < ?2",
        params![root_id, scan_started_at],
    )?;
    Ok(count)
}

pub fn delete_media_file(conn: &Connection, path: &str) -> Result<()> {
    conn.execute("DELETE FROM media_files WHERE path = ?1", params![path])?;
    Ok(())
}

/// Re-point an indexed file after a rename, keeping its id and history.
pub fn update_file_path(conn: &Connection, old_path: &str, new_path: &str) -> Result<()> {
    let path = std::path::Path::new(new_path);
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_string();
    let parent_dir = path
        .parent()
        .and_then(|p| p.to_str())
        .unwrap_or_default()
        .to_string();

    conn.execute(
        "UPDATE media_files SET path = ?1, file_name = ?2, parent_dir = ?3 WHERE path = ?4",
        params![new_path, file_name, parent_dir, old_path],
    )?;
    Ok(())
}

/// Record that Notata wrote tags to this file, so the UI can flag it as
/// modified during the current session.
pub fn mark_file_modified(conn: &Connection, path: &str, when: i64) -> Result<()> {
    conn.execute(
        "UPDATE media_files SET last_modified_by_app = ?1 WHERE path = ?2",
        params![when, path],
    )?;
    Ok(())
}

const MEDIA_FILE_COLUMNS: &str = "m.id, m.path, m.file_name, m.parent_dir, m.media_type, \
     m.audio_format, m.file_size, m.modified_at, m.scanned_at, m.has_cover_art, m.duration_ms, \
     m.bitrate_kbps, m.sample_rate_hz, m.channels, m.first_seen_at, m.last_modified_by_app, \
     CASE WHEN r.previous_scan IS NOT NULL AND m.first_seen_at > r.previous_scan THEN 1 ELSE 0 END AS is_new";

fn map_media_file(row: &rusqlite::Row) -> rusqlite::Result<MediaFile> {
    Ok(MediaFile {
        id: row.get(0)?,
        path: row.get(1)?,
        file_name: row.get(2)?,
        parent_dir: row.get(3)?,
        media_type: crate::models::media_file::MediaType::from_str(&row.get::<_, String>(4)?),
        audio_format: row
            .get::<_, Option<String>>(5)?
            .map(|s| crate::models::media_file::AudioFormat::from_str(&s)),
        file_size: row.get(6)?,
        modified_at: row.get(7)?,
        scanned_at: row.get(8)?,
        has_cover_art: row.get::<_, i32>(9)? != 0,
        duration_ms: row.get::<_, Option<i64>>(10)?.map(|d| d as u64),
        bitrate_kbps: row.get(11)?,
        sample_rate_hz: row.get(12)?,
        channels: row.get(13)?,
        first_seen_at: row.get(14)?,
        last_modified_by_app: row.get(15)?,
        is_new: row.get::<_, i32>(16)? != 0,
    })
}

pub fn get_files_in_directory(conn: &Connection, parent_dir: &str) -> Result<Vec<MediaFile>> {
    let sql = format!(
        "SELECT {} FROM media_files m JOIN library_roots r ON r.id = m.library_root_id
         WHERE m.parent_dir = ?1 ORDER BY m.file_name",
        MEDIA_FILE_COLUMNS
    );
    let mut stmt = conn.prepare(&sql)?;
    let files = stmt
        .query_map(params![parent_dir], map_media_file)?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(files)
}

pub fn get_files_by_root(conn: &Connection, root_id: &str) -> Result<Vec<MediaFile>> {
    let sql = format!(
        "SELECT {} FROM media_files m JOIN library_roots r ON r.id = m.library_root_id
         WHERE m.library_root_id = ?1 ORDER BY m.path",
        MEDIA_FILE_COLUMNS
    );
    let mut stmt = conn.prepare(&sql)?;
    let files = stmt
        .query_map(params![root_id], map_media_file)?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(files)
}

/// Every audio file in the library, used by duplicate detection.
pub fn get_all_audio_files(conn: &Connection) -> Result<Vec<MediaFile>> {
    let sql = format!(
        "SELECT {} FROM media_files m JOIN library_roots r ON r.id = m.library_root_id
         WHERE m.media_type = 'audio' ORDER BY m.path",
        MEDIA_FILE_COLUMNS
    );
    let mut stmt = conn.prepare(&sql)?;
    let files = stmt
        .query_map([], map_media_file)?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(files)
}

pub fn delete_files_by_root(conn: &Connection, root_id: &str) -> Result<usize> {
    let count = conn.execute(
        "DELETE FROM media_files WHERE library_root_id = ?1",
        params![root_id],
    )?;
    Ok(count)
}

pub fn cache_file_metadata(
    conn: &Connection,
    file_id: &str,
    metadata: &TrackMetadata,
) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    let genre_json = metadata.genre.as_ref().map(|g| serde_json::to_string(g).unwrap_or_default());
    let composer_json = metadata.composer.as_ref().map(|c| serde_json::to_string(c).unwrap_or_default());
    let custom_json = if metadata.custom_tags.is_empty() {
        None
    } else {
        Some(serde_json::to_string(&metadata.custom_tags).unwrap_or_default())
    };

    conn.execute(
        "INSERT OR REPLACE INTO file_metadata (file_id, title, artist, album_artist, album, track_number, total_tracks, disc_number, total_discs, year, date, genre, composer, comment, isrc, mb_track_id, mb_release_id, mb_artist_id, mb_release_group_id, custom_tags, cached_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
        params![
            file_id,
            metadata.title,
            metadata.artist,
            metadata.album_artist,
            metadata.album,
            metadata.track_number,
            metadata.total_tracks,
            metadata.disc_number,
            metadata.total_discs,
            metadata.year,
            metadata.date,
            genre_json,
            composer_json,
            metadata.comment,
            metadata.isrc,
            metadata.musicbrainz_track_id,
            metadata.musicbrainz_release_id,
            metadata.musicbrainz_artist_id,
            metadata.musicbrainz_release_group_id,
            custom_json,
            now,
        ],
    )?;
    Ok(())
}

pub fn get_cached_metadata(conn: &Connection, file_id: &str) -> Result<Option<TrackMetadata>> {
    let mut stmt = conn.prepare(
        "SELECT title, artist, album_artist, album, track_number, total_tracks, disc_number, total_discs, year, date, genre, composer, comment, isrc, mb_track_id, mb_release_id, mb_artist_id, mb_release_group_id, custom_tags FROM file_metadata WHERE file_id = ?1",
    )?;

    let mut rows = stmt.query(params![file_id])?;
    if let Some(row) = rows.next()? {
        let genre: Option<Vec<String>> = row
            .get::<_, Option<String>>(10)?
            .and_then(|s| serde_json::from_str(&s).ok());
        let composer: Option<Vec<String>> = row
            .get::<_, Option<String>>(11)?
            .and_then(|s| serde_json::from_str(&s).ok());
        let custom_tags = row
            .get::<_, Option<String>>(18)?
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        Ok(Some(TrackMetadata {
            title: row.get(0)?,
            artist: row.get(1)?,
            album_artist: row.get(2)?,
            album: row.get(3)?,
            track_number: row.get(4)?,
            total_tracks: row.get(5)?,
            disc_number: row.get(6)?,
            total_discs: row.get(7)?,
            year: row.get(8)?,
            date: row.get(9)?,
            genre,
            composer,
            comment: row.get(12)?,
            lyrics: None,
            isrc: row.get(13)?,
            musicbrainz_track_id: row.get(14)?,
            musicbrainz_release_id: row.get(15)?,
            musicbrainz_artist_id: row.get(16)?,
            musicbrainz_release_group_id: row.get(17)?,
            custom_tags,
        }))
    } else {
        Ok(None)
    }
}
