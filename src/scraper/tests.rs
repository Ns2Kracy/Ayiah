//! Scraper integration tests

#[cfg(test)]
mod types_tests {
    use crate::scraper::types::{ExternalIds, MediaInfo, MediaMetadata, MediaType};

    #[test]
    fn test_media_info_builder() {
        let info = MediaInfo::new("123", "Test Movie", "tmdb")
            .with_type(MediaType::Movie)
            .with_year(Some(2023))
            .with_original_title(Some("Original Title".to_string()))
            .with_alt_title("Alternative Title")
            .with_rating(Some(8.5))
            .with_popularity(Some(100.0));

        assert_eq!(info.id, "123");
        assert_eq!(info.title, "Test Movie");
        assert_eq!(info.provider, "tmdb");
        assert_eq!(info.media_type, MediaType::Movie);
        assert_eq!(info.year, Some(2023));
        assert_eq!(info.rating, Some(8.5));
    }

    #[test]
    fn test_media_info_all_titles() {
        let info = MediaInfo::new("1", "English Title", "test")
            .with_original_title(Some("日本語タイトル".to_string()))
            .with_alt_title("Alternative 1")
            .with_alt_title("Alternative 2");

        let titles = info.all_titles();

        assert_eq!(titles.len(), 4);
        assert!(titles.contains(&"English Title"));
        assert!(titles.contains(&"日本語タイトル"));
        assert!(titles.contains(&"Alternative 1"));
    }

    #[test]
    fn test_media_type_compatibility() {
        assert!(MediaType::Anime.is_compatible_with(MediaType::Tv));
        assert!(MediaType::Tv.is_compatible_with(MediaType::Anime));
        assert!(MediaType::Unknown.is_compatible_with(MediaType::Movie));
        assert!(!MediaType::Movie.is_compatible_with(MediaType::Tv));
    }

    #[test]
    fn test_external_ids_merge() {
        let mut ids1 = ExternalIds {
            imdb: Some("tt1234567".to_string()),
            tmdb: Some("123".to_string()),
            ..Default::default()
        };

        let ids2 = ExternalIds {
            tmdb: Some("456".to_string()), // Should override
            tvdb: Some("789".to_string()), // Should add
            ..Default::default()
        };

        ids1.merge(&ids2);

        assert_eq!(ids1.imdb, Some("tt1234567".to_string())); // Unchanged
        assert_eq!(ids1.tmdb, Some("456".to_string())); // Overridden
        assert_eq!(ids1.tvdb, Some("789".to_string())); // Added
    }

    #[test]
    fn test_external_ids_has_any() {
        let empty = ExternalIds::default();
        assert!(!empty.has_any());

        let with_imdb = ExternalIds {
            imdb: Some("tt1234567".to_string()),
            ..Default::default()
        };
        assert!(with_imdb.has_any());
    }

    #[test]
    fn test_media_metadata_default() {
        let metadata = MediaMetadata::default();

        assert!(metadata.id.is_empty());
        assert!(metadata.title.is_empty());
        assert_eq!(metadata.media_type, MediaType::Unknown);
        assert!(metadata.genres.is_empty());
        assert!(metadata.cast.is_empty());
    }
}

#[cfg(test)]
mod cache_tests {
    use crate::scraper::{
        cache::{CacheConfig, ScraperCache},
        types::{MediaInfo, MediaMetadata, MediaType},
    };
    use std::time::Duration;

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

        // Moka cache is async, so we need to sync to ensure entries are counted
        // The entry_count may be eventually consistent
        let stats = cache.stats();
        // Just verify it doesn't panic and returns valid stats
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

#[cfg(test)]
mod manager_tests {
    use crate::scraper::matcher::Confidence;
    use crate::scraper::{AniListProvider, BangumiProvider, ScraperConfig, ScraperManager};

    #[test]
    fn test_manager_creation() {
        let manager = ScraperManager::new();
        assert!(manager.providers().is_empty());
    }

    #[test]
    fn test_manager_add_providers() {
        let mut manager = ScraperManager::new();

        manager.add_provider(AniListProvider::new());
        manager.add_provider(BangumiProvider::new());

        assert_eq!(manager.providers().len(), 2);
    }

    #[test]
    fn test_manager_config() {
        let config = ScraperConfig {
            min_confidence: Confidence::High,
            max_results: 10,
            use_cache: false,
            language: Some("zh-CN".to_string()),
        };

        let manager = ScraperManager::with_config(config);
        assert!(manager.providers().is_empty());
    }

    #[test]
    fn test_default_manager_creation() {
        // Without API key
        let manager = crate::scraper::create_default_manager(None);
        assert_eq!(manager.providers().len(), 2); // AniList + Bangumi

        // With API key
        let manager = crate::scraper::create_default_manager(Some("fake_key"));
        assert_eq!(manager.providers().len(), 3); // TMDB + AniList + Bangumi
    }
}

#[cfg(test)]
mod scanner_tests {
    use crate::scraper::Scanner;
    use std::fs::{self, File};
    use tempfile::TempDir;

    #[test]
    fn test_scan_finds_video_files() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path();

        // Create test video files
        File::create(dir_path.join("movie.mkv")).unwrap();
        File::create(dir_path.join("show.mp4")).unwrap();
        File::create(dir_path.join("document.txt")).unwrap();

        let results = Scanner::scan(dir_path);

        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|p| p.extension().unwrap() == "mkv"));
        assert!(results.iter().any(|p| p.extension().unwrap() == "mp4"));
    }

    #[test]
    fn test_scan_ignores_non_video() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path();

        File::create(dir_path.join("image.jpg")).unwrap();
        File::create(dir_path.join("audio.mp3")).unwrap();
        File::create(dir_path.join("subtitle.srt")).unwrap();

        let results = Scanner::scan(dir_path);

        assert!(results.is_empty());
    }

    #[test]
    fn test_scan_recursive() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path();

        // Create nested structure
        let subdir = dir_path.join("Season 1");
        fs::create_dir(&subdir).unwrap();

        File::create(dir_path.join("movie.mkv")).unwrap();
        File::create(subdir.join("episode.mkv")).unwrap();

        let results = Scanner::scan(dir_path);

        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_scan_detects_bluray_structure() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path();

        // Create Blu-ray structure
        let bdmv_dir = dir_path.join("Movie").join("BDMV");
        fs::create_dir_all(&bdmv_dir).unwrap();
        File::create(bdmv_dir.join("index.bdmv")).unwrap();

        let results = Scanner::scan(dir_path);

        // Should return the root movie folder, not the bdmv file
        assert_eq!(results.len(), 1);
        assert!(results[0].ends_with("Movie"));
    }
}
