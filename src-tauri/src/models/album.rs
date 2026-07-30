use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumMetadata {
    pub title: String,
    pub artist: String,
    pub year: Option<i32>,
    pub release_date: Option<String>,
    pub genre: Option<Vec<String>>,
    pub label: Option<String>,
    pub catalog_number: Option<String>,
    pub barcode: Option<String>,
    pub release_type: Option<String>,
    pub release_country: Option<String>,
    pub musicbrainz_release_id: Option<String>,
    pub musicbrainz_release_group_id: Option<String>,
    pub total_tracks: Option<u32>,
    pub total_discs: Option<u32>,
    pub cover_art: Vec<CoverArt>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoverArt {
    pub id: String,
    pub art_type: CoverArtType,
    pub source: ArtSource,
    pub mime_type: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub data_path: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoverArtType {
    Front,
    Back,
    Disc,
    Booklet,
    Artist,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtSource {
    Embedded,
    LocalFile,
    CoverArtArchive,
    FanartTv,
    Itunes,
    Deezer,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoverArtData {
    pub data: String,
    pub mime_type: String,
    pub art_type: CoverArtType,
    pub width: Option<u32>,
    pub height: Option<u32>,
}
