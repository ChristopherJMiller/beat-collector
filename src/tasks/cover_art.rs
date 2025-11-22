use anyhow::Result;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use std::path::{Path, PathBuf};
use tokio::fs;

use crate::{
    db::entities::{album, Album},
    services::MusicBrainzService,
    state::AppState,
};

/// Download and store cover art for an album
pub async fn download_cover_art(
    state: &AppState,
    album_id: uuid::Uuid,
    mb_release_group_id: uuid::Uuid,
    covers_dir: &Path,
) -> Result<String> {
    // Ensure covers directory exists
    fs::create_dir_all(covers_dir).await?;

    let mb_service = MusicBrainzService::new(format!(
        "BeatCollector/0.1.0 ({})",
        state.config.spotify_client_id
    ));

    // Download cover art (500px size for good quality)
    tracing::debug!(
        "Downloading cover art for album {} from MusicBrainz {}",
        album_id,
        mb_release_group_id
    );

    let cover_data = mb_service
        .fetch_cover_art(
            mb_release_group_id,
            crate::services::CoverArtSize::Medium,
        )
        .await?;

    // Save to disk
    let file_name = format!("{}.jpg", album_id);
    let file_path = covers_dir.join(&file_name);

    fs::write(&file_path, &cover_data).await?;

    tracing::info!("Cover art saved to: {:?}", file_path);

    // Return the URL path (relative to static serving)
    Ok(format!("/static/covers/{}", file_name))
}

/// Download cover art for all matched albums that don't have local covers
pub async fn download_all_missing_covers(state: AppState) -> Result<()> {
    tracing::info!("Starting bulk cover art download");

    // Get static covers directory path
    let covers_dir = PathBuf::from("static/covers");

    // Find all albums with MusicBrainz IDs but no local cover art
    let albums = Album::find()
        .filter(album::Column::MusicbrainzReleaseGroupId.is_not_null())
        .filter(
            album::Column::CoverArtUrl
                .not_like("/static/covers/%")
                .or(album::Column::CoverArtUrl.is_null()),
        )
        .all(&state.db)
        .await?;

    tracing::info!("Found {} albums needing cover art", albums.len());

    for album_model in albums {
        if let Some(mb_id) = album_model.musicbrainz_release_group_id {
            match download_cover_art(&state, album_model.id, mb_id, &covers_dir).await {
                Ok(cover_url) => {
                    // Update database with local cover art URL
                    let mut active: album::ActiveModel = album_model.into();
                    active.cover_art_url = Set(Some(cover_url));
                    active.updated_at = Set(chrono::Utc::now().into());
                    active.update(&state.db).await?;

                    tracing::debug!("Updated album with local cover art URL");

                    // Small delay to be respectful to Cover Art Archive
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
                Err(e) => {
                    // Log but continue - some albums may not have cover art
                    tracing::warn!(
                        "Failed to download cover art for album {}: {}",
                        album_model.id,
                        e
                    );
                }
            }
        }
    }

    tracing::info!("Bulk cover art download completed");
    Ok(())
}
