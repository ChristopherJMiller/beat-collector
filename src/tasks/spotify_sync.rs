use anyhow::Result;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use uuid::Uuid;

use crate::{
    db::entities::{album, artist, user_settings, Album, Artist, UserSettings},
    services::SpotifyService,
    state::AppState,
};

pub async fn run_spotify_sync(state: AppState) -> Result<()> {
    tracing::info!("Starting Spotify sync job");

    // Get user settings with Spotify tokens
    let settings = UserSettings::find()
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

    // Fetch saved albums
    let albums = spotify_service.fetch_saved_albums(&access_token).await?;

    tracing::info!("Fetched {} albums from Spotify", albums.len());

    // Process each album
    for spotify_album in albums {
        // Get or create artist
        let artist_model = match Artist::find()
            .filter(artist::Column::SpotifyId.eq(&spotify_album.artists[0].id))
            .one(&state.db)
            .await?
        {
            Some(existing) => existing,
            None => {
                let new_artist = artist::ActiveModel {
                    id: Set(Uuid::new_v4()),
                    name: Set(spotify_album.artists[0].name.clone()),
                    spotify_id: Set(Some(spotify_album.artists[0].id.clone())),
                    created_at: Set(Utc::now().into()),
                    updated_at: Set(Utc::now().into()),
                    ..Default::default()
                };
                new_artist.insert(&state.db).await?
            }
        };

        // Get or create album
        let existing_album = Album::find()
            .filter(album::Column::SpotifyId.eq(&spotify_album.id))
            .one(&state.db)
            .await?;

        if existing_album.is_none() {
            let cover_url = spotify_album
                .images
                .first()
                .map(|img| img.url.clone());

            let new_album = album::ActiveModel {
                id: Set(Uuid::new_v4()),
                title: Set(spotify_album.name.clone()),
                artist_id: Set(artist_model.id),
                spotify_id: Set(Some(spotify_album.id.clone())),
                release_date: Set(chrono::NaiveDate::parse_from_str(
                    &spotify_album.release_date,
                    "%Y-%m-%d",
                )
                .ok()),
                total_tracks: Set(Some(spotify_album.total_tracks)),
                cover_art_url: Set(cover_url),
                genres: Set(spotify_album.genres),
                ownership_status: Set(album::OwnershipStatus::NotOwned),
                match_status: Set(album::MatchStatus::Pending),
                created_at: Set(Utc::now().into()),
                updated_at: Set(Utc::now().into()),
                last_synced_at: Set(Some(Utc::now().into())),
                ..Default::default()
            };

            new_album.insert(&state.db).await?;
            tracing::debug!("Created album: {}", spotify_album.name);
        }
    }

    tracing::info!("Spotify sync completed successfully");
    Ok(())
}
