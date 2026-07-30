pub mod coverart;
pub mod musicbrainz;
pub mod tmdb;
pub mod tvdb;
pub mod video;

use std::sync::Arc;

use async_trait::async_trait;

use crate::error::Result;
use crate::models::album::CoverArt;
use crate::models::provider_result::{ProviderInfo, ProviderRelease, ProviderSearchResult};

#[async_trait]
pub trait MetadataProvider: Send + Sync {
    fn id(&self) -> &str;
    fn display_name(&self) -> &str;
    fn supported_media_types(&self) -> Vec<String>;

    async fn search_release(
        &self,
        query: &str,
        artist: Option<&str>,
    ) -> Result<Vec<ProviderSearchResult>>;

    async fn search_recording(
        &self,
        query: &str,
        artist: Option<&str>,
    ) -> Result<Vec<ProviderSearchResult>>;

    async fn search_artist(&self, query: &str) -> Result<Vec<ProviderSearchResult>>;

    async fn get_release(&self, id: &str) -> Result<ProviderRelease>;

    async fn get_cover_art(&self, release_id: &str) -> Result<Vec<CoverArt>>;

    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            id: self.id().to_string(),
            display_name: self.display_name().to_string(),
            supported_types: self.supported_media_types(),
        }
    }
}

pub struct ProviderRegistry {
    providers: Vec<Arc<dyn MetadataProvider>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    pub fn register(&mut self, provider: Arc<dyn MetadataProvider>) {
        self.providers.push(provider);
    }

    pub fn get(&self, id: &str) -> Option<&dyn MetadataProvider> {
        self.providers
            .iter()
            .find(|p| p.id() == id)
            .map(|p| p.as_ref())
    }

    pub fn get_arc(&self, id: &str) -> Option<Arc<dyn MetadataProvider>> {
        self.providers
            .iter()
            .find(|p| p.id() == id)
            .cloned()
    }

    pub fn list(&self) -> Vec<ProviderInfo> {
        self.providers.iter().map(|p| p.info()).collect()
    }
}
