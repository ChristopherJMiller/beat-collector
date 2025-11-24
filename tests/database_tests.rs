//! Database integration tests
//!
//! Tests basic CRUD operations for all entities to ensure:
//! - Entities can be created with all required fields
//! - Foreign key constraints work correctly
//! - Timestamps are set properly
//! - Queries return expected results

use beat_collector::test_utils::*;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};

// Import entities
use beat_collector::db::entities::{albums, artists, jobs, lidarr_downloads, tracks};
use beat_collector::db::enums::{
    AcquisitionSource, DownloadStatus, JobStatus, JobType, MatchStatus, OwnershipStatus,
};

#[tokio::test]
async fn test_create_artist() {
    let db = setup_test_db().await;

    let artist = create_test_artist(&db, "The Beatles", Some("spotify:artist:3WrFJ7ztbogyGnTHbHJFl2")).await;

    assert_eq!(artist.name, "The Beatles");
    assert_eq!(artist.spotify_id, Some("spotify:artist:3WrFJ7ztbogyGnTHbHJFl2".to_string()));
    assert!(artist.id > 0);
    assert!(artist.created_at.timestamp() > 0);
    assert!(artist.updated_at.timestamp() > 0);
}

#[tokio::test]
async fn test_create_album() {
    let db = setup_test_db().await;

    let artist = create_test_artist(&db, "Pink Floyd", None).await;
    let album = create_test_album(&db, artist.id, "The Dark Side of the Moon", Some("spotify:album:4LH4d3cOWNNsVw41Gqt2kv")).await;

    assert_eq!(album.title, "The Dark Side of the Moon");
    assert_eq!(album.artist_id, artist.id);
    assert_eq!(album.spotify_id, Some("spotify:album:4LH4d3cOWNNsVw41Gqt2kv".to_string()));
    assert_eq!(album.ownership_status, OwnershipStatus::NotOwned.as_str());
    assert_eq!(album.match_status, Some(MatchStatus::Pending.as_str().to_string()));
    assert!(album.id > 0);
    assert!(album.created_at.timestamp() > 0);
    assert!(album.updated_at.timestamp() > 0);
}

#[tokio::test]
async fn test_album_requires_valid_artist() {
    let db = setup_test_db().await;

    // Try to create an album with a non-existent artist ID
    let now = Utc::now().into();
    let invalid_album = albums::ActiveModel {
        artist_id: Set(99999), // Non-existent artist
        title: Set("Test Album".to_string()),
        spotify_id: Set(None),
        musicbrainz_release_group_id: Set(None),
        release_date: Set(None),
        cover_art_url: Set(None),
        ownership_status: Set(OwnershipStatus::NotOwned.as_str().to_string()),
        match_status: Set(Some(MatchStatus::Pending.as_str().to_string())),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };

    // This should fail due to foreign key constraint
    let result = invalid_album.insert(&db).await;
    assert!(result.is_err(), "Should fail to create album with invalid artist_id");
}

#[tokio::test]
async fn test_create_job() {
    let db = setup_test_db().await;

    let job = create_test_job(&db, JobType::SpotifySync, JobStatus::Pending).await;

    assert_eq!(job.job_type, JobType::SpotifySync.as_str());
    assert_eq!(job.status, JobStatus::Pending.as_str());
    assert_eq!(job.progress, None);
    assert_eq!(job.started_at, None);
    assert_eq!(job.completed_at, None);
    assert!(job.id > 0);
    assert!(job.created_at.timestamp() > 0);
    assert!(job.updated_at.timestamp() > 0);
}

