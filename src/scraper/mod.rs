pub mod downloader;
pub mod parser;
pub mod provider;
pub mod scanner;
pub mod writer;

mod types;

pub use downloader::Downloader;
pub use parser::{ParsedInfo, Parser};
pub use scanner::Scanner;
pub use types::*;
pub use writer::Writer;

use async_trait::async_trait;
use std::time::Duration;

/// Scraper result type
pub type Result<T> = std::result::Result<T, ScraperError>;

/// Scraper error types
#[derive(Debug, thiserror::Error)]
pub enum ScraperError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("API error: {status} - {message}")]
    Api { status: u16, message: String },

    #[error("Rate limit exceeded. Retry after: {0:?}")]
    RateLimit(Duration),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Cache error: {0}")]
    Cache(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("XML error: {0}")]
    Xml(#[from] quick_xml::DeError),
}

/// Core trait for metadata providers
#[async_trait]
pub trait MetadataProvider: Send + Sync {
    /// Provider name
    fn name(&self) -> &str;

    /// Whether the provider requires an API key
    fn requires_api_key(&self) -> bool {
        false
    }

    /// Generic search
    async fn search(&self, query: &str, year: Option<i32>) -> Result<Vec<MediaSearchResult>>;

    /// Get media details
    async fn get_details(&self, result: &MediaSearchResult) -> Result<MediaDetails>;

    /// Get episode details
    async fn get_episode_details(
        &self,
        series_id: &str,
        season: i32,
        episode: i32,
    ) -> Result<EpisodeMetadata>;
}

/// Scraper manager for managing multiple providers
pub struct ScraperManager {
    providers: Vec<Box<dyn MetadataProvider>>,
}

impl ScraperManager {
    /// Create a new scraper manager
    #[must_use]
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    /// Add a provider
    pub fn add_provider(&mut self, provider: Box<dyn MetadataProvider>) {
        self.providers.push(provider);
    }

    /// Get all providers
    #[must_use]
    pub fn providers(&self) -> &[Box<dyn MetadataProvider>] {
        &self.providers
    }

    /// Search media
    pub async fn search(&self, query: &str, year: Option<i32>) -> Result<Vec<MediaSearchResult>> {
        let mut all_results = Vec::new();

        for provider in &self.providers {
            match provider.search(query, year).await {
                Ok(results) => {
                    all_results.extend(results);
                }
                Err(e) => {
                    tracing::debug!("Provider {} search failed: {}", provider.name(), e);
                }
            }
        }

        if all_results.is_empty() {
            Err(ScraperError::NotFound(format!(
                "No provider could find: {query}"
            )))
        } else {
            Ok(all_results)
        }
    }

    /// Get media details
    pub async fn get_details(&self, result: &MediaSearchResult) -> Result<MediaDetails> {
        let provider_name = result.provider();

        let provider = self
            .providers
            .iter()
            .find(|p| p.name() == provider_name)
            .ok_or_else(|| ScraperError::Config(format!("Provider not found: {provider_name}")))?;

        provider.get_details(result).await
    }

    /// Get episode details
    pub async fn get_episode_details(
        &self,
        provider_name: &str,
        series_id: &str,
        season: i32,
        episode: i32,
    ) -> Result<EpisodeMetadata> {
        let provider = self
            .providers
            .iter()
            .find(|p| p.name() == provider_name)
            .ok_or_else(|| ScraperError::Config(format!("Provider not found: {provider_name}")))?;

        provider
            .get_episode_details(series_id, season, episode)
            .await
    }
}

impl Default for ScraperManager {
    fn default() -> Self {
        Self::new()
    }
}
