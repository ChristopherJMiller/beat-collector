//! Integration tests for settings handler routes
//!
//! Tests all settings-related API endpoints including:
//! - Get settings
//! - Update settings (create + update)
//! - Test Lidarr connection

use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use serde_json::json;
use tower::util::ServiceExt;

use beat_collector::db::entities::user_settings;
use beat_collector::handlers;
use beat_collector::state::AppState;
use beat_collector::test_utils::*;

/// Helper to create a test router with settings routes
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
async fn test_get_settings_not_found() {
    let state = setup_test_app_state().await;
    let app = create_test_router(&state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/settings")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_create_settings() {
    let state = setup_test_app_state().await;
    let app = create_test_router(&state);

    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/settings")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "lidarr_url": "http://localhost:8686",
                        "lidarr_api_key": "test-api-key",
                        "music_folder_path": "/music",
                        "auto_sync_enabled": true,
                        "sync_interval_hours": 24
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = parse_json_response(response).await;
    assert!(body["id"].as_i64().unwrap() > 0);
    assert_eq!(body["lidarr_url"], "http://localhost:8686");
    assert_eq!(body["music_folder_path"], "/music");
    assert_eq!(body["auto_sync_enabled"], true);
    assert_eq!(body["sync_interval_hours"], 24);
    assert_eq!(body["spotify_connected"], false);
}

#[tokio::test]
async fn test_get_settings_success() {
    let state = setup_test_app_state().await;

    // Create settings first
    let now = chrono::Utc::now().into();
    let settings = user_settings::ActiveModel {
        lidarr_url: Set(Some("http://localhost:8686".to_string())),
        lidarr_api_key: Set(Some("test-key".to_string())),
        music_folder_path: Set(Some("/music".to_string())),
        auto_sync_enabled: Set(Some(true)),
        sync_interval_hours: Set(Some(12)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };
    settings.insert(&state.db).await.unwrap();

    let app = create_test_router(&state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/settings")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = parse_json_response(response).await;
    assert_eq!(body["lidarr_url"], "http://localhost:8686");
    assert_eq!(body["music_folder_path"], "/music");
    assert_eq!(body["auto_sync_enabled"], true);
    assert_eq!(body["sync_interval_hours"], 12);
    assert_eq!(body["spotify_connected"], false);
}

#[tokio::test]
async fn test_get_settings_with_spotify_token() {
    let state = setup_test_app_state().await;

    // Create settings with Spotify token
    let now = chrono::Utc::now().into();
    let settings = user_settings::ActiveModel {
        spotify_access_token: Set(Some("access_token".to_string())),
        spotify_refresh_token: Set(Some("refresh_token".to_string())),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };
    settings.insert(&state.db).await.unwrap();

    let app = create_test_router(&state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/settings")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = parse_json_response(response).await;
    assert_eq!(body["spotify_connected"], true);
}

#[tokio::test]
async fn test_update_existing_settings() {
    let state = setup_test_app_state().await;

    // Create initial settings
    let now = chrono::Utc::now().into();
    let settings = user_settings::ActiveModel {
        lidarr_url: Set(Some("http://old-url:8686".to_string())),
        lidarr_api_key: Set(Some("old-key".to_string())),
        music_folder_path: Set(Some("/old/music".to_string())),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };
    let created = settings.insert(&state.db).await.unwrap();

    let app = create_test_router(&state);

    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/settings")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "lidarr_url": "http://new-url:8686",
                        "music_folder_path": "/new/music"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = parse_json_response(response).await;
    assert_eq!(body["id"], created.id);
    assert_eq!(body["lidarr_url"], "http://new-url:8686");
    assert_eq!(body["music_folder_path"], "/new/music");

    // Verify in database
    let updated = user_settings::Entity::find_by_id(created.id)
        .one(&state.db)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(updated.lidarr_url, Some("http://new-url:8686".to_string()));
    assert_eq!(updated.music_folder_path, Some("/new/music".to_string()));
    // Old API key should still be there (not updated)
    assert_eq!(updated.lidarr_api_key, Some("old-key".to_string()));
}

#[tokio::test]
async fn test_update_settings_partial() {
    let state = setup_test_app_state().await;

    // Create initial settings
    let now = chrono::Utc::now().into();
    let settings = user_settings::ActiveModel {
        lidarr_url: Set(Some("http://localhost:8686".to_string())),
        lidarr_api_key: Set(Some("api-key".to_string())),
        music_folder_path: Set(Some("/music".to_string())),
        auto_sync_enabled: Set(Some(false)),
        sync_interval_hours: Set(Some(12)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };
    settings.insert(&state.db).await.unwrap();

    let app = create_test_router(&state);

    // Update only auto_sync_enabled
    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/settings")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "auto_sync_enabled": true
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = parse_json_response(response).await;
    // Updated field
    assert_eq!(body["auto_sync_enabled"], true);
    // Unchanged fields
    assert_eq!(body["lidarr_url"], "http://localhost:8686");
    assert_eq!(body["music_folder_path"], "/music");
    assert_eq!(body["sync_interval_hours"], 12);
}

#[tokio::test]
async fn test_update_settings_all_fields() {
    let state = setup_test_app_state().await;

    let app = create_test_router(&state);

    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/settings")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "lidarr_url": "http://lidarr:8686",
                        "lidarr_api_key": "my-api-key",
                        "music_folder_path": "/mnt/music",
                        "auto_sync_enabled": true,
                        "sync_interval_hours": 6
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = parse_json_response(response).await;
    assert_eq!(body["lidarr_url"], "http://lidarr:8686");
    assert_eq!(body["music_folder_path"], "/mnt/music");
    assert_eq!(body["auto_sync_enabled"], true);
    assert_eq!(body["sync_interval_hours"], 6);
}

#[tokio::test]
async fn test_test_lidarr_connection_no_settings() {
    let state = setup_test_app_state().await;
    let app = create_test_router(&state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/settings/test-lidarr")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Should return an error status (Configuration error)
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn test_test_lidarr_connection_no_url() {
    let state = setup_test_app_state().await;

    // Create settings without Lidarr URL
    let now = chrono::Utc::now().into();
    let settings = user_settings::ActiveModel {
        music_folder_path: Set(Some("/music".to_string())),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };
    settings.insert(&state.db).await.unwrap();

    let app = create_test_router(&state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/settings/test-lidarr")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn test_test_lidarr_connection_no_api_key() {
    let state = setup_test_app_state().await;

    // Create settings without API key
    let now = chrono::Utc::now().into();
    let settings = user_settings::ActiveModel {
        lidarr_url: Set(Some("http://localhost:8686".to_string())),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };
    settings.insert(&state.db).await.unwrap();

    let app = create_test_router(&state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/settings/test-lidarr")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn test_settings_response_structure() {
    let state = setup_test_app_state().await;

    // Create settings
    let now = chrono::Utc::now().into();
    let settings = user_settings::ActiveModel {
        lidarr_url: Set(Some("http://localhost:8686".to_string())),
        music_folder_path: Set(Some("/music".to_string())),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };
    settings.insert(&state.db).await.unwrap();

    let app = create_test_router(&state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/settings")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = parse_json_response(response).await;

    // Verify all expected fields are present
    assert!(body.get("id").is_some());
    assert!(body.get("lidarr_url").is_some());
    assert!(body.get("music_folder_path").is_some());
    assert!(body.get("auto_sync_enabled").is_some());
    assert!(body.get("sync_interval_hours").is_some());
    assert!(body.get("spotify_connected").is_some());

    // API key should NOT be in response
    assert!(body.get("lidarr_api_key").is_none());
    assert!(body.get("spotify_access_token").is_none());
    assert!(body.get("spotify_refresh_token").is_none());
}

#[tokio::test]
async fn test_update_preserves_timestamps() {
    let state = setup_test_app_state().await;

    // Create initial settings
    let created_at = chrono::Utc::now().into();
    let settings = user_settings::ActiveModel {
        lidarr_url: Set(Some("http://localhost:8686".to_string())),
        created_at: Set(created_at),
        updated_at: Set(created_at),
        ..Default::default()
    };
    let created = settings.insert(&state.db).await.unwrap();

    // Wait a bit to ensure updated_at will be different
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    let app = create_test_router(&state);

    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/settings")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "music_folder_path": "/new/path"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Verify timestamps in database
    let updated = user_settings::Entity::find_by_id(created.id)
        .one(&state.db)
        .await
        .unwrap()
        .unwrap();

    // created_at should not change
    assert_eq!(updated.created_at.timestamp(), created.created_at.timestamp());
    // updated_at should be newer
    assert!(updated.updated_at.timestamp() >= created.updated_at.timestamp());
}
