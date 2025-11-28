use axum::Router;

use crate::Ctx;

pub mod health;
pub mod library;
pub mod library_folders;
pub mod organizer;
pub mod scraper;

/// Mount all API routes
pub fn mount() -> Router<Ctx> {
    Router::new()
        .merge(health::mount())
        .merge(library::mount())
        .merge(library_folders::mount())
        .merge(organizer::mount())
        .merge(scraper::mount())
}
