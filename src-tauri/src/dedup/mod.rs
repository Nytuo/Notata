pub mod cache;
pub mod detector;
pub mod fingerprint;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DuplicateMode {
    /// Byte-identical files.
    Exact,
    /// Same recording by tags, regardless of encoding.
    Fuzzy,
    /// Same recording by audio content (Chromaprint-style fingerprint),
    /// regardless of tags, container, or encoding.
    Acoustic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateFile {
    pub path: String,
    pub file_name: String,
    pub file_size: u64,
    pub duration_ms: Option<u64>,
    pub bitrate_kbps: Option<u32>,
    pub format: Option<String>,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    /// Similarity to the group's reference file, 0.0–1.0. Always 1.0 for exact.
    pub score: f64,
    /// The copy Notata suggests keeping — highest bitrate, then largest file.
    pub recommended_keep: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateGroup {
    pub id: String,
    pub mode: DuplicateMode,
    /// Human-readable explanation of why these were grouped.
    pub reason: String,
    pub files: Vec<DuplicateFile>,
    pub wasted_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateReport {
    pub groups: Vec<DuplicateGroup>,
    pub total_groups: usize,
    pub total_files: usize,
    pub reclaimable_bytes: u64,
    /// Files the scan could not evaluate, e.g. because acoustic mode
    /// failed to decode/fingerprint them — surfaced so a "no duplicates"
    /// result can be told apart from files that were silently skipped.
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DedupProgress {
    pub scanned: usize,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveOutcome {
    pub path: String,
    pub success: bool,
    pub moved_to: Option<String>,
    pub error: Option<String>,
}
