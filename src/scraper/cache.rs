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
    #[must_use] 
    pub fn new() -> Self {
        Self::with_config(CacheConfig::default())
    }

    /// Create a new cache with custom configuration
    #[must_use] 
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
    #[must_use] 
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scraper::types::MediaType;

    #[tokio::test]
    async fn test_cache_search_results() {
        let cache = ScraperCache::new();

        let results = vec![MediaInfo::new("1", "Test Movie", "tmdb").with_type(MediaType::Movie)];

        // Cache miss
        let cached = cache.get_search("tmdb", "test", None).await;
        assert!(cached.is_none());

        // Set cache
        cache
            .set_search("tmdb", "test", None, results.clone())
            .await;

        // Cache hit
        let cached = cache.get_search("tmdb", "test", None).await;
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_cache_metadata() {
        let cache = ScraperCache::new();

        let metadata = MediaMetadata {
            id: "123".to_string(),
            title: "Test Movie".to_string(),
            provider: "tmdb".to_string(),
            ..Default::default()
        };

        // Cache miss
        let cached = cache.get_metadata("tmdb", "123").await;
        assert!(cached.is_none());

        // Set cache
        cache.set_metadata("tmdb", "123", metadata.clone()).await;

        // Cache hit
        let cached = cache.get_metadata("tmdb", "123").await;
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().title, "Test Movie");
    }

    #[tokio::test]
    async fn test_cache_clear() {
        let cache = ScraperCache::new();

        cache
            .set_search(
                "tmdb",
                "test",
                None,
                vec![MediaInfo::new("1", "Test", "tmdb")],
            )
            .await;

        cache.clear();

        let cached = cache.get_search("tmdb", "test", None).await;
        assert!(cached.is_none());
    }

    #[tokio::test]
    async fn test_cache_stats() {
        let cache = ScraperCache::new();

        cache
            .set_search(
                "tmdb",
                "test1",
                None,
                vec![MediaInfo::new("1", "Test1", "tmdb")],
            )
            .await;
        cache
            .set_search(
                "tmdb",
                "test2",
                None,
                vec![MediaInfo::new("2", "Test2", "tmdb")],
            )
            .await;

        let stats = cache.stats();
        assert!(stats.search_entries <= 2);
    }

    #[test]
    fn test_cache_config_default() {
        let config = CacheConfig::default();

        assert_eq!(config.search_max_entries, 1000);
        assert_eq!(config.search_ttl, Duration::from_secs(3600));
        assert_eq!(config.metadata_max_entries, 500);
        assert_eq!(config.metadata_ttl, Duration::from_secs(86400));
    }
}
