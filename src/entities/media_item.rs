use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Media type enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum MediaType {
    Movie,
    Tv,
    Comic,
    Book,
}

impl std::fmt::Display for MediaType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Movie => write!(f, "movie"),
            Self::Tv => write!(f, "tv"),
            Self::Comic => write!(f, "comic"),
            Self::Book => write!(f, "book"),
        }
    }
}

/// Media item entity
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MediaItem {
    pub id: i64,
    pub library_folder_id: i64,
    pub media_type: MediaType,
    pub title: String,
    pub file_path: String,
    pub file_size: i64,
    pub added_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create media item request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMediaItem {
    pub library_folder_id: i64,
    pub media_type: MediaType,
    pub title: String,
    pub file_path: String,
    pub file_size: i64,
}

impl MediaItem {
    /// Create a new media item in the database
    pub async fn create(db: &sqlx::SqlitePool, item: CreateMediaItem) -> Result<Self, sqlx::Error> {
        let result = sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO media_items (library_folder_id, media_type, title, file_path, file_size)
            VALUES (?, ?, ?, ?, ?)
            RETURNING *
            "#,
        )
        .bind(item.library_folder_id)
        .bind(item.media_type)
        .bind(item.title)
        .bind(item.file_path)
        .bind(item.file_size)
        .fetch_one(db)
        .await?;

        Ok(result)
    }

    /// Find media item by ID
    pub async fn find_by_id(db: &sqlx::SqlitePool, id: i64) -> Result<Option<Self>, sqlx::Error> {
        let result = sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM media_items WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(db)
        .await?;

        Ok(result)
    }

    /// Find media item by file path
    pub async fn find_by_path(
        db: &sqlx::SqlitePool,
        path: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        let result = sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM media_items WHERE file_path = ?
            "#,
        )
        .bind(path)
        .fetch_optional(db)
        .await?;

        Ok(result)
    }

    /// List all media items
    pub async fn list_all(db: &sqlx::SqlitePool) -> Result<Vec<Self>, sqlx::Error> {
        let results = sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM media_items ORDER BY added_at DESC
            "#,
        )
        .fetch_all(db)
        .await?;

        Ok(results)
    }

    /// List all media items by type
    pub async fn list_by_type(
        db: &sqlx::SqlitePool,
        media_type: MediaType,
    ) -> Result<Vec<Self>, sqlx::Error> {
        let results = sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM media_items WHERE media_type = ? ORDER BY added_at DESC
            "#,
        )
        .bind(media_type)
        .fetch_all(db)
        .await?;

        Ok(results)
    }

    /// Update media item
    pub async fn update(&self, db: &sqlx::SqlitePool) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE media_items
            SET title = ?, updated_at = CURRENT_TIMESTAMP
            WHERE id = ?
            "#,
        )
        .bind(&self.title)
        .bind(self.id)
        .execute(db)
        .await?;

        Ok(())
    }

    /// Delete media item
    pub async fn delete(db: &sqlx::SqlitePool, id: i64) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            DELETE FROM media_items WHERE id = ?
            "#,
        )
        .bind(id)
        .execute(db)
        .await?;

        Ok(())
    }
}
