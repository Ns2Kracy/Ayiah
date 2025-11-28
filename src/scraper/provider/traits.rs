use crate::scraper::{
    Result,
    types::{EpisodeInfo, MediaInfo, MediaMetadata, MediaType},
};
use async_trait::async_trait;

/// Search options for providers
#[derive(Debug, Clone, Default)]
pub struct SearchOptions {
    /// Year filter
    pub year: Option<i32>,
    /// Limit results
    pub limit: Option<usize>,
    /// Preferred language (ISO 639-1)
    pub language: Option<String>,
    /// Media type filter
    pub media_type: Option<MediaType>,
}

impl SearchOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_year(mut self, year: Option<i32>) -> Self {
        self.year = year;
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn with_language(mut self, language: impl Into<String>) -> Self {
        self.language = Some(language.into());
        self
    }

    pub fn with_type(mut self, media_type: MediaType) -> Self {
        self.media_type = Some(media_type);
        self
    }
}

/// Core trait for metadata providers
#[async_trait]
pub trait MetadataProvider: Send + Sync {
    /// Provider identifier (e.g., "tmdb", "anilist")
    fn id(&self) -> &'static str;

    /// Human-readable provider name
    fn name(&self) -> &'static str;

    /// Media types this provider supports
    fn supported_types(&self) -> &[MediaType];

    /// Whether this provider requires an API key
    fn requires_api_key(&self) -> bool {
        false
    }

    /// Provider priority for a given media type (higher = preferred)
    fn priority_for(&self, media_type: MediaType) -> i32 {
        if self.supported_types().contains(&media_type) {
            50
        } else {
            0
        }
    }

    /// Search for media
    async fn search(&self, query: &str, options: &SearchOptions) -> Result<Vec<MediaInfo>>;

    /// Get detailed metadata by provider ID
    async fn get_metadata(&self, id: &str, media_type: MediaType) -> Result<MediaMetadata>;

    /// Get episode details
    async fn get_episode(&self, series_id: &str, season: i32, episode: i32) -> Result<EpisodeInfo>;

    /// Search by external ID (e.g., IMDB ID)
    async fn find_by_external_id(
        &self,
        _external_id: &str,
        _source: &str,
    ) -> Result<Option<MediaInfo>> {
        Ok(None)
    }
}

/// Provider capability flags
#[derive(Debug, Clone, Copy)]
pub struct ProviderCapabilities {
    pub search: bool,
    pub metadata: bool,
    pub episodes: bool,
    pub images: bool,
    pub external_ids: bool,
}

impl Default for ProviderCapabilities {
    fn default() -> Self {
        Self {
            search: true,
            metadata: true,
            episodes: false,
            images: true,
            external_ids: false,
        }
    }
}
