use axum::{
    extract::{Path, Query, State},
    Json,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set,
};
use serde::{Deserialize, Serialize};

use crate::{
    db::entities::{albums, artists, playlist_tracks, playlists, tracks},
    error::{AppError, Result},
    state::AppState,
};

#[derive(Deserialize)]
pub struct ListPlaylistsQuery {
    pub is_enabled: Option<bool>,
    #[serde(default = "default_page")]
    pub page: u64,
    #[serde(default = "default_page_size")]
    pub page_size: u64,
}

fn default_page() -> u64 {
    1
}

fn default_page_size() -> u64 {
    50
}

#[derive(Serialize)]
pub struct PlaylistResponse {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub owner_name: Option<String>,
    pub is_collaborative: bool,
    pub total_tracks: i32,
    pub cover_image_url: Option<String>,
    pub is_enabled: bool,
    pub is_synthetic: bool,
    pub owned_count: i64,
    pub ownership_percentage: f64,
    pub last_synced_at: Option<String>,
}

#[derive(Serialize)]
pub struct PaginatedPlaylistsResponse {
    pub playlists: Vec<PlaylistResponse>,
    pub pagination: PaginationInfo,
}

#[derive(Serialize)]
pub struct PaginationInfo {
    pub page: u64,
    pub page_size: u64,
    pub total_items: u64,
    pub total_pages: u64,
}

#[derive(Serialize)]
pub struct PlaylistTrackResponse {
    pub id: i32,
    pub position: i32,
    pub track_name: String,
    pub artist_name: String,
    pub album_id: i32,
    pub album_name: String,
    pub duration_ms: Option<i32>,
    pub ownership_status: String,
    pub added_at: Option<String>,
}

#[derive(Serialize)]
pub struct PlaylistDetailResponse {
    pub playlist: PlaylistResponse,
    pub tracks: Vec<PlaylistTrackResponse>,
}

/// List all playlists with ownership statistics
pub async fn list_playlists(
    State(state): State<AppState>,
    Query(query): Query<ListPlaylistsQuery>,
) -> Result<Json<PaginatedPlaylistsResponse>> {
    let page = query.page.max(1);
    let page_size = query.page_size.min(200).max(1);

    let mut select = playlists::Entity::find();

    if let Some(enabled) = query.is_enabled {
        select = select.filter(playlists::Column::IsEnabled.eq(enabled));
    }

    let total_items = select.clone().count(&state.db).await?;
    let total_pages = (total_items + page_size - 1) / page_size;

    let playlist_models = select
        .order_by_asc(playlists::Column::Name)
        .offset((page - 1) * page_size)
        .limit(page_size)
        .all(&state.db)
        .await?;

    let mut playlist_responses = Vec::with_capacity(playlist_models.len());

    for playlist in playlist_models {
        let (owned_count, total_count) = get_playlist_ownership_stats(&state, playlist.id).await?;
        let ownership_percentage = if total_count > 0 {
            (owned_count as f64 / total_count as f64) * 100.0
        } else {
            0.0
        };

        playlist_responses.push(PlaylistResponse {
            id: playlist.id,
            name: playlist.name,
            description: playlist.description,
            owner_name: playlist.owner_name,
            is_collaborative: playlist.is_collaborative,
            total_tracks: playlist.total_tracks.unwrap_or(0),
            cover_image_url: playlist.cover_image_url,
            is_enabled: playlist.is_enabled,
            is_synthetic: playlist.is_synthetic,
            owned_count,
            ownership_percentage,
            last_synced_at: playlist.last_synced_at.map(|dt| dt.to_rfc3339()),
        });
    }

    Ok(Json(PaginatedPlaylistsResponse {
        playlists: playlist_responses,
        pagination: PaginationInfo {
            page,
            page_size,
            total_items,
            total_pages,
        },
    }))
}

