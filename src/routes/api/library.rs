use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};

use crate::{
    ApiResponse, ApiResult, Ctx,
    entities::{MediaItem, MediaItemWithMetadata, MediaType},
};

/// Library API response
#[derive(Debug, Serialize, Deserialize)]
pub struct LibraryResponse {
    pub items: Vec<MediaItemWithMetadata>,
    pub total: usize,
}

/// Query parameters for library listing
#[derive(Debug, Deserialize)]
pub struct LibraryQuery {
    /// Page number (1-indexed)
    pub page: Option<u32>,
    /// Items per page
    pub limit: Option<u32>,
    /// Sort by field: title, year, rating, added
    pub sort: Option<String>,
    /// Sort order: asc, desc
    pub order: Option<String>,
    /// Search query
    pub search: Option<String>,
}

/// Identify request - match a media item with online metadata
#[derive(Debug, Deserialize)]
pub struct IdentifyRequest {
    /// Provider to use (tmdb, anilist, bangumi)
    pub provider: String,
    /// Provider's media ID
    pub provider_id: String,
    /// Media type
    #[serde(rename = "type")]
    pub media_type: String,
}

/// Batch refresh request
#[derive(Debug, Deserialize)]
pub struct BatchRefreshRequest {
    /// List of media item IDs to refresh
    pub ids: Vec<i64>,
}

/// Batch refresh response
#[derive(Debug, Serialize)]
pub struct BatchRefreshResponse {
    pub success: Vec<i64>,
    pub failed: Vec<BatchRefreshError>,
}

#[derive(Debug, Serialize)]
pub struct BatchRefreshError {
    pub id: i64,
    pub error: String,
}

/// Get movies
async fn get_movies(
    State(ctx): State<Ctx>,
    Query(params): Query<LibraryQuery>,
) -> ApiResult<LibraryResponse> {
    let items = MediaItemWithMetadata::list_by_type(&ctx.db, MediaType::Movie)
        .await
        .map_err(|e| {
            crate::error::AyiahError::DatabaseError(format!("Failed to fetch movies: {e}"))
        })?;

    let items = apply_filters_and_sort(items, &params);
    let total = items.len();

    Ok(ApiResponse {
        code: 200,
        message: "Movies retrieved successfully".to_string(),
        data: Some(LibraryResponse { items, total }),
    })
}

/// Get TV shows
async fn get_tv_shows(
    State(ctx): State<Ctx>,
    Query(params): Query<LibraryQuery>,
) -> ApiResult<LibraryResponse> {
    let items = MediaItemWithMetadata::list_by_type(&ctx.db, MediaType::Tv)
        .await
        .map_err(|e| {
            crate::error::AyiahError::DatabaseError(format!("Failed to fetch TV shows: {e}"))
        })?;

    let items = apply_filters_and_sort(items, &params);
    let total = items.len();

    Ok(ApiResponse {
        code: 200,
        message: "TV shows retrieved successfully".to_string(),
        data: Some(LibraryResponse { items, total }),
    })
}

/// Get all media items
async fn get_all_items(
    State(ctx): State<Ctx>,
    Query(params): Query<LibraryQuery>,
) -> ApiResult<LibraryResponse> {
    let items = MediaItemWithMetadata::list_all(&ctx.db)
        .await
        .map_err(|e| {
            crate::error::AyiahError::DatabaseError(format!("Failed to fetch items: {e}"))
        })?;

    let items = apply_filters_and_sort(items, &params);
    let total = items.len();

    Ok(ApiResponse {
        code: 200,
        message: "Items retrieved successfully".to_string(),
        data: Some(LibraryResponse { items, total }),
    })
}

/// Get media item by ID
async fn get_media_item(
    State(ctx): State<Ctx>,
    Path(id): Path<i64>,
) -> ApiResult<MediaItemWithMetadata> {
    let item = MediaItemWithMetadata::find_by_id(&ctx.db, id)
        .await
        .map_err(|e| {
            crate::error::AyiahError::DatabaseError(format!("Failed to fetch media item: {e}"))
        })?
        .ok_or_else(|| {
            crate::error::AyiahError::ApiError(crate::error::ApiError::NotFound(format!(
                "Media item with ID {id} not found"
            )))
        })?;

    Ok(ApiResponse {
        code: 200,
        message: "Media item retrieved successfully".to_string(),
        data: Some(item),
    })
}

