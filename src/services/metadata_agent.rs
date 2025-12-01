use crate::{
    entities::{CreateVideoMetadata, MediaItem, MediaType as EntityMediaType, VideoMetadata},
    scraper::{Confidence, MediaMetadata, MediaType, Parser, ScraperManager},
};
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

/// Metadata agent service for fetching and saving metadata
pub struct MetadataAgent {
    scraper_manager: Arc<ScraperManager>,
    db: sqlx::SqlitePool,
}

impl MetadataAgent {
    /// Create a new metadata agent
    #[must_use] 
    pub const fn new(scraper_manager: Arc<ScraperManager>, db: sqlx::SqlitePool) -> Self {
        Self {
            scraper_manager,
            db,
        }
    }

    /// Fetch and save metadata for a media item
    pub async fn fetch_and_save_metadata(
        &self,
        media_item: &MediaItem,
    ) -> Result<VideoMetadata, MetadataAgentError> {
        info!(
            "Fetching metadata for {} (ID: {})",
            media_item.title, media_item.id
        );

        // Parse the title to extract structured information
        let parsed = Parser::parse_filename(&media_item.title);

        // Convert entity media type to scraper media type for filtering
        let media_type = match media_item.media_type {
            EntityMediaType::Movie => Some(MediaType::Movie),
            EntityMediaType::Tv => Some(MediaType::Tv),
            EntityMediaType::Comic | EntityMediaType::Book => None,
        };

        // Search and rank results
        let ranked_results = self
            .scraper_manager
            .search_ranked(&parsed.title, parsed.year, media_type)
            .await
            .map_err(|e| {
                error!("Failed to search for {}: {}", parsed.title, e);
                MetadataAgentError::SearchFailed(e.to_string())
            })?;

        // Get the best match
        let best_match = ranked_results
            .into_iter()
            .next()
            .filter(|m| m.confidence >= Confidence::Low)
            .ok_or_else(|| {
                warn!("No matching results found for {}", parsed.title);
                MetadataAgentError::NoMatchingResults
            })?;

        debug!(
            "Found match: {} (score: {}, confidence: {:?}, provider: {})",
            best_match.info.title,
            best_match.score,
            best_match.confidence,
            best_match.info.provider
        );

        // Get detailed metadata
        let metadata = self
            .scraper_manager
            .get_metadata(&best_match.info)
            .await
            .map_err(|e| {
                error!("Failed to get details: {}", e);
                MetadataAgentError::DetailsFailed(e.to_string())
            })?;

        // Convert to database format and save
        let saved = self.save_metadata(media_item.id, &metadata).await?;

        info!(
            "Successfully saved metadata for {} (ID: {}, confidence: {:?})",
            media_item.title, media_item.id, best_match.confidence
        );

        Ok(saved)
    }

    /// Fetch metadata using file path for better parsing
    pub async fn fetch_metadata_from_path(
        &self,
        media_item: &MediaItem,
        file_path: &Path,
    ) -> Result<VideoMetadata, MetadataAgentError> {
        info!(
            "Fetching metadata for {} from path: {}",
            media_item.title,
            file_path.display()
        );

        // Use the scraper's built-in path parsing
        let scrape_result = self.scraper_manager.scrape(file_path).await.map_err(|e| {
            error!("Failed to scrape {}: {}", file_path.display(), e);
            MetadataAgentError::SearchFailed(e.to_string())
        })?;

        debug!(
            "Scrape result: {} (score: {}, confidence: {:?})",
            scrape_result.info.title, scrape_result.score, scrape_result.confidence
        );

        // Get or use existing metadata
        let metadata = if let Some(m) = scrape_result.metadata {
            m
        } else {
            self.scraper_manager
                .get_metadata(&scrape_result.info)
                .await
                .map_err(|e| {
                    error!("Failed to get details: {}", e);
                    MetadataAgentError::DetailsFailed(e.to_string())
                })?
        };

        // Save to database
        let saved = self.save_metadata(media_item.id, &metadata).await?;

        info!(
            "Successfully saved metadata for {} (ID: {})",
            media_item.title, media_item.id
        );

        Ok(saved)
    }

    /// Save metadata to database
    async fn save_metadata(
        &self,
        media_item_id: i64,
        metadata: &MediaMetadata,
    ) -> Result<VideoMetadata, MetadataAgentError> {
        let create_metadata = CreateVideoMetadata {
            media_item_id,
            tmdb_id: metadata
                .external_ids
                .tmdb
                .as_ref()
                .and_then(|id| id.parse().ok()),
            tvdb_id: metadata
                .external_ids
                .tvdb
                .as_ref()
                .and_then(|id| id.parse().ok()),
            imdb_id: metadata.external_ids.imdb.clone(),
            overview: metadata.overview.clone(),
            poster_path: metadata.images.poster.clone(),
            backdrop_path: metadata.images.backdrop.clone(),
            release_date: metadata.release_date.clone(),
            runtime: metadata.runtime,
            vote_average: metadata.rating,
            vote_count: metadata.vote_count,
            genres: metadata.genres.clone(),
        };

        VideoMetadata::upsert(&self.db, create_metadata)
            .await
            .map_err(|e| {
                error!("Failed to save metadata to database: {}", e);
                MetadataAgentError::DatabaseError(e.to_string())
            })
    }

    /// Refresh metadata for an existing media item
    pub async fn refresh_metadata(
        &self,
        media_item_id: i64,
    ) -> Result<VideoMetadata, MetadataAgentError> {
        let media_item = MediaItem::find_by_id(&self.db, media_item_id)
            .await
            .map_err(|e| MetadataAgentError::DatabaseError(e.to_string()))?
            .ok_or(MetadataAgentError::MediaItemNotFound)?;

        self.fetch_and_save_metadata(&media_item).await
    }

    /// Batch fetch metadata for multiple media items
    pub async fn batch_fetch_metadata(
        &self,
        media_items: Vec<MediaItem>,
    ) -> Vec<Result<VideoMetadata, MetadataAgentError>> {
        let mut results = Vec::new();

        for item in media_items {
            let result = self.fetch_and_save_metadata(&item).await;
            results.push(result);

            // Add a small delay to respect rate limits
            tokio::time::sleep(tokio::time::Duration::from_millis(250)).await;
        }

        results
    }

    /// Search for media without saving
    pub async fn search(
        &self,
        query: &str,
        year: Option<i32>,
        media_type: Option<MediaType>,
    ) -> Result<Vec<crate::scraper::MediaInfo>, MetadataAgentError> {
        self.scraper_manager
            .search(query, year, media_type)
            .await
            .map_err(|e| MetadataAgentError::SearchFailed(e.to_string()))
    }

    /// Get metadata for a specific provider ID
    pub async fn get_metadata_by_id(
        &self,
        provider: &str,
        id: &str,
        media_type: MediaType,
    ) -> Result<MediaMetadata, MetadataAgentError> {
        let info = crate::scraper::MediaInfo::new(id, "", provider).with_type(media_type);

        self.scraper_manager
            .get_metadata(&info)
            .await
            .map_err(|e| MetadataAgentError::DetailsFailed(e.to_string()))
    }
}

/// Metadata agent errors
#[derive(Debug, thiserror::Error)]
pub enum MetadataAgentError {
    #[error("Search failed: {0}")]
    SearchFailed(String),

    #[error("No matching results found")]
    NoMatchingResults,

    #[error("Failed to get details: {0}")]
    DetailsFailed(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Media item not found")]
    MediaItemNotFound,

    #[error("Unsupported media type: {0}")]
    UnsupportedMediaType(String),
}
