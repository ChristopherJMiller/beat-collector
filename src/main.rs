use anyhow::Result;
use axum::{
    routing::get,
    Router,
};
use dotenvy::dotenv;
use migration::MigratorTrait;
use sea_orm::{Database, DatabaseConnection};
use std::net::SocketAddr;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
    compression::CompressionLayer,
    services::ServeDir,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod db;
mod error;
mod handlers;
mod jobs;
mod services;
mod state;
mod tasks;
mod templates;

use config::Config;
use state::AppState;

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables
    dotenv().ok();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "beat_collector=debug,tower_http=debug,axum=trace".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting Beat Collector...");

    // Load configuration
    let config = Config::from_env()?;
    tracing::info!("Configuration loaded");

    // Connect to database
    let db = Database::connect(&config.database_url).await?;
    tracing::info!("Connected to database");

    // Run migrations
    migration::Migrator::up(&db, None).await?;
    tracing::info!("Database migrations completed");

    // Connect to Redis
    let redis_client = redis::Client::open(config.redis_url.as_str())?;
    let redis_conn = redis_client.get_connection_manager().await?;
    tracing::info!("Connected to Redis");

    // Initialize job queue and executor
    let (job_queue, job_receiver) = jobs::JobQueue::new();
    tracing::info!("Job queue initialized");

    // Initialize application state
    let state = AppState::new(db, redis_conn, config.clone(), job_queue);

    // Start job executor
    let executor = jobs::JobExecutor::new(state.clone(), job_receiver);
    tokio::spawn(async move {
        executor.start().await;
    });
    tracing::info!("Job executor started");

    // Start background tasks
    let task_scheduler = tasks::start_scheduler(state.clone()).await?;
    tracing::info!("Background task scheduler started");

    // Build application routes
    let app = create_router(state.clone());

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.server_port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("Server listening on {}", addr);

    axum::serve(listener, app)
        .await?;

    Ok(())
}

fn create_router(state: AppState) -> Router {
    Router::new()
        // Health check
        .route("/health", get(handlers::health::health_check))

        // API routes (JSON)
        .nest("/api", handlers::api_routes())

        // HTML routes (MASH stack - Maud + HTMX)
        .merge(handlers::html_routes())

        // Static file serving for cover art and assets
        .nest_service("/static", ServeDir::new("static"))

        // Middleware
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
