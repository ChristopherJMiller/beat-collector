use anyhow::Result;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};

use crate::{
    db::entities::{album, Album},
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
    let pending_albums = Album::find()
        .filter(album::Column::MatchStatus.eq("pending"))
        .find_also_related(crate::db::entities::Artist)
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
                        let mut active: album::ActiveModel = album_model.into();
                        active.musicbrainz_release_group_id = Set(Some(best_match.id));
                        active.match_score = Set(Some(best_match.score));
                        active.match_status = Set(if best_match.score >= 90 {
                            album::MatchStatus::Matched
                        } else if best_match.score >= 80 {
                            album::MatchStatus::ManualReview
                        } else {
                            album::MatchStatus::NoMatch
                        });
                        active.updated_at = Set(chrono::Utc::now().into());

                        active.update(&state.db).await?;
                        tracing::debug!(
                            "Matched with score {}: {}",
                            best_match.score,
                            best_match.title
                        );
                    } else {
                        // No match found
                        let mut active: album::ActiveModel = album_model.into();
                        active.match_status = Set(album::MatchStatus::NoMatch);
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
