//! Integration tests for album handler routes
//!
//! Tests all album-related API endpoints including:
//! - List albums with various filters and pagination
//! - Get single album
//! - Update album
//! - Search Lidarr
//! - Get stats

use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use serde_json::json;
use tower::util::ServiceExt;

// Import from the main crate
use beat_collector::db::{
    entities::{albums, artists, user_settings},
    enums::{AcquisitionSource, MatchStatus, OwnershipStatus},
};
use beat_collector::handlers;
use beat_collector::state::AppState;
use beat_collector::test_utils::*;

/// Helper to create a test router with album routes
fn create_test_router(state: &AppState) -> Router {
    Router::new()
        .nest("/api", handlers::api_routes())
        .with_state(state.clone())
}

/// Helper to parse JSON response body
async fn parse_json_response<T: serde::de::DeserializeOwned>(
    response: axum::response::Response,
) -> T {
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&body).unwrap()
}

#[tokio::test]
async fn test_list_albums_empty() {
    let state = setup_test_app_state().await;
    let app = create_test_router(&state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/albums")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = parse_json_response(response).await;
    assert_eq!(body["albums"].as_array().unwrap().len(), 0);
    assert_eq!(body["pagination"]["total_items"], 0);
    assert_eq!(body["pagination"]["page"], 1);
}

#[tokio::test]
async fn test_list_albums_with_data() {
    let state = setup_test_app_state().await;

    // Create test data
    let artist = create_test_artist(&state.db, "Test Artist", Some("spotify:artist:123")).await;
    create_test_album(&state.db, artist.id, "Album 1", Some("spotify:album:1")).await;
    create_test_album(&state.db, artist.id, "Album 2", Some("spotify:album:2")).await;
    create_test_album(&state.db, artist.id, "Album 3", Some("spotify:album:3")).await;

    let app = create_test_router(&state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/albums")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = parse_json_response(response).await;
    assert_eq!(body["albums"].as_array().unwrap().len(), 3);
    assert_eq!(body["pagination"]["total_items"], 3);
    assert_eq!(body["pagination"]["total_pages"], 1);
}

#[tokio::test]
async fn test_list_albums_pagination() {
    let state = setup_test_app_state().await;

    // Create test data
    let artist = create_test_artist(&state.db, "Test Artist", None).await;
    for i in 1..=10 {
        create_test_album(&state.db, artist.id, &format!("Album {}", i), None).await;
    }

    let app = create_test_router(&state);

    // First page with page_size=5
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/albums?page=1&page_size=5")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = parse_json_response(response).await;
    assert_eq!(body["albums"].as_array().unwrap().len(), 5);
    assert_eq!(body["pagination"]["page"], 1);
    assert_eq!(body["pagination"]["page_size"], 5);
    assert_eq!(body["pagination"]["total_items"], 10);
    assert_eq!(body["pagination"]["total_pages"], 2);

    // Second page
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/albums?page=2&page_size=5")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = parse_json_response(response).await;
    assert_eq!(body["albums"].as_array().unwrap().len(), 5);
    assert_eq!(body["pagination"]["page"], 2);
}

#[tokio::test]
async fn test_list_albums_filter_by_ownership_status() {
    let state = setup_test_app_state().await;

    let artist = create_test_artist(&state.db, "Test Artist", None).await;

    // Create albums with different ownership statuses
    let album1 = create_test_album(&state.db, artist.id, "Owned Album", None).await;
    let album2 = create_test_album(&state.db, artist.id, "Not Owned Album", None).await;
    let album3 = create_test_album(&state.db, artist.id, "Downloading Album", None).await;

    // Update ownership statuses
    let mut album1_active: albums::ActiveModel = album1.into();
    album1_active.ownership_status = Set(OwnershipStatus::Owned.as_str().to_string());
    album1_active.update(&state.db).await.unwrap();

    let mut album3_active: albums::ActiveModel = album3.into();
    album3_active.ownership_status = Set(OwnershipStatus::Downloading.as_str().to_string());
    album3_active.update(&state.db).await.unwrap();

    let app = create_test_router(&state);

    // Filter by owned
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/albums?ownership_status=owned")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = parse_json_response(response).await;
    assert_eq!(body["albums"].as_array().unwrap().len(), 1);
    assert_eq!(body["albums"][0]["title"], "Owned Album");

    // Filter by not_owned
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/albums?ownership_status=not_owned")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = parse_json_response(response).await;
    assert_eq!(body["albums"].as_array().unwrap().len(), 1);
    assert_eq!(body["albums"][0]["title"], "Not Owned Album");
}

#[tokio::test]
async fn test_list_albums_filter_by_match_status() {
    let state = setup_test_app_state().await;

    let artist = create_test_artist(&state.db, "Test Artist", None).await;

    // Create albums with different match statuses
    let album1 = create_test_album(&state.db, artist.id, "Matched Album", None).await;
    let album2 = create_test_album(&state.db, artist.id, "Pending Album", None).await;

    // Update match status
    let mut album1_active: albums::ActiveModel = album1.into();
    album1_active.match_status = Set(Some(MatchStatus::Matched.as_str().to_string()));
    album1_active.musicbrainz_release_group_id = Set(Some("mb-123".to_string()));
    album1_active.update(&state.db).await.unwrap();

    let app = create_test_router(&state);

    // Filter by matched
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/albums?match_status=matched")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = parse_json_response(response).await;
    assert_eq!(body["albums"].as_array().unwrap().len(), 1);
    assert_eq!(body["albums"][0]["title"], "Matched Album");
}

#[tokio::test]
async fn test_list_albums_filter_by_artist() {
    let state = setup_test_app_state().await;

    let artist1 = create_test_artist(&state.db, "Artist 1", None).await;
    let artist2 = create_test_artist(&state.db, "Artist 2", None).await;

    create_test_album(&state.db, artist1.id, "Album A1", None).await;
    create_test_album(&state.db, artist1.id, "Album A2", None).await;
    create_test_album(&state.db, artist2.id, "Album B1", None).await;

    let app = create_test_router(&state);

    // Filter by artist1
    let response = app
        .oneshot(
            Request::builder()
                .uri(&format!("/api/albums?artist_id={}", artist1.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = parse_json_response(response).await;
    assert_eq!(body["albums"].as_array().unwrap().len(), 2);
    assert_eq!(body["pagination"]["total_items"], 2);
}

#[tokio::test]
async fn test_list_albums_search() {
    let state = setup_test_app_state().await;

    let artist = create_test_artist(&state.db, "Test Artist", None).await;

    create_test_album(&state.db, artist.id, "Dark Side of the Moon", None).await;
    create_test_album(&state.db, artist.id, "The Wall", None).await;
    create_test_album(&state.db, artist.id, "Wish You Were Here", None).await;

    let app = create_test_router(&state);

    // Search for "Dark"
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/albums?search=Dark")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = parse_json_response(response).await;
    assert_eq!(body["albums"].as_array().unwrap().len(), 1);
    assert_eq!(body["albums"][0]["title"], "Dark Side of the Moon");

    // Search for "Wall"
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/albums?search=Wall")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = parse_json_response(response).await;
    assert_eq!(body["albums"].as_array().unwrap().len(), 1);
    assert_eq!(body["albums"][0]["title"], "The Wall");
}

#[tokio::test]
async fn test_list_albums_combined_filters() {
    let state = setup_test_app_state().await;

    let artist = create_test_artist(&state.db, "Test Artist", None).await;

    let album1 = create_test_album(&state.db, artist.id, "Owned Album", None).await;
    let album2 = create_test_album(&state.db, artist.id, "Not Owned Album", None).await;

    // Set album1 to owned and matched
    let mut album1_active: albums::ActiveModel = album1.into();
    album1_active.ownership_status = Set(OwnershipStatus::Owned.as_str().to_string());
    album1_active.match_status = Set(Some(MatchStatus::Matched.as_str().to_string()));
    album1_active.update(&state.db).await.unwrap();

    let app = create_test_router(&state);

    // Filter by owned AND matched
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/albums?ownership_status=owned&match_status=matched")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = parse_json_response(response).await;
    assert_eq!(body["albums"].as_array().unwrap().len(), 1);
    assert_eq!(body["albums"][0]["title"], "Owned Album");
}

#[tokio::test]
async fn test_get_album_success() {
    let state = setup_test_app_state().await;

    let artist = create_test_artist(&state.db, "Pink Floyd", Some("spotify:artist:123")).await;
    let album = create_test_album(
        &state.db,
        artist.id,
        "Dark Side of the Moon",
        Some("spotify:album:456"),
    )
    .await;

    let app = create_test_router(&state);

    let response = app
        .oneshot(
            Request::builder()
                .uri(&format!("/api/albums/{}", album.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = parse_json_response(response).await;

    assert_eq!(body["id"], album.id);
    assert_eq!(body["title"], "Dark Side of the Moon");
    assert_eq!(body["artist"]["name"], "Pink Floyd");
    assert_eq!(body["artist"]["id"], artist.id);
}

#[tokio::test]
async fn test_get_album_not_found() {
    let state = setup_test_app_state().await;
    let app = create_test_router(&state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/albums/99999")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_update_album_ownership_status() {
    let state = setup_test_app_state().await;

    let artist = create_test_artist(&state.db, "Test Artist", None).await;
    let album = create_test_album(&state.db, artist.id, "Test Album", None).await;

    let app = create_test_router(&state);

    let response = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(&format!("/api/albums/{}", album.id))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "ownership_status": "owned"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = parse_json_response(response).await;
    // ownership_status is formatted with Debug, which wraps the string in quotes
    assert_eq!(body["ownership_status"].as_str().unwrap(), "\"owned\"");
}

#[tokio::test]
async fn test_update_album_acquisition_source() {
    let state = setup_test_app_state().await;

    let artist = create_test_artist(&state.db, "Test Artist", None).await;
    let album = create_test_album(&state.db, artist.id, "Test Album", None).await;

    let app = create_test_router(&state);

    let response = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(&format!("/api/albums/{}", album.id))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "ownership_status": "owned",
                        "acquisition_source": "bandcamp"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Verify in database
    let updated_album = albums::Entity::find_by_id(album.id)
        .one(&state.db)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(updated_album.ownership_status, OwnershipStatus::Owned.as_str());
    assert_eq!(
        updated_album.acquisition_source,
        Some(AcquisitionSource::Bandcamp.as_str().to_string())
    );
}

#[tokio::test]
async fn test_update_album_local_path() {
    let state = setup_test_app_state().await;

    let artist = create_test_artist(&state.db, "Test Artist", None).await;
    let album = create_test_album(&state.db, artist.id, "Test Album", None).await;

    let app = create_test_router(&state);

    let response = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(&format!("/api/albums/{}", album.id))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "local_path": "/music/artist/album"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Verify in database
    let updated_album = albums::Entity::find_by_id(album.id)
        .one(&state.db)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(updated_album.local_path, Some("/music/artist/album".to_string()));
}

#[tokio::test]
async fn test_update_album_not_found() {
    let state = setup_test_app_state().await;
    let app = create_test_router(&state);

    let response = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/api/albums/99999")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "ownership_status": "owned"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_update_album_invalid_ownership_status() {
    let state = setup_test_app_state().await;

    let artist = create_test_artist(&state.db, "Test Artist", None).await;
    let album = create_test_album(&state.db, artist.id, "Test Album", None).await;

    let app = create_test_router(&state);

    let response = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(&format!("/api/albums/{}", album.id))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "ownership_status": "invalid_status"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn test_get_stats_empty() {
    let state = setup_test_app_state().await;
    let app = create_test_router(&state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/stats")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = parse_json_response(response).await;

    assert_eq!(body["total_albums"], 0);
    assert_eq!(body["owned_albums"], 0);
    assert_eq!(body["not_owned_albums"], 0);
    assert_eq!(body["downloading_albums"], 0);
    assert_eq!(body["matched_albums"], 0);
    assert_eq!(body["unmatched_albums"], 0);
    assert_eq!(body["total_artists"], 0);
}

#[tokio::test]
async fn test_get_stats_with_data() {
    let state = setup_test_app_state().await;

    let artist1 = create_test_artist(&state.db, "Artist 1", None).await;
    let artist2 = create_test_artist(&state.db, "Artist 2", None).await;

    // Create albums with different statuses
    let album1 = create_test_album(&state.db, artist1.id, "Owned Album", None).await;
    let album2 = create_test_album(&state.db, artist1.id, "Not Owned Album", None).await;
    let album3 = create_test_album(&state.db, artist2.id, "Downloading Album", None).await;
    let album4 = create_test_album(&state.db, artist2.id, "Matched Album", None).await;

    // Update ownership statuses
    let mut album1_active: albums::ActiveModel = album1.into();
    album1_active.ownership_status = Set(OwnershipStatus::Owned.as_str().to_string());
    album1_active.update(&state.db).await.unwrap();

    let mut album3_active: albums::ActiveModel = album3.into();
    album3_active.ownership_status = Set(OwnershipStatus::Downloading.as_str().to_string());
    album3_active.update(&state.db).await.unwrap();

    // Update match status
    let mut album4_active: albums::ActiveModel = album4.into();
    album4_active.match_status = Set(Some(MatchStatus::Matched.as_str().to_string()));
    album4_active.update(&state.db).await.unwrap();

    let app = create_test_router(&state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/stats")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = parse_json_response(response).await;

    assert_eq!(body["total_albums"], 4);
    assert_eq!(body["owned_albums"], 1);
    assert_eq!(body["not_owned_albums"], 2);
    assert_eq!(body["downloading_albums"], 1);
    assert_eq!(body["matched_albums"], 1);
    assert_eq!(body["unmatched_albums"], 3); // pending is counted as unmatched
    assert_eq!(body["total_artists"], 2);
}

#[tokio::test]
async fn test_search_lidarr_no_settings() {
    let state = setup_test_app_state().await;

    let artist = create_test_artist(&state.db, "Test Artist", None).await;
    let album = create_test_album(&state.db, artist.id, "Test Album", None).await;

    let app = create_test_router(&state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/api/albums/{}/search-lidarr", album.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Should fail because user settings don't exist
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn test_search_lidarr_no_musicbrainz_id() {
    let state = setup_test_app_state().await;

    // Create user settings
    let now = chrono::Utc::now().into();
    let settings = user_settings::ActiveModel {
        lidarr_url: Set(Some("http://localhost:8686".to_string())),
        lidarr_api_key: Set(Some("test-api-key".to_string())),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };
    settings.insert(&state.db).await.unwrap();

    let artist = create_test_artist(&state.db, "Test Artist", None).await;
    let album = create_test_album(&state.db, artist.id, "Test Album", None).await;

    let app = create_test_router(&state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/api/albums/{}/search-lidarr", album.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Should fail because album doesn't have MusicBrainz ID
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}
