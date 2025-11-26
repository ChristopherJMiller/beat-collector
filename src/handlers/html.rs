use axum::{
    extract::{Path, Query, State},
    response::Html,
};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect};

use crate::{
    db::{
        entities::{albums, artists, playlist_tracks, playlists, tracks, user_settings},
        enums::OwnershipStatus,
    },
    error::Result,
    state::AppState,
    templates::{
        album_detail_modal, album_grid_partial, home_page, jobs_page, playlists_page,
        playlist_detail_partial, playlist_grid_partial, settings_page, stats_page,
        AlbumCardData, PlaylistCardData, PlaylistTrackData,
    },
};

use super::albums::ListAlbumsQuery;
use super::playlists::ListPlaylistsQuery;

/// Home page with album grid
pub async fn index() -> Html<String> {
    Html(home_page().into_string())
}

/// Album grid partial (for HTMX updates)
pub async fn albums_grid(
    State(state): State<AppState>,
    Query(query): Query<ListAlbumsQuery>,
) -> Result<Html<String>> {
    let page = query.page.max(1);
    let page_size = query.page_size.min(200).max(1);

    let mut select = albums::Entity::find();

    // Apply filters
    if let Some(status) = &query.ownership_status {
        select = select.filter(albums::Column::OwnershipStatus.eq(status));
    }

    if let Some(match_status) = &query.match_status {
        select = select.filter(albums::Column::MatchStatus.eq(match_status));
    }

    if let Some(artist_id) = query.artist_id {
        select = select.filter(albums::Column::ArtistId.eq(artist_id));
    }

    if let Some(search) = &query.search {
        select = select.filter(
            albums::Column::Title
                .contains(search)
                .or(albums::Column::Title.like(&format!("%{}%", search))),
        );
    }

    // Get total count
    let total_items = select.clone().count(&state.db).await?;
    let total_pages = (total_items + page_size - 1) / page_size;

    // Get paginated results
    let albums = select
        .order_by_desc(albums::Column::CreatedAt)
        .offset((page - 1) * page_size)
        .limit(page_size)
        .find_also_related(artists::Entity)
        .all(&state.db)
        .await?;

    let album_data: Vec<AlbumCardData> = albums
        .into_iter()
        .filter_map(|(album, artist)| {
            artist.map(|a| AlbumCardData {
                id: album.id,
                title: album.title,
                artist_name: a.name,
                cover_art_url: album.cover_art_url,
                release_date: album.release_date.map(|d| d.to_string()),
                ownership_status: OwnershipStatus::from_str(&album.ownership_status).unwrap_or(OwnershipStatus::NotOwned),
                match_score: album.match_score,
            })
        })
        .collect();

    let markup = album_grid_partial(album_data, page, total_pages);
    Ok(Html(markup.into_string()))
}

