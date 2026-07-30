use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::album::AlbumMetadata;
use super::artist::ArtistMetadata;
use super::track::TrackMetadata;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSearchResult {
    pub provider: String,
    pub result_type: SearchResultType,
    pub id: String,
    pub score: Option<f64>,
    pub title: String,
    pub artist: Option<String>,
    pub year: Option<i32>,
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchResultType {
    Release,
    Recording,
    Artist,
    ReleaseGroup,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderRelease {
    pub provider: String,
    pub id: String,
    pub album: AlbumMetadata,
    pub tracks: Vec<TrackMetadata>,
    pub artists: Vec<ArtistMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInfo {
    pub id: String,
    pub display_name: String,
    pub supported_types: Vec<String>,
}
