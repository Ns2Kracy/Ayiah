use crate::scraper::types::{MediaInfo, MediaMetadata};
use moka::future::Cache;
use std::sync::Arc;
use std::time::Duration;

/// Cache key for search results
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct SearchKey {
    provider: String,
    query: String,
    year: Option<i32>,
}

/// Cache key for metadata
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct MetadataKey {
    provider: String,
    id: String,
}

/// Scraper cache for API responses
#[derive(Clone)]
pub struct ScraperCache {
    search_cache: Cache<SearchKey, Arc<Vec<MediaInfo>>>,
    metadata_cache: Cache<MetadataKey, Arc<MediaMetadata>>,
}

impl ScraperCache {
    /// Create a new cache with default settings
    pub fn new() -> Self {
        Self::with_config(CacheConfig::default())
    }

    /// Create a new cache with custom configuration
    pub fn with_config(config: CacheConfig) -> Self {
        let search_cache = Cache::builder()
            .max_capacity(config.search_max_entries)
            .time_to_live(config.search_ttl)
            .build();

        let metadata_cache = Cache::builder()
            .max_capacity(config.metadata_max_entries)
            .time_to_live(config.metadata_ttl)
            .build();

        Self {
            search_cache,
            metadata_cache,
        }
    }

    /// Get cached search results
    pub async fn get_search(
        &self,
        provider: &str,
        query: &str,
        year: Option<i32>,
    ) -> Option<Vec<MediaInfo>> {
        let key = SearchKey {
            provider: provider.to_string(),
            query: query.to_lowercase(),
            year,
        };

        self.search_cache.get(&key).await.map(|arc| (*arc).clone())
    }

    /// Cache search results
    pub async fn set_search(
        &self,
        provider: &str,
        query: &str,
        year: Option<i32>,
        results: Vec<MediaInfo>,
    ) {
        let key = SearchKey {
            provider: provider.to_string(),
            query: query.to_lowercase(),
            year,
        };

        self.search_cache.insert(key, Arc::new(results)).await;
    }

    /// Get cached metadata
    pub async fn get_metadata(&self, provider: &str, id: &str) -> Option<MediaMetadata> {
        let key = MetadataKey {
            provider: provider.to_string(),
            id: id.to_string(),
        };

        self.metadata_cache
            .get(&key)
            .await
            .map(|arc| (*arc).clone())
    }

    /// Cache metadata
    pub async fn set_metadata(&self, provider: &str, id: &str, metadata: MediaMetadata) {
        let key = MetadataKey {
            provider: provider.to_string(),
            id: id.to_string(),
        };

        self.metadata_cache.insert(key, Arc::new(metadata)).await;
    }

    /// Clear all caches
    pub fn clear(&self) {
        self.search_cache.invalidate_all();
        self.metadata_cache.invalidate_all();
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            search_entries: self.search_cache.entry_count(),
            metadata_entries: self.metadata_cache.entry_count(),
        }
    }
}

impl Default for ScraperCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Cache configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Maximum number of search result entries
    pub search_max_entries: u64,
    /// TTL for search results
    pub search_ttl: Duration,
    /// Maximum number of metadata entries
    pub metadata_max_entries: u64,
    /// TTL for metadata
    pub metadata_ttl: Duration,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            search_max_entries: 1000,
            search_ttl: Duration::from_secs(3600), // 1 hour
            metadata_max_entries: 500,
            metadata_ttl: Duration::from_secs(86400), // 24 hours
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub search_entries: u64,
    pub metadata_entries: u64,
}
