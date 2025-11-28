use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};

use crate::{
    ApiResponse, Ctx,
    scraper::{MediaInfo, MediaMetadata, MediaType, ScoredMatch},
};

/// Search request parameters
#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    /// Search query string
    pub query: String,
    /// Optional year filter
    pub year: Option<i32>,
    /// Optional media type filter: movie, tv, anime
    #[serde(rename = "type")]
    pub media_type: Option<String>,
    /// Maximum number of results (default: 20)
    pub limit: Option<usize>,
}

/// Search result response
#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
    pub total: usize,
}

/// Single search result
#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub id: String,
    pub title: String,
    pub original_title: Option<String>,
    pub year: Option<i32>,
    pub media_type: String,
    pub poster: Option<String>,
    pub overview: Option<String>,
    pub rating: Option<f64>,
    pub provider: String,
    pub score: i32,
    pub confidence: String,
}

impl From<ScoredMatch> for SearchResult {
    fn from(m: ScoredMatch) -> Self {
        Self {
            id: m.info.id.clone(),
            title: m.info.title.clone(),
            original_title: m.info.original_title.clone(),
            year: m.info.year,
            media_type: m.info.media_type.to_string(),
            poster: m.info.poster_url.clone(),
            overview: m.info.overview.clone(),
            rating: m.info.rating,
            provider: m.info.provider.clone(),
            score: m.score,
            confidence: format!("{:?}", m.confidence),
        }
    }
}

/// Metadata request
#[derive(Debug, Deserialize)]
pub struct MetadataRequest {
    /// Provider ID (tmdb, anilist, bangumi)
    pub provider: String,
    /// Media ID from the provider
    pub id: String,
    /// Media type: movie, tv, anime
    #[serde(rename = "type")]
    pub media_type: String,
}

/// Episode request parameters
#[derive(Debug, Deserialize)]
pub struct EpisodeQuery {
    /// Provider ID
    pub provider: String,
    /// Series ID from the provider
    pub series_id: String,
    /// Season number
    pub season: i32,
    /// Episode number
    pub episode: i32,
}

/// Episode response
#[derive(Debug, Serialize)]
pub struct EpisodeResponse {
    pub id: String,
    pub title: String,
    pub season: i32,
    pub episode: i32,
    pub absolute_number: Option<i32>,
    pub air_date: Option<String>,
    pub overview: Option<String>,
    pub runtime: Option<i32>,
    pub rating: Option<f64>,
    pub still_url: Option<String>,
}

/// Parse filename request
#[derive(Debug, Deserialize)]
pub struct ParseRequest {
    pub filename: String,
}

/// Parse response
#[derive(Debug, Serialize)]
pub struct ParseResponse {
    pub title: String,
    pub original_title: String,
    pub year: Option<i32>,
    pub season: Option<i32>,
    pub episode: Option<i32>,
    pub resolution: Option<String>,
    pub quality: Option<String>,
    pub codec: Option<String>,
    pub release_group: Option<String>,
    pub hint: String,
}

/// Provider info
#[derive(Debug, Serialize)]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    pub supported_types: Vec<String>,
    pub requires_api_key: bool,
}

/// Providers response
#[derive(Debug, Serialize)]
pub struct ProvidersResponse {
    pub providers: Vec<ProviderInfo>,
}

// ============ Handlers ============

/// Search for media
/// GET /api/scraper/search?query=...&year=...&type=...
async fn search(
    State(ctx): State<Ctx>,
    Query(params): Query<SearchQuery>,
) -> Result<Json<ApiResponse<SearchResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
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

    let media_type = params.media_type.as_deref().and_then(parse_media_type);

    let results = scraper
        .search_ranked(&params.query, params.year, media_type)
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

    let limit = params.limit.unwrap_or(20);
    let results: Vec<SearchResult> = results.into_iter().take(limit).map(Into::into).collect();
    let total = results.len();

    Ok(Json(ApiResponse {
        code: 200,
        message: "Search completed".to_string(),
        data: Some(SearchResponse { results, total }),
    }))
}

/// Get metadata for a specific media
/// POST /api/scraper/metadata
async fn get_metadata(
    State(ctx): State<Ctx>,
    Json(req): Json<MetadataRequest>,
) -> Result<Json<ApiResponse<MediaMetadata>>, (StatusCode, Json<ApiResponse<()>>)> {
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

    let media_type = parse_media_type(&req.media_type).unwrap_or(MediaType::Unknown);

    let info = MediaInfo::new(&req.id, "", &req.provider).with_type(media_type);

    let metadata = scraper.get_metadata(&info).await.map_err(|e| {
        (
            StatusCode::NOT_FOUND,
            Json(ApiResponse {
                code: 404,
                message: format!("Metadata not found: {e}"),
                data: None,
            }),
        )
    })?;

    Ok(Json(ApiResponse {
        code: 200,
        message: "Metadata retrieved".to_string(),
        data: Some(metadata),
    }))
}