/// Refresh metadata for a media item
async fn refresh_metadata(
    State(ctx): State<Ctx>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<String>>, (StatusCode, Json<ApiResponse<String>>)> {
    let metadata_agent = ctx.metadata_agent.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ApiResponse {
                code: 503,
                message: "Metadata agent not available".to_string(),
                data: None,
            }),
        )
    })?;

    match metadata_agent.refresh_metadata(id).await {
        Ok(_) => Ok(Json(ApiResponse {
            code: 200,
            message: "Metadata refreshed successfully".to_string(),
            data: Some("Metadata updated".to_string()),
        })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse {
                code: 500,
                message: format!("Failed to refresh metadata: {e}"),
                data: None,
            }),
        )),
    }
}

/// Batch refresh metadata for multiple items
async fn batch_refresh_metadata(
    State(ctx): State<Ctx>,
    Json(req): Json<BatchRefreshRequest>,
) -> Result<Json<ApiResponse<BatchRefreshResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let metadata_agent = ctx.metadata_agent.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ApiResponse {
                code: 503,
                message: "Metadata agent not available".to_string(),
                data: None,
            }),
        )
    })?;

    let mut success = Vec::new();
    let mut failed = Vec::new();

    for id in req.ids {
        match metadata_agent.refresh_metadata(id).await {
            Ok(_) => success.push(id),
            Err(e) => failed.push(BatchRefreshError {
                id,
                error: e.to_string(),
            }),
        }
        // Small delay to avoid rate limiting
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    Ok(Json(ApiResponse {
        code: 200,
        message: format!(
            "Batch refresh completed: {} success, {} failed",
            success.len(),
            failed.len()
        ),
        data: Some(BatchRefreshResponse { success, failed }),
    }))
}

/// Identify a media item with a specific provider result
async fn identify_item(
    State(ctx): State<Ctx>,
    Path(id): Path<i64>,
    Json(req): Json<IdentifyRequest>,
) -> Result<Json<ApiResponse<String>>, (StatusCode, Json<ApiResponse<()>>)> {
    let scraper = ctx.scraper_manager.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ApiResponse {
                code: 503,
                message: "Scraper not available".to_string(),
                data: None,
            }),
        )
    })?;

    // Verify the media item exists
    let _item = MediaItem::find_by_id(&ctx.db, id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse {
                    code: 500,
                    message: format!("Database error: {e}"),
                    data: None,
                }),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ApiResponse {
                    code: 404,
                    message: format!("Media item {id} not found"),
                    data: None,
                }),
            )
        })?;

    // Parse media type
    let media_type = match req.media_type.to_lowercase().as_str() {
        "movie" => crate::scraper::MediaType::Movie,
        "tv" | "tvshow" | "series" => crate::scraper::MediaType::Tv,
        "anime" => crate::scraper::MediaType::Anime,
        _ => crate::scraper::MediaType::Unknown,
    };

    // Create MediaInfo and fetch metadata
    let info =
        crate::scraper::MediaInfo::new(&req.provider_id, "", &req.provider).with_type(media_type);

    let metadata = scraper.get_metadata(&info).await.map_err(|e| {
        (
            StatusCode::NOT_FOUND,
            Json(ApiResponse {
                code: 404,
                message: format!("Failed to fetch metadata: {e}"),
                data: None,
            }),
        )
    })?;

    // Save metadata to database
    let create_metadata = crate::entities::CreateVideoMetadata {
        media_item_id: id,
        tmdb_id: metadata
            .external_ids
            .tmdb
            .as_ref()
            .and_then(|s| s.parse().ok()),
        tvdb_id: metadata
            .external_ids
            .tvdb
            .as_ref()
            .and_then(|s| s.parse().ok()),
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

    crate::entities::VideoMetadata::upsert(&ctx.db, create_metadata)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse {
                    code: 500,
                    message: format!("Failed to save metadata: {e}"),
                    data: None,
                }),
            )
        })?;

    Ok(Json(ApiResponse {
        code: 200,
        message: "Item identified and metadata saved".to_string(),
        data: Some(format!("Identified as: {}", metadata.title)),
    }))
}

