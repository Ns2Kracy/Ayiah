use std::{env, path::PathBuf, sync::Arc};

use axum::{Router, http::HeaderName, middleware};
use tokio::net::TcpListener;
use tower_http::{
    compression::CompressionLayer,
    cors::CorsLayer,
    propagate_header::PropagateHeaderLayer,
    request_id::{MakeRequestUuid, SetRequestIdLayer},
    services::{ServeDir, ServeFile},
};
use tracing::info;

use ayiah::{
    Context,
    app::config::ConfigManager,
    db,
    middleware::logger as middleware_logger,
    routes,
    scraper::{ScraperManager, TmdbProvider},
    services::MetadataAgent,
    utils::{graceful_shutdown::shutdown_signal, logger},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let config_path = env::var("AYIAH_CONFIG_PATH").map(PathBuf::from).ok();

    // Initialize config manager
    let config_manager = ConfigManager::init(config_path)?;

    // Initialize logging with configuration
    // Note: we're passing the manager directly as required by the logging module
    logger::init(&config_manager.read().logging)
        .map_err(|e| format!("Logging initialization error: {e}"))?;

    let conn = db::init().await?;

    // Initialize scraper manager and metadata agent
    let (scraper_manager, metadata_agent) = {
        let config = config_manager.read();

        if let Some(tmdb_api_key) = &config.scraper.tmdb_api_key {
            let mut scraper_manager = ScraperManager::new();

            // Add TMDB provider
            let tmdb_provider = TmdbProvider::new(tmdb_api_key.clone());
            scraper_manager.add_provider(tmdb_provider);

            let scraper_manager = Arc::new(scraper_manager);
            let metadata_agent =
                Arc::new(MetadataAgent::new(scraper_manager.clone(), conn.clone()));

            info!("Initialized scraper manager with TMDB provider");
            (Some(scraper_manager), Some(metadata_agent))
        } else {
            info!("No TMDB API key configured, metadata fetching disabled");
            (None, None)
        }
    };

    // Create shared application state
    let ctx = Arc::new(Context {
        db: conn,
        config: config_manager.clone(),
        scraper_manager,
        metadata_agent,
    });

    // Create application router
    let app = Router::new()
        .merge(routes::mount())
        .fallback_service(
            ServeDir::new("/dist").not_found_service(ServeFile::new("/dist/index.html")),
        )
        .with_state(ctx)
        .layer(middleware::from_fn(middleware_logger))
        .layer(CompressionLayer::new())
        .layer(PropagateHeaderLayer::new(HeaderName::from_static(
            "x-request-id",
        )))
        .layer(SetRequestIdLayer::new(
            HeaderName::from_static("x-request-id"),
            MakeRequestUuid,
        ))
        .layer(CorsLayer::permissive());

    // Parse host:port string into SocketAddr
    let address = config_manager.socket_addr()?;

    info!("Server listening on {}", &address);

    let listener = TcpListener::bind(address).await?;

    axum::serve(listener, app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}
