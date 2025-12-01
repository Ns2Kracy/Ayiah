use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use super::MediaType;

/// Library folder entity
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LibraryFolder {
    pub id: i64,
    pub name: String,
    pub path: String,
    pub media_type: MediaType,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create library folder request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateLibraryFolder {
    pub name: String,
    pub path: String,
    pub media_type: MediaType,
}

impl LibraryFolder {
    /// Create a new library folder
    pub async fn create(
        db: &sqlx::SqlitePool,
        folder: CreateLibraryFolder,
    ) -> Result<Self, sqlx::Error> {
        let result = sqlx::query_as::<_, Self>(
            r"
            INSERT INTO library_folders (name, path, media_type)
            VALUES (?, ?, ?)
            RETURNING *
            ",
        )
        .bind(folder.name)
        .bind(folder.path)
        .bind(folder.media_type)
        .fetch_one(db)
        .await?;

        Ok(result)
    }

    /// Find library folder by ID
    pub async fn find_by_id(db: &sqlx::SqlitePool, id: i64) -> Result<Option<Self>, sqlx::Error> {
        let result = sqlx::query_as::<_, Self>(
            r"
            SELECT * FROM library_folders WHERE id = ?
            ",
        )
        .bind(id)
        .fetch_optional(db)
        .await?;

        Ok(result)
    }

    /// List all library folders
    pub async fn list_all(db: &sqlx::SqlitePool) -> Result<Vec<Self>, sqlx::Error> {
        let results = sqlx::query_as::<_, Self>(
            r"
            SELECT * FROM library_folders ORDER BY created_at DESC
            ",
        )
        .fetch_all(db)
        .await?;

        Ok(results)
    }

    /// List enabled library folders
    pub async fn list_enabled(db: &sqlx::SqlitePool) -> Result<Vec<Self>, sqlx::Error> {
        let results = sqlx::query_as::<_, Self>(
            r"
            SELECT * FROM library_folders WHERE enabled = 1 ORDER BY created_at DESC
            ",
        )
        .fetch_all(db)
        .await?;

        Ok(results)
    }

    /// Update library folder
    pub async fn update(&self, db: &sqlx::SqlitePool) -> Result<(), sqlx::Error> {
        sqlx::query(
            r"
            UPDATE library_folders
            SET name = ?, path = ?, media_type = ?, enabled = ?, updated_at = CURRENT_TIMESTAMP
            WHERE id = ?
            ",
        )
        .bind(&self.name)
        .bind(&self.path)
        .bind(self.media_type)
        .bind(self.enabled)
        .bind(self.id)
        .execute(db)
        .await?;

        Ok(())
    }

    /// Delete library folder
    pub async fn delete(db: &sqlx::SqlitePool, id: i64) -> Result<(), sqlx::Error> {
        sqlx::query(
            r"
            DELETE FROM library_folders WHERE id = ?
            ",
        )
        .bind(id)
        .execute(db)
        .await?;

        Ok(())
    }
}