/// Get episode details
/// GET /api/scraper/episode?provider=...&series_id=...&season=...&episode=...
async fn get_episode(
    State(ctx): State<Ctx>,
    Query(params): Query<EpisodeQuery>,
) -> Result<Json<ApiResponse<EpisodeResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
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

    let episode = scraper
        .get_episode(
            &params.provider,
            &params.series_id,
            params.season,
            params.episode,
        )
        .await
        .map_err(|e| {
            (
                StatusCode::NOT_FOUND,
                Json(ApiResponse {
                    code: 404,
                    message: format!("Episode not found: {e}"),
                    data: None,
                }),
            )
        })?;

    Ok(Json(ApiResponse {
        code: 200,
        message: "Episode retrieved".to_string(),
        data: Some(EpisodeResponse {
            id: episode.id,
            title: episode.title,
            season: episode.season,
            episode: episode.episode,
            absolute_number: episode.absolute_number,
            air_date: episode.air_date,
            overview: episode.overview,
            runtime: episode.runtime,
            rating: episode.rating,
            still_url: episode.still_url,
        }),
    }))
}

/// Parse a filename to extract media info
/// POST /api/scraper/parse
async fn parse_filename(Json(req): Json<ParseRequest>) -> Json<ApiResponse<ParseResponse>> {
    use crate::scraper::Parser;
    use std::path::PathBuf;

    let path = PathBuf::from(&req.filename);
    let parsed = Parser::parse(&path);

    Json(ApiResponse {
        code: 200,
        message: "Filename parsed".to_string(),
        data: Some(ParseResponse {
            title: parsed.title,
            original_title: parsed.original_title,
            year: parsed.year,
            season: parsed.season,
            episode: parsed.episode,
            resolution: parsed.resolution,
            quality: parsed.quality,
            codec: parsed.codec,
            release_group: parsed.release_group,
            hint: format!("{:?}", parsed.hint),
        }),
    })
}

/// Scrape metadata from a filename
/// POST /api/scraper/scrape
async fn scrape_from_filename(
    State(ctx): State<Ctx>,
    Json(req): Json<ParseRequest>,
) -> Result<Json<ApiResponse<SearchResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    use crate::scraper::Parser;
    use std::path::PathBuf;

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

    let path = PathBuf::from(&req.filename);
    let parsed = Parser::parse(&path);

    let media_type = match parsed.hint {
        crate::scraper::MediaHint::Movie => Some(MediaType::Movie),
        crate::scraper::MediaHint::TvShow => Some(MediaType::Tv),
        crate::scraper::MediaHint::Anime => Some(MediaType::Anime),
        crate::scraper::MediaHint::Unknown => None,
    };

    let results = scraper
        .search_ranked(&parsed.title, parsed.year, media_type)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse {
                    code: 500,
                    message: format!("Scrape failed: {e}"),
                    data: None,
                }),
            )
        })?;

    let results: Vec<SearchResult> = results.into_iter().take(10).map(Into::into).collect();
    let total = results.len();

    Ok(Json(ApiResponse {
        code: 200,
        message: "Scrape completed".to_string(),
        data: Some(SearchResponse { results, total }),
    }))
}

/// List available providers
/// GET /api/scraper/providers
async fn list_providers(
    State(ctx): State<Ctx>,
) -> Result<Json<ApiResponse<ProvidersResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
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

    let providers: Vec<ProviderInfo> = scraper
        .providers()
        .iter()
        .map(|p| ProviderInfo {
            id: p.id().to_string(),
            name: p.name().to_string(),
            supported_types: p.supported_types().iter().map(|t| t.to_string()).collect(),
            requires_api_key: p.requires_api_key(),
        })
        .collect();

    Ok(Json(ApiResponse {
        code: 200,
        message: "Providers listed".to_string(),
        data: Some(ProvidersResponse { providers }),
    }))
}

/// Refresh metadata for a media item by ID
/// POST /api/scraper/refresh/{id}
async fn refresh_item_metadata(
    State(ctx): State<Ctx>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<String>>, (StatusCode, Json<ApiResponse<()>>)> {
    let agent = ctx.metadata_agent.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ApiResponse {
                code: 503,
                message: "Metadata agent not available".to_string(),
                data: None,
            }),
        )
    })?;

    agent.refresh_metadata(id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse {
                code: 500,
                message: format!("Refresh failed: {e}"),
                data: None,
            }),
        )
    })?;

    Ok(Json(ApiResponse {
        code: 200,
        message: "Metadata refreshed".to_string(),
        data: Some("OK".to_string()),
    }))
}

// ============ Helpers ============

fn parse_media_type(s: &str) -> Option<MediaType> {
    match s.to_lowercase().as_str() {
        "movie" => Some(MediaType::Movie),
        "tv" | "tvshow" | "series" => Some(MediaType::Tv),
        "anime" => Some(MediaType::Anime),
        _ => None,
    }
}

/// Mount scraper routes
pub fn mount() -> Router<Ctx> {
    Router::new()
        .route("/scraper/search", get(search))
        .route("/scraper/metadata", post(get_metadata))
        .route("/scraper/episode", get(get_episode))
        .route("/scraper/parse", post(parse_filename))
        .route("/scraper/scrape", post(scrape_from_filename))
        .route("/scraper/providers", get(list_providers))
        .route("/scraper/refresh/{id}", post(refresh_item_metadata))
}
