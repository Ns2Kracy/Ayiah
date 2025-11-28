use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Video metadata entity
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct VideoMetadata {
    pub id: i64,
    pub media_item_id: i64,
    pub tmdb_id: Option<i64>,
    pub tvdb_id: Option<i64>,
    pub imdb_id: Option<String>,
    pub overview: Option<String>,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
    pub release_date: Option<String>,
    pub runtime: Option<i32>,
    pub vote_average: Option<f64>,
    pub vote_count: Option<i32>,
    pub genres: Option<String>, // JSON array
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create video metadata request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateVideoMetadata {
    pub media_item_id: i64,
    pub tmdb_id: Option<i64>,
    pub tvdb_id: Option<i64>,
    pub imdb_id: Option<String>,
    pub overview: Option<String>,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
    pub release_date: Option<String>,
    pub runtime: Option<i32>,
    pub vote_average: Option<f64>,
    pub vote_count: Option<i32>,
    pub genres: Vec<String>,
}

/// Media item with video metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaItemWithMetadata {
    #[serde(flatten)]
    pub media_item: super::MediaItem,
    pub metadata: Option<VideoMetadata>,
}

impl VideoMetadata {
    /// Create or update video metadata
    pub async fn upsert(
        db: &sqlx::SqlitePool,
        metadata: CreateVideoMetadata,
    ) -> Result<Self, sqlx::Error> {
        let genres_json =
            serde_json::to_string(&metadata.genres).unwrap_or_else(|_| "[]".to_string());

        let result = sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO video_metadata (
                media_item_id, tmdb_id, tvdb_id, imdb_id, overview,
                poster_path, backdrop_path, release_date, runtime,
                vote_average, vote_count, genres
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(media_item_id) DO UPDATE SET
                tmdb_id = excluded.tmdb_id,
                tvdb_id = excluded.tvdb_id,
                imdb_id = excluded.imdb_id,
                overview = excluded.overview,
                poster_path = excluded.poster_path,
                backdrop_path = excluded.backdrop_path,
                release_date = excluded.release_date,
                runtime = excluded.runtime,
                vote_average = excluded.vote_average,
                vote_count = excluded.vote_count,
                genres = excluded.genres,
                updated_at = CURRENT_TIMESTAMP
            RETURNING *
            "#,
        )
        .bind(metadata.media_item_id)
        .bind(metadata.tmdb_id)
        .bind(metadata.tvdb_id)
        .bind(metadata.imdb_id)
        .bind(metadata.overview)
        .bind(metadata.poster_path)
        .bind(metadata.backdrop_path)
        .bind(metadata.release_date)
        .bind(metadata.runtime)
        .bind(metadata.vote_average)
        .bind(metadata.vote_count)
        .bind(genres_json)
        .fetch_one(db)
        .await?;

        Ok(result)
    }

    /// Find metadata by media item ID
    pub async fn find_by_media_item_id(
        db: &sqlx::SqlitePool,
        media_item_id: i64,
    ) -> Result<Option<Self>, sqlx::Error> {
        let result = sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM video_metadata WHERE media_item_id = ?
            "#,
        )
        .bind(media_item_id)
        .fetch_optional(db)
        .await?;

        Ok(result)
    }

    /// Parse genres from JSON string
    pub fn parse_genres(&self) -> Vec<String> {
        self.genres
            .as_ref()
            .and_then(|g| serde_json::from_str(g).ok())
            .unwrap_or_default()
    }
}

impl MediaItemWithMetadata {
    /// Get all media items with metadata
    pub async fn list_all(db: &sqlx::SqlitePool) -> Result<Vec<Self>, sqlx::Error> {
        let media_items = super::MediaItem::list_all(db).await?;

        let mut results = Vec::new();
        for item in media_items {
            let metadata = VideoMetadata::find_by_media_item_id(db, item.id).await?;
            results.push(Self {
                media_item: item,
                metadata,
            });
        }

        Ok(results)
    }

    /// Get media items with metadata by type
    pub async fn list_by_type(
        db: &sqlx::SqlitePool,
        media_type: super::MediaType,
    ) -> Result<Vec<Self>, sqlx::Error> {
        let media_items = super::MediaItem::list_by_type(db, media_type).await?;

        let mut results = Vec::new();
        for item in media_items {
            let metadata = VideoMetadata::find_by_media_item_id(db, item.id).await?;
            results.push(Self {
                media_item: item,
                metadata,
            });
        }

        Ok(results)
    }

    /// Get media item with metadata by ID
    pub async fn find_by_id(db: &sqlx::SqlitePool, id: i64) -> Result<Option<Self>, sqlx::Error> {
        let media_item = match super::MediaItem::find_by_id(db, id).await? {
            Some(item) => item,
            None => return Ok(None),
        };

        let metadata = VideoMetadata::find_by_media_item_id(db, media_item.id).await?;

        Ok(Some(Self {
            media_item,
            metadata,
        }))
    }
}
