mod cache;
mod downloader;
mod manager;
mod matcher;
mod organizer;
mod parser;
mod provider;
mod scanner;
mod types;
mod writer;

pub use cache::{CacheConfig, ScraperCache};
pub use downloader::Downloader;
pub use manager::{ScrapeResult, ScraperConfig, ScraperManager};
pub use matcher::{Confidence, Matcher, ScoredMatch};
pub use organizer::{
    BatchOrganizeResult, NamingTemplate, OrganizeMethod, OrganizeResult, Organizer, OrganizerConfig,
};
pub use parser::{MediaHint, ParsedMedia, Parser};
pub use provider::{
    AniListProvider, BangumiProvider, HttpClient, MetadataProvider, SearchOptions, TmdbProvider,
};
pub use scanner::Scanner;
pub use types::{
    EpisodeInfo, ExternalIds, ImageSet, MediaInfo, MediaMetadata, MediaType, PersonInfo, SeasonInfo,
};
pub use writer::Writer;

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

/// Create a default scraper manager with all providers
#[must_use] 
pub fn create_default_manager(tmdb_api_key: Option<&str>) -> ScraperManager {
    let mut manager = ScraperManager::new();

    // Add TMDB if API key is provided
    if let Some(key) = tmdb_api_key {
        manager.add_provider(TmdbProvider::new(key));
    }

    // Add providers that don't require API keys
    manager.add_provider(AniListProvider::new());
    manager.add_provider(BangumiProvider::new());

    manager
}
