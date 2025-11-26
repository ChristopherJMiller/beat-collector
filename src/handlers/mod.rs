pub mod health;
pub mod albums;
pub mod artists;
pub mod auth;
pub mod jobs;
pub mod playlists;
pub mod settings;
pub mod html;
pub mod lidarr;

use axum::{
    routing::{get, post, patch, put},
    Router,
};

use crate::state::AppState;

/// HTML page routes (MASH stack)
pub fn html_routes() -> Router<AppState> {
    Router::new()
        // Main pages
        .route("/", get(html::index))
        .route("/artists", get(html::artists))
        .route("/artists/:id", get(html::artist_detail))
        .route("/settings", get(html::settings))
        .route("/jobs", get(html::jobs))
        .route("/stats", get(html::stats))
        .route("/playlists", get(html::playlists))

        // OAuth callback (GET with query params from Spotify)
        .route("/auth/callback", get(auth::callback))

        // HTMX partials
        .route("/albums", get(html::albums_grid))
        .route("/albums/:id", get(html::album_detail))
        .route("/artists-grid", get(html::artists_grid))
        .route("/playlists-grid", get(html::playlists_grid))
        .route("/playlists/:id", get(html::playlist_detail))
        .route("/playlists/:id/toggle", post(html::playlist_toggle))
        .route("/playlists/:id/tracks", get(html::playlist_tracks_partial))
}

/// JSON API routes (for programmatic access)
pub fn api_routes() -> Router<AppState> {
    Router::new()
        // Auth endpoints
        .route("/auth/spotify/authorize", get(auth::authorize))
        .route("/auth/spotify/status", get(auth::spotify_status))
        .route("/auth/spotify/button", get(auth::spotify_button))

        // Album endpoints
        .route("/albums", get(albums::list_albums))
        .route("/albums/:id", get(albums::get_album))
        .route("/albums/:id", patch(albums::update_album))
        .route("/albums/:id/match", post(albums::trigger_match))
        .route("/albums/:id/search-lidarr", post(albums::search_lidarr))

        // Playlist endpoints
        .route("/playlists", get(playlists::list_playlists))
        .route("/playlists/:id", get(playlists::get_playlist))
        .route("/playlists/:id/tracks", get(playlists::get_playlist_tracks))
        .route("/playlists/:id/toggle", post(playlists::toggle_playlist_enabled))

        // Job endpoints
        .route("/jobs", get(jobs::list_jobs))
        .route("/jobs/:id/status", get(jobs::get_job_status))
        .route("/jobs/spotify-sync", post(jobs::trigger_spotify_sync))
        .route("/jobs/musicbrainz-match-all", post(jobs::trigger_musicbrainz_match))

        // Settings endpoints
        .route("/settings", get(settings::get_settings))
        .route("/settings", put(settings::update_settings))
        .route("/settings/test-lidarr", post(settings::test_lidarr_connection))

        // Lidarr webhook
        .route("/webhooks/lidarr", post(lidarr::webhook))

        // Artist endpoints
        .route("/artists", get(artists::list_artists))
        .route("/artists/:id", get(artists::get_artist))

        // Statistics
        .route("/stats", get(albums::get_stats))
}
