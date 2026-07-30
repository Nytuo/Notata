use std::sync::Arc;

use async_trait::async_trait;

use crate::error::Result;
use crate::models::video::{
    EpisodeMetadata, MovieMetadata, RemoteArtwork, SeriesMetadata, VideoProviderInfo,
    VideoSearchResult,
};

/// Metadata source for movies and series.
///
/// Kept separate from [`crate::providers::MetadataProvider`] because the music
/// trait is built around releases/recordings/artists, which do not map cleanly
/// onto movies and episodic television.
#[async_trait]
pub trait VideoMetadataProvider: Send + Sync {
    fn id(&self) -> &str;
    fn display_name(&self) -> &str;

    /// True once an API key has been supplied.
    fn is_configured(&self) -> bool;

    /// Store the API key. Called at startup and whenever settings change.
    fn configure(&self, api_key: Option<String>);

    async fn search_movie(&self, query: &str, year: Option<i32>)
        -> Result<Vec<VideoSearchResult>>;

    async fn search_series(&self, query: &str, year: Option<i32>)
        -> Result<Vec<VideoSearchResult>>;

    async fn get_movie(&self, id: &str) -> Result<MovieMetadata>;

    /// Series details. Episodes are included when the provider returns them.
    async fn get_series(&self, id: &str) -> Result<SeriesMetadata>;

    async fn get_episodes(&self, series_id: &str, season: Option<u32>)
        -> Result<Vec<EpisodeMetadata>>;

    /// Every poster and backdrop the provider holds for this title.
    ///
    /// Used by the poster picker, which needs a grid of candidates rather
    /// than the single image a search result carries.
    async fn get_artwork(&self, id: &str, is_series: bool) -> Result<Vec<RemoteArtwork>>;

    fn info(&self) -> VideoProviderInfo {
        VideoProviderInfo {
            id: self.id().to_string(),
            display_name: self.display_name().to_string(),
            requires_api_key: true,
            configured: self.is_configured(),
        }
    }
}

pub struct VideoProviderRegistry {
    providers: Vec<Arc<dyn VideoMetadataProvider>>,
}

impl VideoProviderRegistry {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    pub fn register(&mut self, provider: Arc<dyn VideoMetadataProvider>) {
        self.providers.push(provider);
    }

    /// Clone the `Arc` so callers can drop the registry lock before awaiting.
    pub fn get_arc(&self, id: &str) -> Option<Arc<dyn VideoMetadataProvider>> {
        self.providers.iter().find(|p| p.id() == id).cloned()
    }

    pub fn list(&self) -> Vec<VideoProviderInfo> {
        self.providers.iter().map(|p| p.info()).collect()
    }
}

impl Default for VideoProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}
