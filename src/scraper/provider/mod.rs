pub mod anilist;
pub mod bangumi;
pub mod tmdb;

// Provider implementations will be exported in their respective modules
pub use anilist::AniListProvider;
pub use bangumi::BangumiProvider;
pub use tmdb::TmdbProvider;

use reqwest::Client;

/// Provider base configuration
#[derive(Debug, Clone, Default)]
pub struct ProviderConfig {
    /// API key
    pub api_key: Option<String>,
    /// Base URL
    pub base_url: String,
}

impl ProviderConfig {
    /// Create new configuration
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            api_key: None,
            base_url: base_url.into(),
        }
    }

    /// Set API key
    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }
}

/// Provider base structure
#[derive(Default)]
pub struct ProviderBase {
    pub config: ProviderConfig,
    pub client: Client,
}

impl ProviderBase {
    /// Create new provider base instance
    #[must_use]
    pub fn new(config: ProviderConfig) -> Self {
        let client = Client::builder()
            .user_agent("Ayiah/0.1.0")
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client");

        Self { config, client }
    }

    /// Execute rate-limited HTTP GET request
    pub async fn get(&self, url: &str) -> Result<reqwest::Response, crate::scraper::ScraperError> {
        self.client
            .get(url)
            .send()
            .await
            .map_err(crate::scraper::ScraperError::Network)
    }
}
