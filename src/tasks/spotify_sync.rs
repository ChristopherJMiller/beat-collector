use anyhow::Result;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use sha2::{Digest, Sha256};

use crate::{
    db::{
        entities::{albums, artists, playlist_tracks, playlists, tracks, user_settings},
        enums::{AlbumSource, MatchStatus, OwnershipStatus},
    },
    services::{SpotifyAlbum, SpotifyArtist, SpotifyPlaylist, SpotifyPlaylistTrack, SpotifyService, SpotifyTrack},
    state::AppState,
};

/// Synthetic Spotify ID for Liked Songs playlist
pub const LIKED_SONGS_SPOTIFY_ID: &str = "__LIKED_SONGS__";
pub const LIKED_SONGS_NAME: &str = "Liked Songs";

/// Main entry point for Spotify sync job
pub async fn run_spotify_sync(state: AppState) -> Result<()> {
    tracing::info!("Starting Spotify sync job");

    // Get user settings with Spotify tokens
    let settings = user_settings::Entity::find()
        .one(&state.db)
        .await?
        .ok_or_else(|| anyhow::anyhow!("No user settings found"))?;

    let access_token = settings
        .spotify_access_token
        .ok_or_else(|| anyhow::anyhow!("Spotify not connected"))?;

    // Initialize Spotify service
    let spotify_service = SpotifyService::new(
        state.config.spotify_client_id.clone(),
        state.config.spotify_redirect_uri.clone(),
    );

    // Phase 1: Sync saved albums
    sync_saved_albums(&state.db, &spotify_service, &access_token).await?;

    // Phase 2: Sync playlists
    sync_playlists(&state.db, &spotify_service, &access_token).await?;

    tracing::info!("Spotify sync completed successfully");
    Ok(())
}

/// Sync saved albums from user's Spotify library
async fn sync_saved_albums(
    db: &DatabaseConnection,
    spotify_service: &SpotifyService,
    access_token: &str,
) -> Result<()> {
    let albums = spotify_service.fetch_saved_albums(access_token).await?;
    tracing::info!("Fetched {} saved albums from Spotify", albums.len());

    for spotify_album in albums {
        let artist = upsert_artist(db, &spotify_album.artists[0]).await?;
        upsert_album(db, &spotify_album, artist.id, AlbumSource::SavedAlbum).await?;
    }

    Ok(())
}

/// Sync playlists and their tracks from Spotify
async fn sync_playlists(
    db: &DatabaseConnection,
    spotify_service: &SpotifyService,
    access_token: &str,
) -> Result<()> {
    // Sync Liked Songs as a synthetic playlist first
    sync_liked_songs(db, spotify_service, access_token).await?;

    // Then sync regular playlists
    let spotify_playlists = spotify_service.fetch_user_playlists(access_token).await?;
    tracing::info!("Fetched {} playlists from Spotify", spotify_playlists.len());

    for spotify_playlist in spotify_playlists {
        // Upsert the playlist record
        let playlist = upsert_playlist(db, &spotify_playlist).await?;

        // Only sync tracks for enabled playlists
        if !playlist.is_enabled {
            tracing::debug!("Skipping disabled playlist: {}", playlist.name);
            continue;
        }

        // Check if playlist changed (via snapshot_id)
        let should_sync_tracks = playlist.snapshot_id.as_deref() != Some(&spotify_playlist.snapshot_id)
            || playlist.last_synced_at.is_none();

        if !should_sync_tracks {
            tracing::debug!("Playlist {} unchanged, skipping track sync", playlist.name);
            continue;
        }

        // Fetch and sync tracks for this playlist
        let spotify_tracks = spotify_service
            .fetch_playlist_tracks(access_token, &spotify_playlist.id)
            .await?;

        tracing::info!(
            "Syncing {} tracks for playlist: {}",
            spotify_tracks.len(),
            playlist.name
        );

        sync_playlist_tracks(db, playlist.id, &spotify_tracks).await?;

        // Update playlist snapshot_id and last_synced_at
        let mut active: playlists::ActiveModel = playlist.into();
        active.snapshot_id = Set(Some(spotify_playlist.snapshot_id.clone()));
        active.last_synced_at = Set(Some(Utc::now().into()));
        active.updated_at = Set(Utc::now().into());
        active.update(db).await?;
    }

    Ok(())
}