#[tokio::test]
async fn test_job_has_updated_at_set() {
    let db = setup_test_db().await;

    // This is the bug we fixed - ensure updated_at is always set
    let now = Utc::now().into();
    let job = jobs::ActiveModel {
        job_type: Set(JobType::MusicbrainzMatch.as_str().to_string()),
        status: Set(JobStatus::Running.as_str().to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };

    let inserted = job.insert(&db).await.expect("Should successfully insert job");

    assert!(inserted.updated_at.timestamp() > 0, "updated_at must be set");
    assert_eq!(inserted.created_at.timestamp(), inserted.updated_at.timestamp());
}

#[tokio::test]
async fn test_query_artist_by_spotify_id() {
    let db = setup_test_db().await;

    create_test_artist(&db, "Artist 1", Some("spotify:1")).await;
    create_test_artist(&db, "Artist 2", Some("spotify:2")).await;

    use sea_orm::ColumnTrait;
    use sea_orm::QueryFilter;

    let found = artists::Entity::find()
        .filter(artists::Column::SpotifyId.eq("spotify:2"))
        .one(&db)
        .await
        .expect("Query should succeed");

    assert!(found.is_some());
    let artist = found.unwrap();
    assert_eq!(artist.name, "Artist 2");
}

#[tokio::test]
async fn test_query_albums_by_artist() {
    let db = setup_test_db().await;

    let artist = create_test_artist(&db, "Test Artist", None).await;

    create_test_album(&db, artist.id, "Album 1", None).await;
    create_test_album(&db, artist.id, "Album 2", None).await;
    create_test_album(&db, artist.id, "Album 3", None).await;

    use sea_orm::ColumnTrait;
    use sea_orm::QueryFilter;

    let albums = albums::Entity::find()
        .filter(albums::Column::ArtistId.eq(artist.id))
        .all(&db)
        .await
        .expect("Query should succeed");

    assert_eq!(albums.len(), 3);
}

#[tokio::test]
async fn test_update_album_ownership() {
    let db = setup_test_db().await;

    let artist = create_test_artist(&db, "Test Artist", None).await;
    let album = create_test_album(&db, artist.id, "Test Album", None).await;

    // Initially should be NotOwned
    assert_eq!(album.ownership_status, OwnershipStatus::NotOwned.as_str());

    // Update to Owned
    let mut album_active: albums::ActiveModel = album.into();
    album_active.ownership_status = Set(OwnershipStatus::Owned.as_str().to_string());
    album_active.updated_at = Set(Utc::now().into());

    let updated = album_active.update(&db).await.expect("Update should succeed");

    assert_eq!(updated.ownership_status, OwnershipStatus::Owned.as_str());
}

#[tokio::test]
async fn test_create_lidarr_download() {
    let db = setup_test_db().await;

    let artist = create_test_artist(&db, "Download Artist", None).await;
    let album = create_test_album(&db, artist.id, "Download Album", None).await;

    let now = Utc::now().into();
    let download = lidarr_downloads::ActiveModel {
        album_id: Set(album.id),
        lidarr_album_id: Set(Some(12345)),
        status: Set(DownloadStatus::Pending.as_str().to_string()),
        quality_profile: Set(Some("Lossless".to_string())),
        estimated_completion_at: Set(None),
        completed_at: Set(None),
        error_message: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };

    let inserted = download.insert(&db).await.expect("Should insert download");

    assert_eq!(inserted.album_id, album.id);
    assert_eq!(inserted.status, DownloadStatus::Pending.as_str());
    assert_eq!(inserted.quality_profile, Some("Lossless".to_string()));
    assert!(inserted.id > 0);
}

#[tokio::test]
async fn test_parallel_database_isolation() {
    // Create two separate databases and verify they don't interfere
    let (db1, db2) = tokio::join!(setup_test_db(), setup_test_db());

    let (artist1, artist2) = tokio::join!(
        create_test_artist(&db1, "Artist DB1", None),
        create_test_artist(&db2, "Artist DB2", None)
    );

    // Both should have ID 1 (different databases)
    assert_eq!(artist1.id, 1);
    assert_eq!(artist2.id, 1);

    // Verify they're actually in separate databases
    let db1_artists = artists::Entity::find().all(&db1).await.unwrap();
    let db2_artists = artists::Entity::find().all(&db2).await.unwrap();

    assert_eq!(db1_artists.len(), 1);
    assert_eq!(db2_artists.len(), 1);
    assert_eq!(db1_artists[0].name, "Artist DB1");
    assert_eq!(db2_artists[0].name, "Artist DB2");
}

// ============================================================================
// Tracks Entity Tests
// ============================================================================

#[tokio::test]
async fn test_create_track_with_all_fields() {
    let db = setup_test_db().await;

    let artist = create_test_artist(&db, "Test Artist", None).await;
    let album = create_test_album(&db, artist.id, "Test Album", None).await;

    let now = Utc::now().into();
    let track = tracks::ActiveModel {
        album_id: Set(album.id),
        title: Set("Test Track".to_string()),
        track_number: Set(Some(1)),
        disc_number: Set(Some(1)),
        duration_ms: Set(Some(180000)), // 3 minutes
        spotify_id: Set(Some("spotify:track:123".to_string())),
        musicbrainz_id: Set(Some("mb-track-456".to_string())),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };

    let inserted = track.insert(&db).await.expect("Should insert track");

    assert_eq!(inserted.album_id, album.id);
    assert_eq!(inserted.title, "Test Track");
    assert_eq!(inserted.track_number, Some(1));
    assert_eq!(inserted.disc_number, Some(1));
    assert_eq!(inserted.duration_ms, Some(180000));
    assert_eq!(inserted.spotify_id, Some("spotify:track:123".to_string()));
    assert_eq!(inserted.musicbrainz_id, Some("mb-track-456".to_string()));
    assert!(inserted.id > 0);
    assert!(inserted.created_at.timestamp() > 0);
    assert!(inserted.updated_at.timestamp() > 0);
}

#[tokio::test]
async fn test_create_track_minimal_fields() {
    let db = setup_test_db().await;

    let artist = create_test_artist(&db, "Test Artist", None).await;
    let album = create_test_album(&db, artist.id, "Test Album", None).await;

    let now = Utc::now().into();
    let track = tracks::ActiveModel {
        album_id: Set(album.id),
        title: Set("Minimal Track".to_string()),
        track_number: Set(None),
        disc_number: Set(None),
        duration_ms: Set(None),
        spotify_id: Set(None),
        musicbrainz_id: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };

    let inserted = track.insert(&db).await.expect("Should insert track with minimal fields");

    assert_eq!(inserted.album_id, album.id);
    assert_eq!(inserted.title, "Minimal Track");
    assert_eq!(inserted.track_number, None);
    assert_eq!(inserted.disc_number, None);
    assert_eq!(inserted.duration_ms, None);
    assert_eq!(inserted.spotify_id, None);
    assert_eq!(inserted.musicbrainz_id, None);
}

#[tokio::test]
async fn test_track_requires_valid_album() {
    let db = setup_test_db().await;

    let now = Utc::now().into();
    let invalid_track = tracks::ActiveModel {
        album_id: Set(99999), // Non-existent album
        title: Set("Invalid Track".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };

    // Should fail due to foreign key constraint
    let result = invalid_track.insert(&db).await;
    assert!(result.is_err(), "Should fail to create track with invalid album_id");
}

#[tokio::test]
async fn test_query_tracks_by_album() {
    let db = setup_test_db().await;

    let artist = create_test_artist(&db, "Test Artist", None).await;
    let album = create_test_album(&db, artist.id, "Test Album", None).await;

    let now = Utc::now().into();

    // Create multiple tracks for the album
    for i in 1..=5 {
        let track = tracks::ActiveModel {
            album_id: Set(album.id),
            title: Set(format!("Track {}", i)),
            track_number: Set(Some(i)),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        };
        track.insert(&db).await.unwrap();
    }

    use sea_orm::ColumnTrait;
    use sea_orm::QueryFilter;

    let tracks_list = tracks::Entity::find()
        .filter(tracks::Column::AlbumId.eq(album.id))
        .all(&db)
        .await
        .expect("Query should succeed");

    assert_eq!(tracks_list.len(), 5);
}

#[tokio::test]
async fn test_tracks_cascade_delete_with_album() {
    let db = setup_test_db().await;

    let artist = create_test_artist(&db, "Test Artist", None).await;
    let album = create_test_album(&db, artist.id, "Test Album", None).await;

    let now = Utc::now().into();

    // Create tracks for the album
    let track1 = tracks::ActiveModel {
        album_id: Set(album.id),
        title: Set("Track 1".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };
    let track2 = tracks::ActiveModel {
        album_id: Set(album.id),
        title: Set("Track 2".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };

    track1.insert(&db).await.unwrap();
    track2.insert(&db).await.unwrap();

    // Verify tracks exist
    use sea_orm::ColumnTrait;
    use sea_orm::QueryFilter;
    let tracks_before = tracks::Entity::find()
        .filter(tracks::Column::AlbumId.eq(album.id))
        .all(&db)
        .await
        .unwrap();
    assert_eq!(tracks_before.len(), 2);

    // Store album ID before deleting
    let album_id = album.id;

    // Delete the album
    let album_active: albums::ActiveModel = album.into();
    album_active.delete(&db).await.expect("Should delete album");

    // Tracks should be cascade deleted
    let tracks_after = tracks::Entity::find()
        .filter(tracks::Column::AlbumId.eq(album_id))
        .all(&db)
        .await
        .unwrap();
    assert_eq!(tracks_after.len(), 0, "Tracks should be cascade deleted when album is deleted");
}

#[tokio::test]
async fn test_track_ordering() {
    let db = setup_test_db().await;

    let artist = create_test_artist(&db, "Test Artist", None).await;
    let album = create_test_album(&db, artist.id, "Test Album", None).await;

    let now = Utc::now().into();

    // Create tracks with different disc and track numbers
    let track_data = vec![
        (2, 1, "Disc 2 Track 1"),
        (1, 3, "Disc 1 Track 3"),
        (1, 1, "Disc 1 Track 1"),
        (1, 2, "Disc 1 Track 2"),
        (2, 2, "Disc 2 Track 2"),
    ];

    for (disc, track_num, title) in track_data {
        let track = tracks::ActiveModel {
            album_id: Set(album.id),
            title: Set(title.to_string()),
            track_number: Set(Some(track_num)),
            disc_number: Set(Some(disc)),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        };
        track.insert(&db).await.unwrap();
    }

    use sea_orm::{ColumnTrait, QueryFilter, QueryOrder};

    // Query tracks ordered by disc and track number
    let ordered_tracks = tracks::Entity::find()
        .filter(tracks::Column::AlbumId.eq(album.id))
        .order_by_asc(tracks::Column::DiscNumber)
        .order_by_asc(tracks::Column::TrackNumber)
        .all(&db)
        .await
        .unwrap();

    assert_eq!(ordered_tracks.len(), 5);
    assert_eq!(ordered_tracks[0].title, "Disc 1 Track 1");
    assert_eq!(ordered_tracks[1].title, "Disc 1 Track 2");
    assert_eq!(ordered_tracks[2].title, "Disc 1 Track 3");
    assert_eq!(ordered_tracks[3].title, "Disc 2 Track 1");
    assert_eq!(ordered_tracks[4].title, "Disc 2 Track 2");
}

#[tokio::test]
async fn test_update_track() {
    let db = setup_test_db().await;

    let artist = create_test_artist(&db, "Test Artist", None).await;
    let album = create_test_album(&db, artist.id, "Test Album", None).await;

    let now = Utc::now().into();
    let track = tracks::ActiveModel {
        album_id: Set(album.id),
        title: Set("Original Title".to_string()),
        track_number: Set(Some(1)),
        duration_ms: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };

    let inserted = track.insert(&db).await.unwrap();

    // Update the track
    let mut track_active: tracks::ActiveModel = inserted.into();
    track_active.title = Set("Updated Title".to_string());
    track_active.duration_ms = Set(Some(200000));
    track_active.updated_at = Set(Utc::now().into());

    let updated = track_active.update(&db).await.expect("Update should succeed");

    assert_eq!(updated.title, "Updated Title");
    assert_eq!(updated.duration_ms, Some(200000));
    assert_eq!(updated.track_number, Some(1)); // Unchanged
}

#[tokio::test]
async fn test_track_with_spotify_and_musicbrainz_ids() {
    let db = setup_test_db().await;

    let artist = create_test_artist(&db, "Test Artist", None).await;
    let album = create_test_album(&db, artist.id, "Test Album", None).await;

    let now = Utc::now().into();
    let track = tracks::ActiveModel {
        album_id: Set(album.id),
        title: Set("Identified Track".to_string()),
        spotify_id: Set(Some("spotify:track:abc123".to_string())),
        musicbrainz_id: Set(Some("mb-recording-xyz789".to_string())),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };

    let inserted = track.insert(&db).await.unwrap();

    // Query by Spotify ID
    use sea_orm::{ColumnTrait, QueryFilter};
    let found_by_spotify = tracks::Entity::find()
        .filter(tracks::Column::SpotifyId.eq("spotify:track:abc123"))
        .one(&db)
        .await
        .unwrap();

    assert!(found_by_spotify.is_some());
    assert_eq!(found_by_spotify.unwrap().id, inserted.id);

    // Query by MusicBrainz ID
    let found_by_mb = tracks::Entity::find()
        .filter(tracks::Column::MusicbrainzId.eq("mb-recording-xyz789"))
        .one(&db)
        .await
        .unwrap();

    assert!(found_by_mb.is_some());
    assert_eq!(found_by_mb.unwrap().id, inserted.id);
}

#[tokio::test]
async fn test_multi_disc_album_tracks() {
    let db = setup_test_db().await;

    let artist = create_test_artist(&db, "Test Artist", None).await;
    let album = create_test_album(&db, artist.id, "Multi-Disc Album", None).await;

    let now = Utc::now().into();

    // Create tracks across 3 discs
    for disc in 1..=3 {
        for track_num in 1..=4 {
            let track = tracks::ActiveModel {
                album_id: Set(album.id),
                title: Set(format!("D{} T{}", disc, track_num)),
                disc_number: Set(Some(disc)),
                track_number: Set(Some(track_num)),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            };
            track.insert(&db).await.unwrap();
        }
    }

    use sea_orm::{ColumnTrait, QueryFilter};

    // Query all tracks
    let all_tracks = tracks::Entity::find()
        .filter(tracks::Column::AlbumId.eq(album.id))
        .all(&db)
        .await
        .unwrap();

    assert_eq!(all_tracks.len(), 12); // 3 discs × 4 tracks

    // Query tracks from disc 2
    let disc2_tracks = tracks::Entity::find()
        .filter(tracks::Column::AlbumId.eq(album.id))
        .filter(tracks::Column::DiscNumber.eq(2))
        .all(&db)
        .await
        .unwrap();

    assert_eq!(disc2_tracks.len(), 4);
}
