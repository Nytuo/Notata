use rusqlite::Connection;

use crate::error::Result;

pub fn run_migrations(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS library_roots (
            id          TEXT PRIMARY KEY,
            path        TEXT NOT NULL UNIQUE,
            label       TEXT,
            added_at    INTEGER NOT NULL,
            last_scan   INTEGER
        );

        CREATE TABLE IF NOT EXISTS media_files (
            id              TEXT PRIMARY KEY,
            library_root_id TEXT NOT NULL REFERENCES library_roots(id) ON DELETE CASCADE,
            path            TEXT NOT NULL UNIQUE,
            file_name       TEXT NOT NULL,
            parent_dir      TEXT NOT NULL,
            media_type      TEXT NOT NULL,
            audio_format    TEXT,
            file_size       INTEGER NOT NULL,
            modified_at     INTEGER NOT NULL,
            scanned_at      INTEGER NOT NULL,
            has_cover_art   INTEGER NOT NULL DEFAULT 0,
            duration_ms     INTEGER,
            bitrate_kbps    INTEGER,
            sample_rate_hz  INTEGER,
            channels        INTEGER
        );

        CREATE INDEX IF NOT EXISTS idx_media_files_parent_dir ON media_files(parent_dir);
        CREATE INDEX IF NOT EXISTS idx_media_files_media_type ON media_files(media_type);
        CREATE INDEX IF NOT EXISTS idx_media_files_library_root ON media_files(library_root_id);

        CREATE TABLE IF NOT EXISTS file_metadata (
            file_id             TEXT PRIMARY KEY REFERENCES media_files(id) ON DELETE CASCADE,
            title               TEXT,
            artist              TEXT,
            album_artist        TEXT,
            album               TEXT,
            track_number        INTEGER,
            total_tracks        INTEGER,
            disc_number         INTEGER,
            total_discs         INTEGER,
            year                INTEGER,
            date                TEXT,
            genre               TEXT,
            composer            TEXT,
            comment             TEXT,
            isrc                TEXT,
            mb_track_id         TEXT,
            mb_release_id       TEXT,
            mb_artist_id        TEXT,
            mb_release_group_id TEXT,
            custom_tags         TEXT,
            cached_at           INTEGER NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_file_metadata_artist ON file_metadata(artist);
        CREATE INDEX IF NOT EXISTS idx_file_metadata_album ON file_metadata(album);
        CREATE INDEX IF NOT EXISTS idx_file_metadata_title ON file_metadata(title);

        CREATE TABLE IF NOT EXISTS search_cache (
            id              TEXT PRIMARY KEY,
            provider        TEXT NOT NULL,
            query_type      TEXT NOT NULL,
            query_text      TEXT NOT NULL,
            query_artist    TEXT,
            results_json    TEXT NOT NULL,
            fetched_at      INTEGER NOT NULL,
            UNIQUE(provider, query_type, query_text, query_artist)
        );

        CREATE TABLE IF NOT EXISTS cover_art_cache (
            id              TEXT PRIMARY KEY,
            source          TEXT NOT NULL,
            source_id       TEXT NOT NULL,
            art_type        TEXT NOT NULL,
            mime_type       TEXT NOT NULL,
            width           INTEGER,
            height          INTEGER,
            file_path       TEXT NOT NULL,
            fetched_at      INTEGER NOT NULL,
            UNIQUE(source, source_id, art_type)
        );

        CREATE TABLE IF NOT EXISTS preferences (
            key             TEXT PRIMARY KEY,
            value           TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS duplicate_ignores (
            id              TEXT PRIMARY KEY,
            group_key       TEXT NOT NULL UNIQUE,
            created_at      INTEGER NOT NULL
        );

        PRAGMA journal_mode=WAL;
        PRAGMA foreign_keys=ON;
        ",
    )?;

    // Columns added after the initial schema shipped. `ALTER TABLE` has no
    // `IF NOT EXISTS`, so check the table shape before adding each one.
    add_column_if_missing(
        conn,
        "media_files",
        "first_seen_at",
        "INTEGER NOT NULL DEFAULT 0",
    )?;
    add_column_if_missing(conn, "media_files", "last_modified_by_app", "INTEGER")?;
    // Timestamp of the scan before `last_scan`; a file is "new" when it was
    // first seen after that point.
    add_column_if_missing(conn, "library_roots", "previous_scan", "INTEGER")?;
    // Existing roots predate the movies/series workflow, so default to music.
    add_column_if_missing(
        conn,
        "library_roots",
        "media_kind",
        "TEXT NOT NULL DEFAULT 'music'",
    )?;

    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_media_files_first_seen ON media_files(first_seen_at);",
    )?;

    Ok(())
}

fn add_column_if_missing(
    conn: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> Result<()> {
    let exists: bool = conn
        .prepare(&format!("PRAGMA table_info({})", table))?
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<std::result::Result<Vec<_>, _>>()?
        .iter()
        .any(|name| name == column);

    if !exists {
        conn.execute_batch(&format!(
            "ALTER TABLE {} ADD COLUMN {} {};",
            table, column, definition
        ))?;
    }

    Ok(())
}
