use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};

use crate::{
    db::{
        entities::{albums, artists, lidarr_downloads},
        enums::{AcquisitionSource, OwnershipStatus},
    },
    error::Result,
    services::LidarrWebhook,
    state::AppState,
};

/// Handle Lidarr webhook notifications
pub async fn webhook(
    State(state): State<AppState>,
    Json(payload): Json<LidarrWebhook>,
) -> Result<StatusCode> {
    tracing::info!("Received Lidarr webhook: {:?}", payload);

    match payload {
        LidarrWebhook::Grab {
            artist,
            albums,
            download_id,
        } => {
            handle_grab(&state, artist, albums, download_id).await?;
        }
        LidarrWebhook::Download {
            artist,
            albums,
            track_files,
            is_upgrade,
        } => {
            handle_download(&state, artist, albums, track_files, is_upgrade).await?;
        }
        LidarrWebhook::AlbumDownload { artist, album } => {
            handle_album_download(&state, artist, album).await?;
        }
        LidarrWebhook::DownloadFailure {
            artist,
            albums,
            message,
        } => {
            handle_download_failure(&state, artist, albums, message).await?;
        }
    }

    Ok(StatusCode::OK)
}

/// Handle "Grab" event - album download started
async fn handle_grab(
    state: &AppState,
    artist: crate::services::LidarrArtist,
    albums: Vec<crate::services::LidarrAlbum>,
    download_id: String,
) -> Result<()> {
    for lidarr_album in albums {
        // Find matching album in database
        if let Some(album) = find_album_by_title_and_artist(
            state,
            &lidarr_album.title,
            &artist.artist_name,
        )
        .await?
        {
            // Update album status to Downloading
            let mut active: albums::ActiveModel = album.clone().into();
            active.ownership_status = Set(OwnershipStatus::Downloading.as_str().to_string());
            active.updated_at = Set(Utc::now().into());
            active.update(&state.db).await?;

            // Create lidarr_download record
            let download_record = lidarr_downloads::ActiveModel {
                album_id: Set(album.id),
                lidarr_album_id: Set(Some(lidarr_album.id)),
                download_id: Set(Some(download_id.clone())),
                status: Set("grabbing".to_string()),
                created_at: Set(Utc::now().into()),
                ..Default::default()
            };
            download_record.insert(&state.db).await?;

            tracing::info!(
                "Album '{}' download started (download_id: {})",
                lidarr_album.title,
                download_id
            );
        }
    }

    Ok(())
}

/// Handle "Download" event - album successfully imported
async fn handle_download(
    state: &AppState,
    artist: crate::services::LidarrArtist,
    albums: Vec<crate::services::LidarrAlbum>,
    track_files: Vec<crate::services::TrackFile>,
    _is_upgrade: bool,
) -> Result<()> {
    for lidarr_album in albums {
        if let Some(album) = find_album_by_title_and_artist(
            state,
            &lidarr_album.title,
            &artist.artist_name,
        )
        .await?
        {
            // Extract local path from first track file
            let local_path = track_files
                .first()
                .and_then(|tf| {
                    std::path::Path::new(&tf.path)
                        .parent()
                        .map(|p| p.to_string_lossy().to_string())
                });

            // Update album to Owned status
            let mut active: albums::ActiveModel = album.clone().into();
            active.ownership_status = Set(OwnershipStatus::Owned.as_str().to_string());
            active.acquisition_source = Set(Some(AcquisitionSource::Lidarr.as_str().to_string()));
            active.local_path = Set(local_path);
            active.updated_at = Set(Utc::now().into());
            active.update(&state.db).await?;

            // Update playlist owned_count
            if let Err(e) = crate::services::playlist_stats::update_playlists_for_album(&state.db, album.id).await {
                tracing::warn!("Failed to update playlist stats after download: {}", e);
            }

            // Update lidarr_download record
            if let Some(download) = lidarr_downloads::Entity::find()
                .filter(lidarr_downloads::Column::AlbumId.eq(album.id))
                .filter(lidarr_downloads::Column::LidarrAlbumId.eq(lidarr_album.id))
                .one(&state.db)
                .await?
            {
                let mut active_download: lidarr_downloads::ActiveModel = download.into();
                active_download.status = Set("completed".to_string());
                active_download.completed_at = Set(Some(Utc::now().into()));
                active_download.update(&state.db).await?;
            }

            tracing::info!(
                "Album '{}' by '{}' successfully downloaded and imported",
                lidarr_album.title,
                artist.artist_name
            );
        }
    }

    Ok(())
}

