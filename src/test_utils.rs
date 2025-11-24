//! Test utilities for Beat Collector
//!
//! Provides helpers for creating isolated test environments with:
//! - In-memory SQLite databases (one per test)
//! - Isolated Redis connections (separate DB numbers)
//! - AppState factories
//! - Test data generators

use std::sync::atomic::{AtomicU8, Ordering};

use chrono::Utc;
use migration::MigratorTrait;
use redis::aio::ConnectionManager;
use sea_orm::{ActiveModelTrait, Database, DatabaseConnection, Set};

use crate::{
    config::Config,
    db::{
        entities::{albums, artists, jobs},
        enums::{JobStatus, JobType, MatchStatus, OwnershipStatus},
    },
    jobs::JobQueue,
    state::AppState,
};

/// Global counter for test isolation
/// Used to ensure each test gets unique resources (like Redis DB numbers)
static TEST_COUNTER: AtomicU8 = AtomicU8::new(0);

/// Get a unique test ID for this test
pub fn get_test_id() -> u8 {
    TEST_COUNTER.fetch_add(1, Ordering::SeqCst)
}

/// Setup an in-memory SQLite database with all migrations applied
///
/// Each call creates a fresh, isolated database perfect for parallel testing
pub async fn setup_test_db() -> DatabaseConnection {
    let db = Database::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");

    // Run all migrations
    migration::Migrator::up(&db, None)
        .await
        .expect("Failed to run migrations");

    db
}

/// Setup a test Redis connection using a unique database number
///
/// Redis supports 16 databases (0-15), so we use test_id % 16 to isolate tests
/// Note: For more than 16 parallel tests, consider using key prefixes instead
pub async fn setup_test_redis() -> ConnectionManager {
    let test_id = get_test_id();
    let db_number = test_id % 16;

    let redis_url = std::env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());

    // Append database number to URL
    let test_redis_url = format!("{}/{}", redis_url.trim_end_matches('/'), db_number);

    let client = redis::Client::open(test_redis_url.as_str())
        .expect("Failed to create Redis client");

    let conn = client
        .get_connection_manager()
        .await
        .expect("Failed to connect to Redis");

    // Flush the test database to ensure clean state
    let mut conn_clone = conn.clone();
    redis::cmd("FLUSHDB")
        .query_async::<_, ()>(&mut conn_clone)
        .await
        .expect("Failed to flush Redis DB");

    conn
}

/// Create a test configuration with sensible defaults
pub fn test_config() -> Config {
    Config {
        database_url: "sqlite::memory:".to_string(),
        redis_url: "redis://127.0.0.1:6379".to_string(),
        server_host: "127.0.0.1".to_string(),
        server_port: 3000,
        spotify_client_id: "test_client_id".to_string(),
        spotify_redirect_uri: "http://localhost:3000/api/auth/spotify/callback".to_string(),
        music_folder_path: None,
        lidarr_url: None,
        lidarr_api_key: None,
    }
}

/// Create a complete test AppState with isolated database and Redis
pub async fn setup_test_app_state() -> AppState {
    let db = setup_test_db().await;
    let redis = setup_test_redis().await;
    let config = test_config();
    let (job_queue, _receiver) = JobQueue::new();

    AppState::new(db, redis, config, job_queue)
}

/// Create a test AppState with job queue that keeps the receiver alive
/// Returns (AppState, receiver) tuple - keep receiver in scope to prevent queue from closing
pub async fn setup_test_app_state_with_queue() -> (
    AppState,
    tokio::sync::mpsc::UnboundedReceiver<crate::jobs::queue::JobMessage>,
) {
    let db = setup_test_db().await;
    let redis = setup_test_redis().await;
    let config = test_config();
    let (job_queue, receiver) = JobQueue::new();

    (AppState::new(db, redis, config, job_queue), receiver)
}

// ============================================================================
// Test Data Factories
// ============================================================================

