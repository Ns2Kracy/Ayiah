use axum::{Json, Router, extract::State, http::StatusCode, routing::post};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::{
    ApiResponse, Ctx,
    scraper::{NamingTemplate, OrganizeMethod, Organizer, OrganizerConfig},
};

/// Organize request
#[derive(Debug, Deserialize)]
pub struct OrganizeRequest {
    /// Source directory containing media files
    pub source: String,
    /// Target directory for organized files
    pub target: String,
    /// Organization method: symlink, hardlink, move, copy
    #[serde(default)]
    pub method: String,
    /// Whether to separate by media type (Movies/TV/Anime)
    #[serde(default = "default_true")]
    pub separate_by_type: bool,
    /// Dry run mode (preview without making changes)
    #[serde(default)]
    pub dry_run: bool,
    /// Overwrite existing files
    #[serde(default)]
    pub overwrite: bool,
    /// Custom naming templates (optional)
    pub templates: Option<TemplateConfig>,
}

const fn default_true() -> bool {
    true
}

/// Custom naming templates
#[derive(Debug, Deserialize)]
pub struct TemplateConfig {
    /// Movie folder template, e.g., "{title} ({year})"
    pub movie_folder: Option<String>,
    /// Movie file template
    pub movie_file: Option<String>,
    /// TV show folder template
    pub tv_folder: Option<String>,
    /// Season folder template, e.g., "Season {season:02}"
    pub season_folder: Option<String>,
    /// Episode file template, e.g., "{title} - S{season:02}E{episode:02}"
    pub episode_file: Option<String>,
}

/// Organize response
#[derive(Debug, Serialize)]
pub struct OrganizeResponse {
    /// Total files processed
    pub total: usize,
    /// Successfully organized
    pub success: usize,
    /// Failed to organize
    pub failed: usize,
    /// Skipped files
    pub skipped: usize,
    /// Details of organized files
    pub results: Vec<OrganizedFile>,
    /// Errors encountered
    pub errors: Vec<OrganizeError>,
}

/// Single organized file result
#[derive(Debug, Serialize)]
pub struct OrganizedFile {
    pub source: String,
    pub target: String,
    pub title: String,
    pub media_type: String,
    pub season: Option<i32>,
    pub episode: Option<i32>,
}

/// Organize error
#[derive(Debug, Serialize)]
pub struct OrganizeError {
    pub source: String,
    pub error: String,
}

/// Preview organize request (same as organize but always dry run)
#[derive(Debug, Deserialize)]
pub struct PreviewRequest {
    /// Source directory containing media files
    pub source: String,
    /// Target directory for organized files
    pub target: String,
    /// Organization method: symlink, hardlink, move, copy
    #[serde(default)]
    pub method: String,
    /// Whether to separate by media type
    #[serde(default = "default_true")]
    pub separate_by_type: bool,
    /// Custom naming templates
    pub templates: Option<TemplateConfig>,
}

/// Organize media files
/// POST /api/organizer/organize
async fn organize(
    State(_ctx): State<Ctx>,
    Json(req): Json<OrganizeRequest>,
) -> Result<Json<ApiResponse<OrganizeResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    // Parse method
    let method = req.method.parse::<OrganizeMethod>().unwrap_or_default();

    // Build naming template
    let mut template = NamingTemplate::default();
    if let Some(ref t) = req.templates {
        if let Some(ref s) = t.movie_folder {
            template.movie_folder = s.clone();
        }
        if let Some(ref s) = t.movie_file {
            template.movie_file = s.clone();
        }
        if let Some(ref s) = t.tv_folder {
            template.tv_folder = s.clone();
        }
        if let Some(ref s) = t.season_folder {
            template.season_folder = s.clone();
        }
        if let Some(ref s) = t.episode_file {
            template.episode_file = s.clone();
        }
    }

    // Build config
    let config = OrganizerConfig {
        source_dir: PathBuf::from(&req.source),
        target_dir: PathBuf::from(&req.target),
        method,
        template,
        separate_by_type: req.separate_by_type,
        dry_run: req.dry_run,
        overwrite: req.overwrite,
    };

    // Validate paths
    if !config.source_dir.exists() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse {
                code: 400,
                message: format!("Source directory does not exist: {}", req.source),
                data: None,
            }),
        ));
    }

    // Create organizer
    let organizer = Organizer::new(config);

    // Run organize
    let result = organizer.organize_all().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse {
                code: 500,
                message: format!("Organize failed: {e}"),
                data: None,
            }),
        )
    })?;

    // Build response
    let mut results = Vec::new();
    let mut errors = Vec::new();

    for r in &result.success {
        results.push(OrganizedFile {
            source: r.source.display().to_string(),
            target: r.target.display().to_string(),
            title: r
                .metadata
                .as_ref()
                .map_or_else(|| r.parsed.title.clone(), |m| m.title.clone()),
            media_type: r.metadata.as_ref().map_or_else(
                || format!("{:?}", r.parsed.hint),
                |m| m.media_type.to_string(),
            ),
            season: r.parsed.season,
            episode: r.parsed.episode,
        });
    }

    for r in &result.failed {
        errors.push(OrganizeError {
            source: r.source.display().to_string(),
            error: r
                .error
                .clone()
                .unwrap_or_else(|| "Unknown error".to_string()),
        });
    }

    for (path, reason) in &result.skipped {
        errors.push(OrganizeError {
            source: path.display().to_string(),
            error: format!("Skipped: {reason}"),
        });
    }

    let response = OrganizeResponse {
        total: result.total(),
        success: result.success_count(),
        failed: result.failed_count(),
        skipped: result.skipped.len(),
        results,
        errors,
    };

    let message = if req.dry_run {
        format!(
            "[DRY RUN] Would organize {} files ({} success, {} failed)",
            response.total, response.success, response.failed
        )
    } else {
        format!(
            "Organized {} files ({} success, {} failed)",
            response.total, response.success, response.failed
        )
    };

    Ok(Json(ApiResponse {
        code: 200,
        message,
        data: Some(response),
    }))
}

/// Preview organize operation (dry run)
/// POST /api/organizer/preview
async fn preview(
    State(ctx): State<Ctx>,
    Json(req): Json<PreviewRequest>,
) -> Result<Json<ApiResponse<OrganizeResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    // Convert to organize request with dry_run = true
    let organize_req = OrganizeRequest {
        source: req.source,
        target: req.target,
        method: req.method,
        separate_by_type: req.separate_by_type,
        dry_run: true,
        overwrite: false,
        templates: req.templates,
    };

    organize(State(ctx), Json(organize_req)).await
}

/// Mount organizer routes
pub fn mount() -> Router<Ctx> {
    Router::new()
        .route("/organizer/organize", post(organize))
        .route("/organizer/preview", post(preview))
}