/// Handle "AlbumDownload" event
async fn handle_album_download(
    state: &AppState,
    artist: crate::services::LidarrArtist,
    album: crate::services::LidarrAlbum,
) -> Result<()> {
    // Similar to handle_download but for a single album
    if let Some(db_album) = find_album_by_title_and_artist(
        state,
        &album.title,
        &artist.artist_name,
    )
    .await?
    {
        let mut active: albums::ActiveModel = db_album.clone().into();
        active.ownership_status = Set(OwnershipStatus::Owned.as_str().to_string());
        active.acquisition_source = Set(Some(AcquisitionSource::Lidarr.as_str().to_string()));
        active.updated_at = Set(Utc::now().into());
        active.update(&state.db).await?;

        // Update playlist owned_count
        if let Err(e) = crate::services::playlist_stats::update_playlists_for_album(&state.db, db_album.id).await {
            tracing::warn!("Failed to update playlist stats after album download: {}", e);
        }

        tracing::info!(
            "Album '{}' by '{}' download completed",
            album.title,
            artist.artist_name
        );
    }

    Ok(())
}

/// Handle "DownloadFailure" event
async fn handle_download_failure(
    state: &AppState,
    artist: crate::services::LidarrArtist,
    albums: Vec<crate::services::LidarrAlbum>,
    error_message: String,
) -> Result<()> {
    for lidarr_album in albums {
        if let Some(album) = find_album_by_title_and_artist(
            state,
            &lidarr_album.title,
            &artist.artist_name,
        )
        .await?
        {
            // Update album back to NotOwned
            let mut active: albums::ActiveModel = album.clone().into();
            active.ownership_status = Set(OwnershipStatus::NotOwned.as_str().to_string());
            active.updated_at = Set(Utc::now().into());
            active.update(&state.db).await?;

            // Update playlist owned_count
            if let Err(e) = crate::services::playlist_stats::update_playlists_for_album(&state.db, album.id).await {
                tracing::warn!("Failed to update playlist stats after download failure: {}", e);
            }

            // Update lidarr_download record
            if let Some(download) = lidarr_downloads::Entity::find()
                .filter(lidarr_downloads::Column::AlbumId.eq(album.id))
                .filter(lidarr_downloads::Column::LidarrAlbumId.eq(lidarr_album.id))
                .one(&state.db)
                .await?
            {
                let mut active_download: lidarr_downloads::ActiveModel = download.into();
                active_download.status = Set("failed".to_string());
                active_download.error_message = Set(Some(error_message.clone()));
                active_download.update(&state.db).await?;
            }

            tracing::error!(
                "Album '{}' download failed: {}",
                lidarr_album.title,
                error_message
            );
        }
    }

    Ok(())
}

/// Find album in database by title and artist name (fuzzy match)
async fn find_album_by_title_and_artist(
    state: &AppState,
    title: &str,
    artist_name: &str,
) -> Result<Option<albums::Model>> {
    // Find all artists and albums, then fuzzy match
    // This is not the most efficient but works for moderate sizes
    let artists = artists::Entity::find()
        .all(&state.db)
        .await?;

    let matching_artist = artists.iter().find(|a| {
        a.name.to_lowercase() == artist_name.to_lowercase()
            || similarity_score(&a.name.to_lowercase(), &artist_name.to_lowercase()) > 0.85
    });

    if let Some(artist) = matching_artist {
        let albums = albums::Entity::find()
            .filter(albums::Column::ArtistId.eq(artist.id))
            .all(&state.db)
            .await?;

        let matching_album = albums.into_iter().find(|alb| {
            alb.title.to_lowercase() == title.to_lowercase()
                || similarity_score(&alb.title.to_lowercase(), &title.to_lowercase()) > 0.85
        });

        Ok(matching_album)
    } else {
        Ok(None)
    }
}

/// Simple normalized Levenshtein distance for string similarity
fn similarity_score(s1: &str, s2: &str) -> f64 {
    let len1 = s1.chars().count();
    let len2 = s2.chars().count();

    if len1 == 0 && len2 == 0 {
        return 1.0;
    }

    let distance = levenshtein_distance(s1, s2);
    let max_len = len1.max(len2);

    1.0 - (distance as f64 / max_len as f64)
}

fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let s1_chars: Vec<char> = s1.chars().collect();
    let s2_chars: Vec<char> = s2.chars().collect();
    let len1 = s1_chars.len();
    let len2 = s2_chars.len();

    let mut matrix = vec![vec![0; len2 + 1]; len1 + 1];

    for i in 0..=len1 {
        matrix[i][0] = i;
    }
    for j in 0..=len2 {
        matrix[0][j] = j;
    }

    for i in 1..=len1 {
        for j in 1..=len2 {
            let cost = if s1_chars[i - 1] == s2_chars[j - 1] {
                0
            } else {
                1
            };
            matrix[i][j] = (matrix[i - 1][j] + 1)
                .min(matrix[i][j - 1] + 1)
                .min(matrix[i - 1][j - 1] + cost);
        }
    }

    matrix[len1][len2]
}