/// Sync tracks for a specific playlist
async fn sync_playlist_tracks(
    db: &DatabaseConnection,
    playlist_id: i32,
    spotify_tracks: &[SpotifyPlaylistTrack],
) -> Result<()> {
    // Collect track IDs that should be in this playlist
    let mut valid_track_ids: Vec<i32> = Vec::new();

    for (position, playlist_track) in spotify_tracks.iter().enumerate() {
        // Skip tracks without data (local files, deleted tracks)
        let spotify_track = match &playlist_track.track {
            Some(t) => t,
            None => continue,
        };

        // Skip tracks without Spotify ID (local files)
        let track_spotify_id = match &spotify_track.id {
            Some(id) => id,
            None => continue,
        };

        // Upsert artist (use first artist)
        let artist = upsert_artist(db, &spotify_track.artists[0]).await?;

        // Upsert album (mark as playlist import if new)
        let album = upsert_album(db, &spotify_track.album, artist.id, AlbumSource::PlaylistImport).await?;

        // Upsert track
        let track = upsert_track(db, spotify_track, album.id, track_spotify_id).await?;

        valid_track_ids.push(track.id);

        // Upsert playlist_tracks junction record
        upsert_playlist_track(db, playlist_id, track.id, position as i32, &playlist_track.added_at).await?;
    }

    // Remove tracks no longer in the playlist
    cleanup_removed_tracks(db, playlist_id, &valid_track_ids).await?;

    Ok(())
}

/// Sync Liked Songs as a synthetic playlist
async fn sync_liked_songs(
    db: &DatabaseConnection,
    spotify_service: &SpotifyService,
    access_token: &str,
) -> Result<()> {
    tracing::info!("Syncing Liked Songs");

    // Upsert the Liked Songs playlist record
    let playlist = upsert_liked_songs_playlist(db, spotify_service, access_token).await?;

    // Only sync tracks if enabled
    if !playlist.is_enabled {
        tracing::debug!("Liked Songs is disabled, skipping track sync");
        return Ok(());
    }

    // Fetch all saved tracks
    let spotify_tracks = spotify_service.fetch_saved_tracks(access_token).await?;
    tracing::info!("Fetched {} Liked Songs tracks", spotify_tracks.len());

    // Compute content hash for change detection
    let new_snapshot = compute_tracks_hash(&spotify_tracks);

    // Check if content changed
    let should_sync = playlist.snapshot_id.as_deref() != Some(&new_snapshot)
        || playlist.last_synced_at.is_none();

    if !should_sync {
        tracing::debug!("Liked Songs unchanged (hash match), skipping track sync");
        return Ok(());
    }

    // Sync tracks using existing function
    sync_playlist_tracks(db, playlist.id, &spotify_tracks).await?;

    // Update snapshot and last_synced_at
    let mut active: playlists::ActiveModel = playlist.into();
    active.snapshot_id = Set(Some(new_snapshot));
    active.last_synced_at = Set(Some(Utc::now().into()));
    active.updated_at = Set(Utc::now().into());
    active.update(db).await?;

    tracing::info!("Liked Songs sync completed");
    Ok(())
}

/// Upsert the Liked Songs synthetic playlist
async fn upsert_liked_songs_playlist(
    db: &DatabaseConnection,
    spotify_service: &SpotifyService,
    access_token: &str,
) -> Result<playlists::Model> {
    // Get current track count for metadata
    let total_tracks = spotify_service.get_saved_tracks_total(access_token).await?;

    match playlists::Entity::find()
        .filter(playlists::Column::SpotifyId.eq(LIKED_SONGS_SPOTIFY_ID))
        .one(db)
        .await?
    {
        Some(existing) => {
            // Update track count
            let mut active: playlists::ActiveModel = existing.into();
            active.total_tracks = Set(Some(total_tracks));
            active.updated_at = Set(Utc::now().into());
            Ok(active.update(db).await?)
        }
        None => {
            // Create new Liked Songs playlist
            let new_playlist = playlists::ActiveModel {
                name: Set(LIKED_SONGS_NAME.to_string()),
                spotify_id: Set(LIKED_SONGS_SPOTIFY_ID.to_string()),
                description: Set(Some("Your liked songs from Spotify".to_string())),
                owner_name: Set(None), // Liked Songs has no "owner"
                is_collaborative: Set(false),
                total_tracks: Set(Some(total_tracks)),
                cover_image_url: Set(None), // No cover for Liked Songs
                snapshot_id: Set(None), // Will be set after first sync
                is_enabled: Set(false), // Disabled by default like other playlists
                is_synthetic: Set(true), // Mark as synthetic playlist
                created_at: Set(Utc::now().into()),
                updated_at: Set(Utc::now().into()),
                last_synced_at: Set(None),
                ..Default::default()
            };

            let playlist = new_playlist.insert(db).await?;
            tracing::info!("Created Liked Songs playlist (id={})", playlist.id);
            Ok(playlist)
        }
    }
}