/// Create a test artist in the database
pub async fn create_test_artist(
    db: &DatabaseConnection,
    name: &str,
    spotify_id: Option<&str>,
) -> artists::Model {
    let now = Utc::now().into();
    let artist = artists::ActiveModel {
        name: Set(name.to_string()),
        spotify_id: Set(spotify_id.map(|s| s.to_string())),
        musicbrainz_id: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };

    artist.insert(db).await.expect("Failed to insert test artist")
}

/// Create a test album in the database
pub async fn create_test_album(
    db: &DatabaseConnection,
    artist_id: i32,
    title: &str,
    spotify_id: Option<&str>,
) -> albums::Model {
    let now = Utc::now().into();
    let album = albums::ActiveModel {
        artist_id: Set(artist_id),
        title: Set(title.to_string()),
        spotify_id: Set(spotify_id.map(|s| s.to_string())),
        musicbrainz_release_group_id: Set(None),
        release_date: Set(None),
        cover_art_url: Set(None),
        ownership_status: Set(OwnershipStatus::NotOwned.as_str().to_string()),
        match_status: Set(Some(MatchStatus::Pending.as_str().to_string())),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };

    album.insert(db).await.expect("Failed to insert test album")
}

/// Create a test job in the database
pub async fn create_test_job(
    db: &DatabaseConnection,
    job_type: JobType,
    status: JobStatus,
) -> jobs::Model {
    let now = Utc::now().into();
    let job = jobs::ActiveModel {
        job_type: Set(job_type.as_str().to_string()),
        status: Set(status.as_str().to_string()),
        progress: Set(None),
        processed_items: Set(None),
        total_items: Set(None),
        error_message: Set(None),
        started_at: Set(None),
        completed_at: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };

    job.insert(db).await.expect("Failed to insert test job")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_setup_test_db() {
        let db = setup_test_db().await;
        // Verify we can query the database (it has tables from migrations)
        use sea_orm::EntityTrait;
        let artists = artists::Entity::find().all(&db).await.unwrap();
        assert_eq!(artists.len(), 0);
    }

    #[tokio::test]
    async fn test_create_test_artist() {
        let db = setup_test_db().await;
        let artist = create_test_artist(&db, "Test Artist", Some("spotify:123")).await;

        assert_eq!(artist.name, "Test Artist");
        assert_eq!(artist.spotify_id, Some("spotify:123".to_string()));
    }

    #[tokio::test]
    async fn test_create_test_album() {
        let db = setup_test_db().await;
        let artist = create_test_artist(&db, "Test Artist", None).await;
        let album = create_test_album(&db, artist.id, "Test Album", None).await;

        assert_eq!(album.title, "Test Album");
        assert_eq!(album.artist_id, artist.id);
    }

    #[tokio::test]
    async fn test_create_test_job() {
        let db = setup_test_db().await;
        let job = create_test_job(&db, JobType::SpotifySync, JobStatus::Pending).await;

        assert_eq!(job.job_type, JobType::SpotifySync.as_str());
        assert_eq!(job.status, JobStatus::Pending.as_str());
    }

    #[tokio::test]
    async fn test_parallel_databases() {
        // Run two database setups in parallel - they should not interfere
        let (db1, db2) = tokio::join!(setup_test_db(), setup_test_db());

        let artist1 = create_test_artist(&db1, "Artist 1", None).await;
        let artist2 = create_test_artist(&db2, "Artist 2", None).await;

        // Both should be ID 1 (separate databases)
        assert_eq!(artist1.id, 1);
        assert_eq!(artist2.id, 1);

        // Verify isolation
        use sea_orm::EntityTrait;
        let db1_artists = artists::Entity::find().all(&db1).await.unwrap();
        let db2_artists = artists::Entity::find().all(&db2).await.unwrap();

        assert_eq!(db1_artists.len(), 1);
        assert_eq!(db2_artists.len(), 1);
        assert_eq!(db1_artists[0].name, "Artist 1");
        assert_eq!(db2_artists[0].name, "Artist 2");
    }
}
