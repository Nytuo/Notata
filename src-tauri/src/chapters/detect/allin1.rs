use std::path::Path;
use std::process::Stdio;

use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use crate::db::queries;
use crate::error::{NotataError, Result};
use crate::state::{AppState, PREF_ALLIN1_PATH};

use super::super::Chapter;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Allin1Status {
    pub available: bool,
    pub found: bool,
    pub path: String,
    pub error: Option<String>,
}

pub fn resolve_allin1_path(state: &AppState) -> String {
    let custom = state
        .db
        .lock()
        .ok()
        .and_then(|conn| queries::get_preference(&conn, PREF_ALLIN1_PATH).ok().flatten())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    custom.unwrap_or_else(|| "allin1".to_string())
}

pub async fn check_available(allin1_path: &str) -> Allin1Status {
    let output = Command::new(allin1_path).arg("--help").output().await;
    match output {
        Ok(out) if out.status.success() => Allin1Status {
            available: true,
            found: true,
            path: allin1_path.to_string(),
            error: None,
        },
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            let tail: String = stderr.lines().rev().take(5).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\n");
            Allin1Status {
                available: false,
                found: true,
                path: allin1_path.to_string(),
                error: Some(if tail.trim().is_empty() {
                    format!("exited with status {}", out.status)
                } else {
                    tail
                }),
            }
        }
        Err(e) => Allin1Status {
            available: false,
            found: false,
            path: allin1_path.to_string(),
            error: Some(e.to_string()),
        },
    }
}

#[derive(Deserialize)]
struct RawSegment {
    start: f64,
    end: f64,
    label: String,
}

#[derive(Deserialize)]
struct RawResult {
    segments: Vec<RawSegment>,
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

pub async fn detect(
    _app: &AppHandle,
    allin1_path: &str,
    audio_path: &str,
    mut on_progress: impl FnMut(&str, f64),
) -> Result<Vec<Chapter>> {
    on_progress("starting allin1", 0.0);

    let out_dir = std::env::temp_dir().join(format!("notata-allin1-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&out_dir)?;

    let stem = Path::new(audio_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("track")
        .to_string();

    let mut child = Command::new(allin1_path)
        .arg(audio_path)
        .args(["--out-dir", &out_dir.to_string_lossy()])
        .args(["--model", "harmonix-all"])
        .arg("--overwrite")
        .arg("--no-multiprocess")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| NotataError::Custom(format!("Could not start allin1: {e}")))?;

    if let Some(stderr) = child.stderr.take() {
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let line = line.trim();
            if !line.is_empty() {
                on_progress(line, 0.5);
            }
        }
    }

    let status = child
        .wait()
        .await
        .map_err(|e| NotataError::Custom(format!("allin1 failed to run: {e}")))?;

    if !status.success() {
        let _ = std::fs::remove_dir_all(&out_dir);
        return Err(NotataError::Custom(
            "allin1 exited with an error — check that it's installed correctly (`pip install allin1`)"
                .to_string(),
        ));
    }

    let json_path = out_dir.join(format!("{stem}.json"));
    let data = std::fs::read_to_string(&json_path).map_err(|e| {
        NotataError::Custom(format!("Could not read allin1's output ({json_path:?}): {e}"))
    })?;
    let _ = std::fs::remove_dir_all(&out_dir);

    let parsed: RawResult = serde_json::from_str(&data)?;

    on_progress("done", 1.0);

    Ok(parsed
        .segments
        .into_iter()
        .enumerate()
        .map(|(i, seg)| Chapter {
            id: format!("chp{}", i + 1),
            title: capitalize(&seg.label),
            start_ms: (seg.start * 1000.0).round() as u64,
            end_ms: (seg.end * 1000.0).round() as u64,
        })
        .collect())
}
