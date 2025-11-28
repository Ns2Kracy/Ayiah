use crate::scraper::{
    Result, ScraperError,
    cache::ScraperCache,
    matcher::{Confidence, Matcher, ScoredMatch},
    parser::{MediaHint, ParsedMedia, Parser},
    provider::{MetadataProvider, SearchOptions},
    types::{EpisodeInfo, MediaInfo, MediaMetadata, MediaType},
};
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Scraper manager configuration
#[derive(Debug, Clone)]
pub struct ScraperConfig {
    /// Minimum confidence level to auto-accept a match
    pub min_confidence: Confidence,
    /// Maximum number of results to return from search
    pub max_results: usize,
    /// Whether to use caching
    pub use_cache: bool,
    /// Default language for searches
    pub language: Option<String>,
}

impl Default for ScraperConfig {
    fn default() -> Self {
        Self {
            min_confidence: Confidence::Medium,
            max_results: 20,
            use_cache: true,
            language: None,
        }
    }
}

/// Result of a scrape operation
#[derive(Debug, Clone)]
pub struct ScrapeResult {
    /// The matched media info
    pub info: MediaInfo,
    /// Full metadata (if fetched)
    pub metadata: Option<MediaMetadata>,
    /// Match confidence
    pub confidence: Confidence,
    /// Match score
    pub score: i32,
    /// Parsed filename info
    pub parsed: ParsedMedia,
}

/// Main scraper manager
pub struct ScraperManager {
    providers: Vec<Arc<dyn MetadataProvider>>,
    cache: ScraperCache,
    config: ScraperConfig,
}

impl ScraperManager {
    /// Create a new scraper manager
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
            cache: ScraperCache::new(),
            config: ScraperConfig::default(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: ScraperConfig) -> Self {
        Self {
            providers: Vec::new(),
            cache: ScraperCache::new(),
            config,
        }
    }