/// Get a single playlist with all its tracks
pub async fn get_playlist(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<PlaylistDetailResponse>> {
    let playlist = playlists::Entity::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Playlist not found".to_string()))?;

    let (owned_count, total_count) = get_playlist_ownership_stats(&state, playlist.id).await?;
    let ownership_percentage = if total_count > 0 {
        (owned_count as f64 / total_count as f64) * 100.0
    } else {
        0.0
    };

    let playlist_response = PlaylistResponse {
        id: playlist.id,
        name: playlist.name.clone(),
        description: playlist.description.clone(),
        owner_name: playlist.owner_name.clone(),
        is_collaborative: playlist.is_collaborative,
        total_tracks: playlist.total_tracks.unwrap_or(0),
        cover_image_url: playlist.cover_image_url.clone(),
        is_enabled: playlist.is_enabled,
        is_synthetic: playlist.is_synthetic,
        owned_count,
        ownership_percentage,
        last_synced_at: playlist.last_synced_at.map(|dt| dt.to_rfc3339()),
    };

    let tracks = get_playlist_tracks_with_details(&state, id).await?;

    Ok(Json(PlaylistDetailResponse {
        playlist: playlist_response,
        tracks,
    }))
}

/// Get tracks for a playlist
pub async fn get_playlist_tracks(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<Vec<PlaylistTrackResponse>>> {
    // Verify playlist exists
    let _playlist = playlists::Entity::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Playlist not found".to_string()))?;

    let tracks = get_playlist_tracks_with_details(&state, id).await?;
    Ok(Json(tracks))
}

/// Toggle playlist enabled status
pub async fn toggle_playlist_enabled(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<PlaylistResponse>> {
    let playlist = playlists::Entity::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Playlist not found".to_string()))?;

    let new_enabled = !playlist.is_enabled;

    let mut active: playlists::ActiveModel = playlist.into();
    active.is_enabled = Set(new_enabled);
    active.updated_at = Set(chrono::Utc::now().into());
    let updated = active.update(&state.db).await?;

    let (owned_count, total_count) = get_playlist_ownership_stats(&state, updated.id).await?;
    let ownership_percentage = if total_count > 0 {
        (owned_count as f64 / total_count as f64) * 100.0
    } else {
        0.0
    };

    Ok(Json(PlaylistResponse {
        id: updated.id,
        name: updated.name,
        description: updated.description,
        owner_name: updated.owner_name,
        is_collaborative: updated.is_collaborative,
        total_tracks: updated.total_tracks.unwrap_or(0),
        cover_image_url: updated.cover_image_url,
        is_enabled: updated.is_enabled,
        is_synthetic: updated.is_synthetic,
        owned_count,
        ownership_percentage,
        last_synced_at: updated.last_synced_at.map(|dt| dt.to_rfc3339()),
    }))
}

/// Helper: Get ownership statistics for a playlist
async fn get_playlist_ownership_stats(state: &AppState, playlist_id: i32) -> Result<(i64, i64)> {
    // Get all tracks in this playlist with their albums
    let playlist_track_records = playlist_tracks::Entity::find()
        .filter(playlist_tracks::Column::PlaylistId.eq(playlist_id))
        .all(&state.db)
        .await?;

    if playlist_track_records.is_empty() {
        return Ok((0, 0));
    }

    let track_ids: Vec<i32> = playlist_track_records.iter().map(|pt| pt.track_id).collect();

    // Get tracks with their albums
    let track_models = tracks::Entity::find()
        .filter(tracks::Column::Id.is_in(track_ids))
        .all(&state.db)
        .await?;

    let album_ids: Vec<i32> = track_models.iter().map(|t| t.album_id).collect();

    // Get unique albums and count owned ones
    let unique_album_ids: std::collections::HashSet<i32> = album_ids.into_iter().collect();

    let owned_albums = albums::Entity::find()
        .filter(albums::Column::Id.is_in(unique_album_ids.iter().cloned().collect::<Vec<_>>()))
        .filter(albums::Column::OwnershipStatus.eq("owned"))
        .count(&state.db)
        .await?;

    // For track-level ownership, we count tracks whose albums are owned
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

/// Helper: Get playlist tracks with full details
async fn get_playlist_tracks_with_details(
    state: &AppState,
    playlist_id: i32,
) -> Result<Vec<PlaylistTrackResponse>> {
    let playlist_track_records = playlist_tracks::Entity::find()
        .filter(playlist_tracks::Column::PlaylistId.eq(playlist_id))
        .order_by_asc(playlist_tracks::Column::Position)
        .all(&state.db)
        .await?;

    let mut responses = Vec::with_capacity(playlist_track_records.len());

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
                responses.push(PlaylistTrackResponse {
                    id: pt.id,
                    position: pt.position,
                    track_name: track.title,
                    artist_name: artist.name,
                    album_id: album.id,
                    album_name: album.title,
                    duration_ms: track.duration_ms,
                    ownership_status: album.ownership_status,
                    added_at: pt.added_at.map(|dt| dt.to_rfc3339()),
                });
            }
        }
    }

    Ok(responses)
}