/// Compute a deterministic hash of track IDs for change detection
fn compute_tracks_hash(tracks: &[SpotifyPlaylistTrack]) -> String {
    let mut track_ids: Vec<&str> = tracks
        .iter()
        .filter_map(|t| t.track.as_ref())
        .filter_map(|t| t.id.as_deref())
        .collect();

    // Sort for consistent hashing regardless of pagination order
    track_ids.sort();

    let mut hasher = Sha256::new();
    for id in track_ids {
        hasher.update(id.as_bytes());
        hasher.update(b"|"); // delimiter between IDs
    }

    format!("{:x}", hasher.finalize())
}

/// Upsert an artist by Spotify ID
async fn upsert_artist(db: &DatabaseConnection, spotify_artist: &SpotifyArtist) -> Result<artists::Model> {
    match artists::Entity::find()
        .filter(artists::Column::SpotifyId.eq(&spotify_artist.id))
        .one(db)
        .await?
    {
        Some(existing) => Ok(existing),
        None => {
            let new_artist = artists::ActiveModel {
                name: Set(spotify_artist.name.clone()),
                spotify_id: Set(Some(spotify_artist.id.clone())),
                created_at: Set(Utc::now().into()),
                updated_at: Set(Utc::now().into()),
                ..Default::default()
            };
            Ok(new_artist.insert(db).await?)
        }
    }
}

/// Upsert an album by Spotify ID
async fn upsert_album(
    db: &DatabaseConnection,
    spotify_album: &SpotifyAlbum,
    artist_id: i32,
    source: AlbumSource,
) -> Result<albums::Model> {
    match albums::Entity::find()
        .filter(albums::Column::SpotifyId.eq(&spotify_album.id))
        .one(db)
        .await?
    {
        Some(existing) => Ok(existing),
        None => {
            let cover_url = spotify_album.images.first().map(|img| img.url.clone());

            let new_album = albums::ActiveModel {
                title: Set(spotify_album.name.clone()),
                artist_id: Set(artist_id),
                spotify_id: Set(Some(spotify_album.id.clone())),
                release_date: Set(parse_release_date(&spotify_album.release_date)),
                total_tracks: Set(Some(spotify_album.total_tracks)),
                cover_art_url: Set(cover_url),
                genres: Set(spotify_album.genres.as_ref().and_then(|g| serde_json::to_string(g).ok())),
                ownership_status: Set(OwnershipStatus::NotOwned.as_str().to_string()),
                match_status: Set(Some(MatchStatus::Pending.as_str().to_string())),
                source: Set(source.as_str().to_string()),
                created_at: Set(Utc::now().into()),
                updated_at: Set(Utc::now().into()),
                last_synced_at: Set(Some(Utc::now().into())),
                ..Default::default()
            };

            let album = new_album.insert(db).await?;
            tracing::debug!("Created album: {} (source: {:?})", spotify_album.name, source);
            Ok(album)
        }
    }
}

/// Upsert a track by Spotify ID
async fn upsert_track(
    db: &DatabaseConnection,
    spotify_track: &SpotifyTrack,
    album_id: i32,
    spotify_id: &str,
) -> Result<tracks::Model> {
    match tracks::Entity::find()
        .filter(tracks::Column::SpotifyId.eq(spotify_id))
        .one(db)
        .await?
    {
        Some(existing) => Ok(existing),
        None => {
            let new_track = tracks::ActiveModel {
                album_id: Set(album_id),
                title: Set(spotify_track.name.clone()),
                track_number: Set(Some(spotify_track.track_number)),
                disc_number: Set(Some(spotify_track.disc_number)),
                duration_ms: Set(Some(spotify_track.duration_ms)),
                spotify_id: Set(Some(spotify_id.to_string())),
                created_at: Set(Utc::now().into()),
                updated_at: Set(Utc::now().into()),
                ..Default::default()
            };

            let track = new_track.insert(db).await?;
            tracing::debug!("Created track: {}", spotify_track.name);
            Ok(track)
        }
    }
}

