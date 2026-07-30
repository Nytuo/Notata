use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BookKind {
    /// CBZ/CBR archive carrying ComicInfo.xml.
    Comic,
    /// EPUB carrying an OPF package document.
    Ebook,
}

impl Default for BookKind {
    fn default() -> Self {
        Self::Comic
    }
}

/// Where the metadata came from.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BookMetadataSource {
    /// ComicInfo.xml inside the archive.
    ComicInfo,
    /// OPF package document inside the EPUB.
    Opf,
    /// Nothing embedded; values derived from the filename.
    Filename,
    None,
}

impl Default for BookMetadataSource {
    fn default() -> Self {
        Self::None
    }
}

/// Editable metadata for a comic issue or an ebook.
///
/// One model covers both because the useful fields overlap heavily; the
/// writer maps them onto whichever schema the container actually uses.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookMetadata {
    #[serde(default)]
    pub kind: BookKind,

    pub title: Option<String>,
    pub series: Option<String>,
    /// Issue number — a string because comics use "1.5", "0", "Annual 2".
    pub number: Option<String>,
    /// Total issues in the series, when the file records it.
    pub count: Option<u32>,
    pub volume: Option<u32>,
    pub summary: Option<String>,

    pub year: Option<i32>,
    pub month: Option<u32>,
    pub day: Option<u32>,

    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub pencillers: Vec<String>,
    #[serde(default)]
    pub inkers: Vec<String>,
    #[serde(default)]
    pub colorists: Vec<String>,
    #[serde(default)]
    pub letterers: Vec<String>,
    #[serde(default)]
    pub cover_artists: Vec<String>,
    #[serde(default)]
    pub editors: Vec<String>,
    #[serde(default)]
    pub translators: Vec<String>,

    pub publisher: Option<String>,
    pub imprint: Option<String>,
    #[serde(default)]
    pub genres: Vec<String>,
    #[serde(default)]
    pub characters: Vec<String>,
    pub story_arc: Option<String>,

    pub language: Option<String>,
    pub isbn: Option<String>,
    pub page_count: Option<u32>,
    pub age_rating: Option<String>,
    pub web: Option<String>,
    pub rights: Option<String>,

    #[serde(default)]
    pub source: BookMetadataSource,
    /// Entry inside the archive the metadata was read from.
    pub entry_path: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookProperties {
    pub container: String,
    pub file_size: u64,
    /// Number of image entries in a comic archive, when countable.
    pub page_count: Option<u32>,
    /// False when the archive format cannot be opened for metadata.
    pub readable: bool,
}

/// The cover image extracted from a comic archive or EPUB.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookCover {
    pub data: String,
    pub mime_type: String,
    pub entry_path: String,
}
