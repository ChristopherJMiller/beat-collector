use anyhow::Result;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};

use crate::{
    db::{
        entities::{albums, artists},
        enums::MatchStatus,
    },
    services::MusicBrainzService,
    state::AppState,
};

pub async fn run_musicbrainz_match(state: AppState) -> Result<()> {
    tracing::info!("Starting MusicBrainz matching job");

    // Initialize MusicBrainz service
    let mb_service = MusicBrainzService::new(format!(
        "BeatCollector/0.1.0 ({})",
        state.config.spotify_client_id
    ));

    // Get all albums with pending match status
    let pending_albums = albums::Entity::find()
        .filter(albums::Column::MatchStatus.eq("pending"))
        .find_also_related(artists::Entity)
        .all(&state.db)
        .await?;

    tracing::info!("Found {} albums to match", pending_albums.len());

    for (album_model, artist_option) in pending_albums {
        if let Some(artist) = artist_option {
            tracing::debug!("Matching album: {} by {}", album_model.title, artist.name);

            // Search MusicBrainz
            match mb_service
                .search_release_group(&artist.name, &album_model.title)
                .await
            {
                Ok(matches) => {
                    if let Some(best_match) = matches.first() {
                        let album_id = album_model.id;
                        let mb_id = best_match.id;

                        let mut active: albums::ActiveModel = album_model.into();
                        active.musicbrainz_release_group_id = Set(Some(mb_id.to_string()));
                        active.match_score = Set(Some(best_match.score));
                        active.match_status = Set(Some(if best_match.score >= 90 {
                            MatchStatus::Matched.as_str().to_string()
                        } else if best_match.score >= 80 {
                            MatchStatus::ManualReview.as_str().to_string()
                        } else {
                            MatchStatus::NoMatch.as_str().to_string()
                        }));
                        active.updated_at = Set(chrono::Utc::now().into());

                        active.update(&state.db).await?;
                        tracing::debug!(
                            "Matched with score {}: {}",
                            best_match.score,
                            best_match.title
                        );

                        // Download cover art after successful match
                        let covers_dir = std::path::PathBuf::from("static/covers");
                        match super::cover_art::download_cover_art(&state, album_id, &mb_id.to_string(), &covers_dir).await {
                            Ok(cover_url) => {
                                // Update album with local cover art URL
                                let album_for_cover = albums::Entity::find_by_id(album_id)
                                    .one(&state.db)
                                    .await?;

                                if let Some(alb) = album_for_cover {
                                    let mut active_cover: albums::ActiveModel = alb.into();
                                    active_cover.cover_art_url = Set(Some(cover_url));
                                    active_cover.updated_at = Set(chrono::Utc::now().into());
                                    active_cover.update(&state.db).await?;
                                    tracing::debug!("Cover art downloaded and saved");
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Failed to download cover art: {}", e);
                                // Continue even if cover art fails
                            }
                        }
                    } else {
                        // No match found
                        let mut active: albums::ActiveModel = album_model.into();
                        active.match_status = Set(Some(MatchStatus::NoMatch.as_str().to_string()));
                        active.updated_at = Set(chrono::Utc::now().into());
                        active.update(&state.db).await?;
                        tracing::debug!("No match found");
                    }
                }
                Err(e) => {
                    tracing::error!("Error matching album: {}", e);
                    // Continue to next album
                }
            }
        }
    }

    tracing::info!("MusicBrainz matching completed");
    Ok(())
}
