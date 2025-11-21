pub mod health;
pub mod albums;
pub mod auth;
pub mod jobs;
pub mod settings;

use axum::{
    routing::{get, post, patch, put},
    Router,
};

use crate::state::AppState;

pub fn api_routes() -> Router<AppState> {
    Router::new()
        // Auth endpoints
        .route("/auth/spotify/authorize", get(auth::authorize))
        .route("/auth/spotify/callback", post(auth::callback))

        // Album endpoints
        .route("/albums", get(albums::list_albums))
        .route("/albums/:id", get(albums::get_album))
        .route("/albums/:id", patch(albums::update_album))
        .route("/albums/:id/match", post(albums::trigger_match))
        .route("/albums/:id/search-lidarr", post(albums::search_lidarr))

        // Job endpoints
        .route("/jobs", get(jobs::list_jobs))
        .route("/jobs/:id/status", get(jobs::get_job_status))
        .route("/jobs/spotify-sync", post(jobs::trigger_spotify_sync))
        .route("/jobs/musicbrainz-match-all", post(jobs::trigger_musicbrainz_match))

        // Settings endpoints
        .route("/settings", get(settings::get_settings))
        .route("/settings", put(settings::update_settings))
        .route("/settings/test-lidarr", post(settings::test_lidarr_connection))

        // Statistics
        .route("/stats", get(albums::get_stats))
}