/// Upsert a playlist by Spotify ID
async fn upsert_playlist(db: &DatabaseConnection, spotify_playlist: &SpotifyPlaylist) -> Result<playlists::Model> {
    match playlists::Entity::find()
        .filter(playlists::Column::SpotifyId.eq(&spotify_playlist.id))
        .one(db)
        .await?
    {
        Some(existing) => {
            // Update playlist metadata (name, track count, etc.)
            let mut active: playlists::ActiveModel = existing.into();
            active.name = Set(spotify_playlist.name.clone());
            active.description = Set(spotify_playlist.description.clone());
            active.owner_name = Set(spotify_playlist.owner.display_name.clone());
            active.is_collaborative = Set(spotify_playlist.collaborative);
            active.total_tracks = Set(Some(spotify_playlist.tracks.total));
            active.cover_image_url = Set(spotify_playlist.images.first().map(|i| i.url.clone()));
            active.updated_at = Set(Utc::now().into());
            Ok(active.update(db).await?)
        }
        None => {
            let new_playlist = playlists::ActiveModel {
                name: Set(spotify_playlist.name.clone()),
                spotify_id: Set(spotify_playlist.id.clone()),
                description: Set(spotify_playlist.description.clone()),
                owner_name: Set(spotify_playlist.owner.display_name.clone()),
                is_collaborative: Set(spotify_playlist.collaborative),
                total_tracks: Set(Some(spotify_playlist.tracks.total)),
                cover_image_url: Set(spotify_playlist.images.first().map(|i| i.url.clone())),
                snapshot_id: Set(None), // Will be set after first sync
                is_enabled: Set(false),  // Disabled by default - user opts in
                is_synthetic: Set(false), // Regular Spotify playlist
                created_at: Set(Utc::now().into()),
                updated_at: Set(Utc::now().into()),
                last_synced_at: Set(None),
                ..Default::default()
            };

            let playlist = new_playlist.insert(db).await?;
            tracing::debug!("Created playlist: {}", spotify_playlist.name);
            Ok(playlist)
        }
    }
}

/// Upsert a playlist_tracks junction record
async fn upsert_playlist_track(
    db: &DatabaseConnection,
    playlist_id: i32,
    track_id: i32,
    position: i32,
    added_at: &Option<String>,
) -> Result<playlist_tracks::Model> {
    match playlist_tracks::Entity::find()
        .filter(playlist_tracks::Column::PlaylistId.eq(playlist_id))
        .filter(playlist_tracks::Column::TrackId.eq(track_id))
        .one(db)
        .await?
    {
        Some(existing) => {
            // Update position if changed
            let mut active: playlist_tracks::ActiveModel = existing.into();
            active.position = Set(position);
            active.updated_at = Set(Utc::now().into());
            Ok(active.update(db).await?)
        }
        None => {
            let added_at_parsed = added_at
                .as_ref()
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&Utc).into());

            let new_record = playlist_tracks::ActiveModel {
                playlist_id: Set(playlist_id),
                track_id: Set(track_id),
                position: Set(position),
                added_at: Set(added_at_parsed),
                created_at: Set(Utc::now().into()),
                updated_at: Set(Utc::now().into()),
                ..Default::default()
            };
            Ok(new_record.insert(db).await?)
        }
    }
}

/// Remove tracks from a playlist that are no longer in the Spotify playlist
async fn cleanup_removed_tracks(
    db: &DatabaseConnection,
    playlist_id: i32,
    valid_track_ids: &[i32],
) -> Result<()> {
    use sea_orm::Condition;

    if valid_track_ids.is_empty() {
        // If no valid tracks, delete all tracks for this playlist
        playlist_tracks::Entity::delete_many()
            .filter(playlist_tracks::Column::PlaylistId.eq(playlist_id))
            .exec(db)
            .await?;
    } else {
        // Delete tracks not in the valid list
        playlist_tracks::Entity::delete_many()
            .filter(
                Condition::all()
                    .add(playlist_tracks::Column::PlaylistId.eq(playlist_id))
                    .add(playlist_tracks::Column::TrackId.is_not_in(valid_track_ids.to_vec())),
            )
            .exec(db)
            .await?;
    }

    Ok(())
}

/// Parse release date in various formats (YYYY, YYYY-MM, YYYY-MM-DD)
fn parse_release_date(date_str: &str) -> Option<chrono::NaiveDate> {
    // Try full date first
    if let Ok(date) = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
        return Some(date);
    }
    // Try year-month
    if let Ok(date) = chrono::NaiveDate::parse_from_str(&format!("{}-01", date_str), "%Y-%m-%d") {
        return Some(date);
    }
    // Try year only
    if let Ok(date) = chrono::NaiveDate::parse_from_str(&format!("{}-01-01", date_str), "%Y-%m-%d") {
        return Some(date);
    }
    None
}