/// Search candidates for identifying a media item
async fn search_identify_candidates(
    State(ctx): State<Ctx>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<Vec<super::scraper::SearchResult>>>, (StatusCode, Json<ApiResponse<()>>)>
{
    let scraper = ctx.scraper_manager.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ApiResponse {
                code: 503,
                message: "Scraper not available".to_string(),
                data: None,
            }),
        )
    })?;

    // Get the media item
    let item = MediaItem::find_by_id(&ctx.db, id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse {
                    code: 500,
                    message: format!("Database error: {e}"),
                    data: None,
                }),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ApiResponse {
                    code: 404,
                    message: format!("Media item {id} not found"),
                    data: None,
                }),
            )
        })?;

    // Parse the title
    let parsed = crate::scraper::Parser::parse_filename(&item.title);

    // Convert media type
    let media_type = match item.media_type {
        MediaType::Movie => Some(crate::scraper::MediaType::Movie),
        MediaType::Tv => Some(crate::scraper::MediaType::Tv),
        _ => None,
    };

    // Search for candidates
    let results = scraper
        .search_ranked(&parsed.title, parsed.year, media_type)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse {
                    code: 500,
                    message: format!("Search failed: {e}"),
                    data: None,
                }),
            )
        })?;

    let candidates: Vec<super::scraper::SearchResult> =
        results.into_iter().take(20).map(Into::into).collect();

    Ok(Json(ApiResponse {
        code: 200,
        message: format!("Found {} candidates", candidates.len()),
        data: Some(candidates),
    }))
}

// ============ Helpers ============

fn apply_filters_and_sort(
    mut items: Vec<MediaItemWithMetadata>,
    params: &LibraryQuery,
) -> Vec<MediaItemWithMetadata> {
    // Apply search filter
    if let Some(ref search) = params.search {
        let search_lower = search.to_lowercase();
        items.retain(|item| item.media_item.title.to_lowercase().contains(&search_lower));
    }

    // Apply sorting
    if let Some(ref sort) = params.sort {
        let desc = params.order.as_deref() == Some("desc");
        match sort.as_str() {
            "title" => {
                items.sort_by(|a, b| {
                    let cmp = a.media_item.title.cmp(&b.media_item.title);
                    if desc { cmp.reverse() } else { cmp }
                });
            }
            "year" => {
                items.sort_by(|a, b| {
                    let year_a = a.metadata.as_ref().and_then(|m| {
                        m.release_date
                            .as_ref()
                            .and_then(|d| d.split('-').next()?.parse::<i32>().ok())
                    });
                    let year_b = b.metadata.as_ref().and_then(|m| {
                        m.release_date
                            .as_ref()
                            .and_then(|d| d.split('-').next()?.parse::<i32>().ok())
                    });
                    let cmp = year_a.cmp(&year_b);
                    if desc { cmp.reverse() } else { cmp }
                });
            }
            "rating" => {
                items.sort_by(|a, b| {
                    let rating_a = a.metadata.as_ref().and_then(|m| m.vote_average);
                    let rating_b = b.metadata.as_ref().and_then(|m| m.vote_average);
                    let cmp = rating_a
                        .partial_cmp(&rating_b)
                        .unwrap_or(std::cmp::Ordering::Equal);
                    if desc { cmp.reverse() } else { cmp }
                });
            }
            "added" => {
                items.sort_by(|a, b| {
                    let cmp = a.media_item.added_at.cmp(&b.media_item.added_at);
                    if desc { cmp.reverse() } else { cmp }
                });
            }
            _ => {}
        }
    }

    // Apply pagination
    if let (Some(page), Some(limit)) = (params.page, params.limit) {
        let start = ((page.saturating_sub(1)) * limit) as usize;
        let end = (start + limit as usize).min(items.len());
        if start < items.len() {
            items = items[start..end].to_vec();
        } else {
            items = Vec::new();
        }
    }

    items
}

/// Mount library routes
pub fn mount() -> Router<Ctx> {
    Router::new()
        .route("/library", get(get_all_items))
        .route("/library/movies", get(get_movies))
        .route("/library/tv", get(get_tv_shows))
        .route("/library/items/{id}", get(get_media_item))
        .route("/library/items/{id}/refresh", post(refresh_metadata))
        .route("/library/items/{id}/identify", post(identify_item))
        .route(
            "/library/items/{id}/candidates",
            get(search_identify_candidates),
        )
        .route("/library/batch/refresh", post(batch_refresh_metadata))
}
