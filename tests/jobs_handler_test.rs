//! Integration tests for job handler routes
//!
//! Tests all job-related API endpoints including:
//! - List jobs
//! - Get job status
//! - Trigger Spotify sync
//! - Trigger MusicBrainz match

use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use sea_orm::EntityTrait;
use serde_json::json;
use tower::util::ServiceExt;

use beat_collector::db::{
    entities::jobs,
    enums::{JobStatus, JobType},
};
use beat_collector::handlers;
use beat_collector::state::AppState;
use beat_collector::test_utils::*;

/// Helper to create a test router with job routes
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
async fn test_list_jobs_empty() {
    let state = setup_test_app_state().await;
    let app = create_test_router(&state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/jobs")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = parse_json_response(response).await;
    assert_eq!(body.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_list_jobs_with_data() {
    let state = setup_test_app_state().await;

    // Create multiple jobs
    create_test_job(&state.db, JobType::SpotifySync, JobStatus::Pending).await;
    create_test_job(&state.db, JobType::MusicbrainzMatch, JobStatus::Running).await;
    create_test_job(&state.db, JobType::CoverArtFetch, JobStatus::Completed).await;

    let app = create_test_router(&state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/jobs")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = parse_json_response(response).await;
    let jobs = body.as_array().unwrap();
    assert_eq!(jobs.len(), 3);

    // Jobs should be ordered by created_at DESC (most recent first)
    assert_eq!(jobs[0]["status"], "\"completed\"");
    assert_eq!(jobs[1]["status"], "\"running\"");
    assert_eq!(jobs[2]["status"], "\"pending\"");
}

#[tokio::test]
async fn test_list_jobs_ordering() {
    let state = setup_test_app_state().await;

    // Create jobs in specific order
    let job1 = create_test_job(&state.db, JobType::SpotifySync, JobStatus::Completed).await;
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    let job2 = create_test_job(&state.db, JobType::MusicbrainzMatch, JobStatus::Running).await;
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    let job3 = create_test_job(&state.db, JobType::CoverArtFetch, JobStatus::Pending).await;

    let app = create_test_router(&state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/jobs")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = parse_json_response(response).await;
    let jobs = body.as_array().unwrap();

    // Most recent job should be first
    assert_eq!(jobs[0]["id"], job3.id);
    assert_eq!(jobs[1]["id"], job2.id);
    assert_eq!(jobs[2]["id"], job1.id);
}

#[tokio::test]
async fn test_list_jobs_limit_50() {
    let state = setup_test_app_state().await;

    // Create 60 jobs to test the limit
    for _ in 0..60 {
        create_test_job(&state.db, JobType::SpotifySync, JobStatus::Pending).await;
    }

    let app = create_test_router(&state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/jobs")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = parse_json_response(response).await;
    let jobs = body.as_array().unwrap();

    // Should only return 50 most recent jobs
    assert_eq!(jobs.len(), 50);
}

#[tokio::test]
async fn test_get_job_status_success() {
    let state = setup_test_app_state().await;

    let job = create_test_job(&state.db, JobType::SpotifySync, JobStatus::Running).await;

    let app = create_test_router(&state);

    let response = app
        .oneshot(
            Request::builder()
                .uri(&format!("/api/jobs/{}/status", job.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = parse_json_response(response).await;
    assert_eq!(body["id"], job.id);
    assert_eq!(body["job_type"], "\"spotify_sync\"");
    assert_eq!(body["status"], "\"running\"");
}

#[tokio::test]
async fn test_get_job_status_not_found() {
    let state = setup_test_app_state().await;
    let app = create_test_router(&state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/jobs/99999/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_trigger_spotify_sync() {
    let (state, _receiver) = setup_test_app_state_with_queue().await;
    let app = create_test_router(&state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/jobs/spotify-sync")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = parse_json_response(response).await;
    let job_id = body["job_id"].as_i64().unwrap() as i32;
    assert!(job_id > 0);
    assert_eq!(body["status"], "pending");

    // Verify job was created in database
    let job = jobs::Entity::find_by_id(job_id)
        .one(&state.db)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(job.job_type, JobType::SpotifySync.as_str());
    assert_eq!(job.status, JobStatus::Pending.as_str());
    assert!(job.created_at.timestamp() > 0);
    assert!(job.updated_at.timestamp() > 0);
}

#[tokio::test]
async fn test_trigger_musicbrainz_match() {
    let (state, _receiver) = setup_test_app_state_with_queue().await;
    let app = create_test_router(&state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/jobs/musicbrainz-match-all")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = parse_json_response(response).await;
    let job_id = body["job_id"].as_i64().unwrap() as i32;
    assert!(job_id > 0);
    assert_eq!(body["status"], "pending");

    // Verify job was created with correct type
    let job = jobs::Entity::find_by_id(job_id)
        .one(&state.db)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(job.job_type, JobType::MusicbrainzMatch.as_str());
    assert_eq!(job.status, JobStatus::Pending.as_str());
}

#[tokio::test]
async fn test_job_response_fields() {
    let state = setup_test_app_state().await;

    let job = create_test_job(&state.db, JobType::SpotifySync, JobStatus::Running).await;

    let app = create_test_router(&state);

    let response = app
        .oneshot(
            Request::builder()
                .uri(&format!("/api/jobs/{}/status", job.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = parse_json_response(response).await;

    // Verify all expected fields are present
    assert!(body.get("id").is_some());
    assert!(body.get("job_type").is_some());
    assert!(body.get("status").is_some());
    assert!(body.get("progress").is_some());
    assert!(body.get("processed_items").is_some());
    assert!(body.get("total_items").is_some());
    assert!(body.get("error_message").is_some());
    assert!(body.get("started_at").is_some());
    assert!(body.get("completed_at").is_some());
    assert!(body.get("created_at").is_some());

    // Verify nullable fields are null when not set
    assert!(body["progress"].is_null());
    assert!(body["processed_items"].is_null());
    assert!(body["total_items"].is_null());
    assert!(body["error_message"].is_null());
    assert!(body["started_at"].is_null());
    assert!(body["completed_at"].is_null());
}

#[tokio::test]
async fn test_multiple_job_types() {
    let state = setup_test_app_state().await;

    // Create jobs of different types
    create_test_job(&state.db, JobType::SpotifySync, JobStatus::Completed).await;
    create_test_job(&state.db, JobType::MusicbrainzMatch, JobStatus::Running).await;
    create_test_job(&state.db, JobType::CoverArtFetch, JobStatus::Pending).await;
    create_test_job(&state.db, JobType::FilesystemScan, JobStatus::Completed).await;

    let app = create_test_router(&state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/jobs")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = parse_json_response(response).await;
    let jobs = body.as_array().unwrap();

    assert_eq!(jobs.len(), 4);

    // Verify different job types are correctly serialized
    let job_types: Vec<String> = jobs
        .iter()
        .map(|j| j["job_type"].as_str().unwrap().to_string())
        .collect();

    assert!(job_types.contains(&"\"filesystem_scan\"".to_string()));
    assert!(job_types.contains(&"\"cover_art_fetch\"".to_string()));
    assert!(job_types.contains(&"\"musicbrainz_match\"".to_string()));
    assert!(job_types.contains(&"\"spotify_sync\"".to_string()));
}
