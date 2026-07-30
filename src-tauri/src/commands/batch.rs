use serde::{Deserialize, Serialize};
use tauri::State;

use crate::commands::metadata::stamp_modified;
use crate::metadata::{reader, writer};
use crate::models::track::TrackMetadata;
use crate::state::AppState;

/// What to do to one field across the selected files.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FieldOp {
    /// Overwrite with a fixed value.
    Set { value: String },
    /// Clear the field.
    Clear,
    /// Substring replace within the existing value.
    Replace { find: String, replace: String },
    /// Number the files in the order they were supplied, starting at `start`.
    Enumerate { start: u32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchEdit {
    /// Field name, matching the lowercase `TrackMetadata` keys.
    pub field: String,
    pub op: FieldOp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchPreviewEntry {
    pub path: String,
    pub field: String,
    pub before: String,
    pub after: String,
    pub changed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchResult {
    pub path: String,
    pub success: bool,
    pub error: Option<String>,
}

fn read_field(metadata: &TrackMetadata, field: &str) -> String {
    match field {
        "title" => metadata.title.clone().unwrap_or_default(),
        "artist" => metadata.artist.clone().unwrap_or_default(),
        "albumartist" => metadata.album_artist.clone().unwrap_or_default(),
        "album" => metadata.album.clone().unwrap_or_default(),
        "tracknumber" => metadata.track_number.map(|n| n.to_string()).unwrap_or_default(),
        "totaltracks" => metadata.total_tracks.map(|n| n.to_string()).unwrap_or_default(),
        "discnumber" => metadata.disc_number.map(|n| n.to_string()).unwrap_or_default(),
        "totaldiscs" => metadata.total_discs.map(|n| n.to_string()).unwrap_or_default(),
        "year" => metadata.year.map(|n| n.to_string()).unwrap_or_default(),
        "date" => metadata.date.clone().unwrap_or_default(),
        "comment" => metadata.comment.clone().unwrap_or_default(),
        "isrc" => metadata.isrc.clone().unwrap_or_default(),
        "genre" => metadata
            .genre
            .as_ref()
            .map(|g| g.join("; "))
            .unwrap_or_default(),
        "composer" => metadata
            .composer
            .as_ref()
            .map(|c| c.join("; "))
            .unwrap_or_default(),
        _ => String::new(),
    }
}

fn write_field(metadata: &mut TrackMetadata, field: &str, value: &str) -> Result<(), String> {
    let trimmed = value.trim();
    let text = if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    };

    // Numeric fields silently keep their old value on unparseable input rather
    // than writing a nonsense tag.
    let number = |v: &str| -> Option<u32> { v.trim().parse().ok() };

    match field {
        "title" => metadata.title = text,
        "artist" => metadata.artist = text,
        "albumartist" => metadata.album_artist = text,
        "album" => metadata.album = text,
        "date" => metadata.date = text,
        "comment" => metadata.comment = text,
        "isrc" => metadata.isrc = text,
        "tracknumber" => metadata.track_number = text.and_then(|v| number(&v)),
        "totaltracks" => metadata.total_tracks = text.and_then(|v| number(&v)),
        "discnumber" => metadata.disc_number = text.and_then(|v| number(&v)),
        "totaldiscs" => metadata.total_discs = text.and_then(|v| number(&v)),
        "year" => metadata.year = text.and_then(|v| v.trim().parse().ok()),
        "genre" => {
            metadata.genre = text.map(|v| {
                v.split(';')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
        }
        "composer" => {
            metadata.composer = text.map(|v| {
                v.split(';')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
        }
        other => return Err(format!("Unknown field '{}'", other)),
    }

    Ok(())
}

fn apply_op(current: &str, op: &FieldOp, index: usize) -> String {
    match op {
        FieldOp::Set { value } => value.clone(),
        FieldOp::Clear => String::new(),
        FieldOp::Replace { find, replace } => {
            if find.is_empty() {
                current.to_string()
            } else {
                current.replace(find.as_str(), replace)
            }
        }
        FieldOp::Enumerate { start } => (*start as usize + index).to_string(),
    }
}

/// Compute what a batch edit would change, without writing anything.
#[tauri::command]
pub fn preview_batch_edit(
    paths: Vec<String>,
    edits: Vec<BatchEdit>,
) -> Result<Vec<BatchPreviewEntry>, String> {
    let mut preview = Vec::new();

    for (index, path) in paths.iter().enumerate() {
        let metadata = reader::read_metadata(path).unwrap_or_default();

        for edit in &edits {
            let field = edit.field.to_lowercase();
            let before = read_field(&metadata, &field);
            let after = apply_op(&before, &edit.op, index);

            preview.push(BatchPreviewEntry {
                path: path.clone(),
                field: field.clone(),
                changed: before != after,
                before,
                after,
            });
        }
    }

    Ok(preview)
}

/// Apply a batch edit. Each file is written independently so one failure does
/// not abort the rest.
#[tauri::command]
pub fn apply_batch_edit(
    state: State<'_, AppState>,
    paths: Vec<String>,
    edits: Vec<BatchEdit>,
) -> Result<Vec<BatchResult>, String> {
    let mut results = Vec::new();

    for (index, path) in paths.iter().enumerate() {
        let mut metadata = match reader::read_metadata(path) {
            Ok(m) => m,
            Err(e) => {
                results.push(BatchResult {
                    path: path.clone(),
                    success: false,
                    error: Some(e.to_string()),
                });
                continue;
            }
        };

        let mut failure = None;
        for edit in &edits {
            let field = edit.field.to_lowercase();
            let before = read_field(&metadata, &field);
            let after = apply_op(&before, &edit.op, index);
            if let Err(e) = write_field(&mut metadata, &field, &after) {
                failure = Some(e);
                break;
            }
        }

        if let Some(error) = failure {
            results.push(BatchResult {
                path: path.clone(),
                success: false,
                error: Some(error),
            });
            continue;
        }

        match writer::write_metadata(path, &metadata) {
            Ok(()) => {
                stamp_modified(&state, path);
                results.push(BatchResult {
                    path: path.clone(),
                    success: true,
                    error: None,
                });
            }
            Err(e) => results.push(BatchResult {
                path: path.clone(),
                success: false,
                error: Some(e.to_string()),
            }),
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_overwrites_and_clear_empties() {
        assert_eq!(
            apply_op(
                "old",
                &FieldOp::Set {
                    value: "new".into()
                },
                0
            ),
            "new"
        );
        assert_eq!(apply_op("old", &FieldOp::Clear, 0), "");
    }

    #[test]
    fn replace_rewrites_substrings_and_ignores_an_empty_needle() {
        let op = FieldOp::Replace {
            find: "feat.".into(),
            replace: "ft.".into(),
        };
        assert_eq!(apply_op("Song feat. X", &op, 0), "Song ft. X");

        let noop = FieldOp::Replace {
            find: String::new(),
            replace: "x".into(),
        };
        assert_eq!(apply_op("Song", &noop, 0), "Song");
    }

    #[test]
    fn enumerate_numbers_by_position() {
        let op = FieldOp::Enumerate { start: 1 };
        assert_eq!(apply_op("", &op, 0), "1");
        assert_eq!(apply_op("", &op, 4), "5");
    }

    #[test]
    fn numeric_fields_reject_unparseable_values() {
        let mut m = TrackMetadata {
            track_number: Some(3),
            ..Default::default()
        };
        write_field(&mut m, "tracknumber", "not a number").unwrap();
        assert_eq!(m.track_number, None);

        write_field(&mut m, "tracknumber", "7").unwrap();
        assert_eq!(m.track_number, Some(7));
    }

    #[test]
    fn multi_value_fields_split_on_semicolons() {
        let mut m = TrackMetadata::default();
        write_field(&mut m, "genre", "Rock; Alternative ; ").unwrap();
        assert_eq!(
            m.genre,
            Some(vec!["Rock".to_string(), "Alternative".to_string()])
        );
        assert_eq!(read_field(&m, "genre"), "Rock; Alternative");
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let mut m = TrackMetadata::default();
        assert!(write_field(&mut m, "nonsense", "x").is_err());
    }
}
