#![forbid(unsafe_code)]
#![allow(dead_code)]

use std::sync::Arc;

use app::config::ConfigManager;
use axum::{
    Json,
    response::{IntoResponse, Response},
};
use error::AyiahError;
use serde::{Deserialize, Serialize};

pub mod app;
pub mod db;
pub mod entities;
pub mod error;
pub mod middleware;
pub mod routes;
pub mod scraper;
pub mod services;
pub mod utils;

pub type ApiResult<T> = std::result::Result<ApiResponse<T>, AyiahError>;
pub type Ctx = Arc<Context>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub code: u16,
    pub message: String,
    pub data: Option<T>,
}

impl<T> IntoResponse for ApiResponse<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}

/// Context holds all shared application resources
#[derive(Clone)]
pub struct Context {
    /// Shared configuration manager
    pub config: ConfigManager,

    /// Database connection
    pub db: db::Database,

    /// Scraper manager for metadata fetching
    pub scraper_manager: Option<Arc<scraper::ScraperManager>>,

    /// Metadata agent for fetching and saving metadata
    pub metadata_agent: Option<Arc<services::MetadataAgent>>,
}
