use anyhow::Result;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};

use crate::{
    db::{
        entities::{albums, artists, user_settings},
        enums::{MatchStatus, OwnershipStatus},
    },
    services::SpotifyService,
    state::AppState,
};

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

    // Fetch saved albums
    let albums = spotify_service.fetch_saved_albums(&access_token).await?;

    tracing::info!("Fetched {} albums from Spotify", albums.len());

    // Process each album
    for spotify_album in albums {
        // Get or create artist
        let artist_model = match artists::Entity::find()
            .filter(artists::Column::SpotifyId.eq(&spotify_album.artists[0].id))
            .one(&state.db)
            .await?
        {
            Some(existing) => existing,
            None => {
                let new_artist = artists::ActiveModel {
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
        let existing_album = albums::Entity::find()
            .filter(albums::Column::SpotifyId.eq(&spotify_album.id))
            .one(&state.db)
            .await?;

        if existing_album.is_none() {
            let cover_url = spotify_album
                .images
                .first()
                .map(|img| img.url.clone());

            let new_album = albums::ActiveModel {
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
                genres: Set(spotify_album.genres.and_then(|g| serde_json::to_string(&g).ok())),
                ownership_status: Set(OwnershipStatus::NotOwned.as_str().to_string()),
                match_status: Set(Some(MatchStatus::Pending.as_str().to_string())),
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