    /// Add a provider
    pub fn add_provider<P: MetadataProvider + 'static>(&mut self, provider: P) {
        self.providers.push(Arc::new(provider));
    }

    /// Get all providers
    pub fn providers(&self) -> &[Arc<dyn MetadataProvider>] {
        &self.providers
    }

    /// Scrape metadata for a file path
    pub async fn scrape(&self, path: &Path) -> Result<ScrapeResult> {
        let parsed = Parser::parse(path);
        self.scrape_parsed(&parsed).await
    }

    /// Scrape metadata using pre-parsed info
    pub async fn scrape_parsed(&self, parsed: &ParsedMedia) -> Result<ScrapeResult> {
        info!("Scraping: {} (hint: {:?})", parsed.title, parsed.hint);

        // Search all relevant providers
        let results = self
            .search_all(&parsed.title, parsed.year, parsed.hint)
            .await?;

        // Rank results
        let ranked = Matcher::rank(results, parsed);

        if ranked.is_empty() {
            return Err(ScraperError::NotFound(format!(
                "No results found for: {}",
                parsed.title
            )));
        }

        // Get best match
        let best = ranked.into_iter().next().ok_or_else(|| {
            ScraperError::NotFound(format!("No results found for: {}", parsed.title))
        })?;

        debug!(
            "Best match: {} (score: {}, confidence: {:?})",
            best.info.title, best.score, best.confidence
        );

        // Fetch full metadata if confidence is high enough
        let metadata = if best.confidence >= self.config.min_confidence {
            match self.get_metadata(&best.info).await {
                Ok(m) => Some(m),
                Err(e) => {
                    warn!("Failed to fetch metadata: {}", e);
                    None
                }
            }
        } else {
            None
        };

        Ok(ScrapeResult {
            info: best.info,
            metadata,
            confidence: best.confidence,
            score: best.score,
            parsed: parsed.clone(),
        })
    }

    /// Search for media across all providers
    pub async fn search(
        &self,
        query: &str,
        year: Option<i32>,
        media_type: Option<MediaType>,
    ) -> Result<Vec<MediaInfo>> {
        let hint = media_type
            .map(|t| match t {
                MediaType::Movie => MediaHint::Movie,
                MediaType::Tv => MediaHint::TvShow,
                MediaType::Anime => MediaHint::Anime,
                MediaType::Unknown => MediaHint::Unknown,
            })
            .unwrap_or(MediaHint::Unknown);

        self.search_all(query, year, hint).await
    }

    /// Search and rank results
    pub async fn search_ranked(
        &self,
        query: &str,
        year: Option<i32>,
        media_type: Option<MediaType>,
    ) -> Result<Vec<ScoredMatch>> {
        let results = self.search(query, year, media_type).await?;

        let parsed = ParsedMedia {
            title: query.to_string(),
            original_title: query.to_string(),
            year,
            hint: media_type
                .map(|t| match t {
                    MediaType::Movie => MediaHint::Movie,
                    MediaType::Tv => MediaHint::TvShow,
                    MediaType::Anime => MediaHint::Anime,
                    MediaType::Unknown => MediaHint::Unknown,
                })
                .unwrap_or(MediaHint::Unknown),
            ..Default::default()
        };

        Ok(Matcher::rank(results, &parsed))
    }

    /// Get full metadata for a media item
    pub async fn get_metadata(&self, info: &MediaInfo) -> Result<MediaMetadata> {
        // Check cache first
        if self.config.use_cache
            && let Some(cached) = self.cache.get_metadata(&info.provider, &info.id).await
        {
            debug!("Cache hit for metadata: {}:{}", info.provider, info.id);
            return Ok(cached);
        }

        // Find the provider
        let provider = self
            .providers
            .iter()
            .find(|p| p.id() == info.provider)
            .ok_or_else(|| {
                ScraperError::Config(format!("Provider not found: {}", info.provider))
            })?;

        // Fetch metadata
        let metadata = provider.get_metadata(&info.id, info.media_type).await?;

        // Cache the result
        if self.config.use_cache {
            self.cache
                .set_metadata(&info.provider, &info.id, metadata.clone())
                .await;
        }

        Ok(metadata)
    }

    /// Get episode details
    pub async fn get_episode(
        &self,
        provider: &str,
        series_id: &str,
        season: i32,
        episode: i32,
    ) -> Result<EpisodeInfo> {
        let provider = self
            .providers
            .iter()
            .find(|p| p.id() == provider)
            .ok_or_else(|| ScraperError::Config(format!("Provider not found: {provider}")))?;

        provider.get_episode(series_id, season, episode).await
    }

    /// Find by external ID
    pub async fn find_by_external_id(
        &self,
        external_id: &str,
        source: &str,
    ) -> Result<Option<MediaInfo>> {
        for provider in &self.providers {
            if let Ok(Some(info)) = provider.find_by_external_id(external_id, source).await {
                return Ok(Some(info));
            }
        }
        Ok(None)
    }

    /// Internal search across all providers
    async fn search_all(
        &self,
        query: &str,
        year: Option<i32>,
        hint: MediaHint,
    ) -> Result<Vec<MediaInfo>> {
        let media_type = match hint {
            MediaHint::Movie => Some(MediaType::Movie),
            MediaHint::TvShow => Some(MediaType::Tv),
            MediaHint::Anime => Some(MediaType::Anime),
            MediaHint::Unknown => None,
        };

        // Sort providers by priority for this media type
        let mut providers: Vec<_> = self.providers.iter().collect();
        providers.sort_by(|a, b| {
            let type_for_sort = media_type.unwrap_or(MediaType::Unknown);
            b.priority_for(type_for_sort)
                .cmp(&a.priority_for(type_for_sort))
        });

        let options = SearchOptions::new()
            .with_year(year)
            .with_limit(self.config.max_results);

        let options = if let Some(mt) = media_type {
            options.with_type(mt)
        } else {
            options
        };

        let options = if let Some(ref lang) = self.config.language {
            options.with_language(lang.clone())
        } else {
            options
        };

        let mut all_results = Vec::new();

        for provider in providers {
            // Check cache first
            if self.config.use_cache
                && let Some(cached) = self.cache.get_search(provider.id(), query, year).await
            {
                debug!("Cache hit for search: {}:{}", provider.id(), query);
                all_results.extend(cached);
                continue;
            }

            // Search provider
            match provider.search(query, &options).await {
                Ok(results) => {
                    debug!(
                        "Provider {} returned {} results",
                        provider.id(),
                        results.len()
                    );

                    // Cache results
                    if self.config.use_cache {
                        self.cache
                            .set_search(provider.id(), query, year, results.clone())
                            .await;
                    }

                    all_results.extend(results);
                }
                Err(e) => {
                    debug!("Provider {} search failed: {}", provider.id(), e);
                }
            }
        }

        if all_results.is_empty() {
            return Err(ScraperError::NotFound(format!(
                "No results found for: {query}"
            )));
        }

        // Limit total results
        all_results.truncate(self.config.max_results);

        Ok(all_results)
    }

    /// Clear the cache
    pub fn clear_cache(&self) {
        self.cache.clear();
    }
}

impl Default for ScraperManager {
    fn default() -> Self {
        Self::new()
    }
}