/// Album detail modal (for HTMX)
pub async fn album_detail(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Html<String>> {
    let album_with_artist = albums::Entity::find_by_id(id)
        .find_also_related(artists::Entity)
        .one(&state.db)
        .await?;

    if let Some((album, Some(artist))) = album_with_artist {
        let album_data = AlbumCardData {
            id: album.id,
            title: album.title.clone(),
            artist_name: artist.name.clone(),
            cover_art_url: album.cover_art_url.clone(),
            release_date: album.release_date.map(|d| d.to_string()),
            ownership_status: OwnershipStatus::from_str(&album.ownership_status).unwrap_or(OwnershipStatus::NotOwned),
            match_score: album.match_score,
        };

        let genres: Option<Vec<String>> = album.genres.and_then(|g| serde_json::from_str(&g).ok());
        let markup = album_detail_modal(
            &album_data,
            &artist.name,
            &genres,
            album.total_tracks,
        );
        Ok(Html(markup.into_string()))
    } else {
        Ok(Html("<div class='p-4 text-red-600'>Album not found</div>".to_string()))
    }
}

/// Settings page
pub async fn settings(State(state): State<AppState>) -> Html<String> {
    let settings_result = user_settings::Entity::find().one(&state.db).await;

    let (lidarr_url, music_folder, spotify_connected) = match settings_result {
        Ok(Some(settings)) => (
            settings.lidarr_url,
            settings.music_folder_path,
            settings.spotify_access_token.is_some(),
        ),
        _ => (None, None, false),
    };

    Html(settings_page(lidarr_url, music_folder, spotify_connected).into_string())
}

/// Jobs page
pub async fn jobs() -> Html<String> {
    Html(jobs_page().into_string())
}

/// Stats page
pub async fn stats() -> Html<String> {
    Html(stats_page().into_string())
}

/// Playlists page
pub async fn playlists() -> Html<String> {
    Html(playlists_page().into_string())
}

/// Playlists grid partial (for HTMX)
pub async fn playlists_grid(
    State(state): State<AppState>,
    Query(query): Query<ListPlaylistsQuery>,
) -> Result<Html<String>> {
    let page = query.page.max(1);
    let page_size = query.page_size.min(200).max(1);

    let mut select = playlists::Entity::find();

    if let Some(enabled) = query.is_enabled {
        select = select.filter(playlists::Column::IsEnabled.eq(enabled));
    }

    let total_items = select.clone().count(&state.db).await?;
    let total_pages = (total_items + page_size - 1) / page_size;

    let playlist_models = select
        .order_by_desc(playlists::Column::TotalTracks)
        .offset((page - 1) * page_size)
        .limit(page_size)
        .all(&state.db)
        .await?;

    let mut playlist_data = Vec::with_capacity(playlist_models.len());

    for playlist in playlist_models {
        let (owned_count, total_count) = get_playlist_ownership_stats(&state, playlist.id).await?;
        let ownership_percentage = if total_count > 0 {
            (owned_count as f64 / total_count as f64) * 100.0
        } else {
            0.0
        };

        playlist_data.push(PlaylistCardData {
            id: playlist.id,
            name: playlist.name,
            owner_name: playlist.owner_name,
            track_count: playlist.total_tracks.unwrap_or(0),
            owned_count: owned_count as i32,
            cover_image_url: playlist.cover_image_url,
            is_enabled: playlist.is_enabled,
            ownership_percentage,
            is_synthetic: playlist.is_synthetic,
        });
    }

    let markup = playlist_grid_partial(playlist_data, page, total_pages);
    Ok(Html(markup.into_string()))
}

/// Playlist detail partial (for HTMX)
pub async fn playlist_detail(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Html<String>> {
    let playlist = playlists::Entity::find_by_id(id)
        .one(&state.db)
        .await?;

    if let Some(playlist) = playlist {
        let (owned_count, total_count) = get_playlist_ownership_stats(&state, playlist.id).await?;
        let ownership_percentage = if total_count > 0 {
            (owned_count as f64 / total_count as f64) * 100.0
        } else {
            0.0
        };

        let playlist_data = PlaylistCardData {
            id: playlist.id,
            name: playlist.name.clone(),
            owner_name: playlist.owner_name.clone(),
            track_count: playlist.total_tracks.unwrap_or(0),
            owned_count: owned_count as i32,
            cover_image_url: playlist.cover_image_url.clone(),
            is_enabled: playlist.is_enabled,
            ownership_percentage,
            is_synthetic: playlist.is_synthetic,
        };

        let track_data = get_playlist_tracks_for_display(&state, id).await?;

        let markup = playlist_detail_partial(&playlist_data, track_data);
        Ok(Html(markup.into_string()))
    } else {
        Ok(Html("<div class='p-4 text-red-600'>Playlist not found</div>".to_string()))
    }
}

/// Helper: Get ownership statistics for a playlist
async fn get_playlist_ownership_stats(state: &AppState, playlist_id: i32) -> Result<(i64, i64)> {
    let playlist_track_records = playlist_tracks::Entity::find()
        .filter(playlist_tracks::Column::PlaylistId.eq(playlist_id))
        .all(&state.db)
        .await?;

    if playlist_track_records.is_empty() {
        return Ok((0, 0));
    }

    let track_ids: Vec<i32> = playlist_track_records.iter().map(|pt| pt.track_id).collect();

    let track_models = tracks::Entity::find()
        .filter(tracks::Column::Id.is_in(track_ids))
        .all(&state.db)
        .await?;

    let mut owned_count = 0i64;
    for track in &track_models {
        let album = albums::Entity::find_by_id(track.album_id)
            .one(&state.db)
            .await?;
        if let Some(album) = album {
            if album.ownership_status == "owned" {
                owned_count += 1;
            }
        }
    }

    Ok((owned_count, track_models.len() as i64))
}

/// Helper: Get playlist tracks for display
async fn get_playlist_tracks_for_display(
    state: &AppState,
    playlist_id: i32,
) -> Result<Vec<PlaylistTrackData>> {
    let playlist_track_records = playlist_tracks::Entity::find()
        .filter(playlist_tracks::Column::PlaylistId.eq(playlist_id))
        .order_by_asc(playlist_tracks::Column::Position)
        .all(&state.db)
        .await?;

    let mut track_data = Vec::with_capacity(playlist_track_records.len());

    for pt in playlist_track_records {
        let track = tracks::Entity::find_by_id(pt.track_id)
            .one(&state.db)
            .await?;

        if let Some(track) = track {
            let album = albums::Entity::find_by_id(track.album_id)
                .find_also_related(artists::Entity)
                .one(&state.db)
                .await?;

            if let Some((album, Some(artist))) = album {
                track_data.push(PlaylistTrackData {
                    position: pt.position,
                    track_name: track.title,
                    artist_name: artist.name,
                    album_id: album.id,
                    album_name: album.title,
                    duration_ms: track.duration_ms,
                    ownership_status: OwnershipStatus::from_str(&album.ownership_status)
                        .unwrap_or(OwnershipStatus::NotOwned),
                });
            }
        }
    }

    Ok(track_data)
}
