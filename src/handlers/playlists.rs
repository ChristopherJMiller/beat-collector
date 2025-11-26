use axum::{
    extract::{Path, Query, State},
    Json,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set,
};
use serde::{Deserialize, Serialize};

use crate::{
    db::entities::{playlists},
    error::{AppError, Result},
    services::playlist_stats,
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
        .order_by_desc(playlists::Column::IsEnabled)  // Enabled playlists first
        .order_by_asc(playlists::Column::Name)
        .offset((page - 1) * page_size)
        .limit(page_size)
        .all(&state.db)
        .await?;

    // Batch fetch ownership stats for all playlists on this page (single query!)
    let playlist_ids: Vec<i32> = playlist_models.iter().map(|p| p.id).collect();
    let stats_map = playlist_stats::get_batch_playlist_ownership_stats(&state.db, playlist_ids)
        .await
        .unwrap_or_default();

    let playlist_responses: Vec<PlaylistResponse> = playlist_models
        .into_iter()
        .map(|playlist| {
            // Use precomputed owned_count if available, otherwise use batch stats
            let (owned_count, total_count) = if let Some(precomputed) = playlist.owned_count {
                (precomputed as i64, playlist.total_tracks.unwrap_or(0) as i64)
            } else {
                stats_map.get(&playlist.id).copied().unwrap_or((0, 0))
            };

            let ownership_percentage = if total_count > 0 {
                (owned_count as f64 / total_count as f64) * 100.0
            } else {
                0.0
            };

            PlaylistResponse {
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
            }
        })
        .collect();

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

/// Get a single playlist with all its tracks (use paginated version for large playlists)
pub async fn get_playlist(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<PlaylistDetailResponse>> {
    let playlist = playlists::Entity::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Playlist not found".to_string()))?;

    // Use precomputed owned_count if available, otherwise calculate
    let total_count = playlist.total_tracks.unwrap_or(0) as i64;
    let owned_count = if let Some(precomputed) = playlist.owned_count {
        precomputed as i64
    } else {
        playlist_stats::recalculate_playlist_owned_count(&state.db, playlist.id)
            .await
            .unwrap_or(0) as i64
    };

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

    // Use optimized paginated query - get first batch of tracks
    let (track_details, _total) = playlist_stats::get_playlist_tracks_paginated(&state.db, id, 0, 100)
        .await
        .unwrap_or_default();

    let tracks: Vec<PlaylistTrackResponse> = track_details
        .into_iter()
        .map(|t| PlaylistTrackResponse {
            id: t.id,
            position: t.position,
            track_name: t.track_name,
            artist_name: t.artist_name,
            album_id: t.album_id,
            album_name: t.album_name,
            duration_ms: t.duration_ms,
            ownership_status: t.ownership_status,
            added_at: None,
        })
        .collect();

    Ok(Json(PlaylistDetailResponse {
        playlist: playlist_response,
        tracks,
    }))
}

#[derive(Deserialize)]
pub struct PlaylistTracksQuery {
    #[serde(default)]
    pub offset: u64,
    #[serde(default = "default_track_limit")]
    pub limit: u64,
}

fn default_track_limit() -> u64 {
    50
}

#[derive(Serialize)]
pub struct PaginatedTracksResponse {
    pub tracks: Vec<PlaylistTrackResponse>,
    pub has_more: bool,
    pub total: u64,
    pub next_offset: u64,
}

/// Get paginated tracks for a playlist (for infinite scroll)
pub async fn get_playlist_tracks(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Query(query): Query<PlaylistTracksQuery>,
) -> Result<Json<PaginatedTracksResponse>> {
    // Verify playlist exists
    let _playlist = playlists::Entity::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Playlist not found".to_string()))?;

    let (track_details, total) = playlist_stats::get_playlist_tracks_paginated(
        &state.db,
        id,
        query.offset,
        query.limit,
    )
    .await?;

    let has_more = (query.offset + track_details.len() as u64) < total;

    let tracks: Vec<PlaylistTrackResponse> = track_details
        .into_iter()
        .map(|t| PlaylistTrackResponse {
            id: t.id,
            position: t.position,
            track_name: t.track_name,
            artist_name: t.artist_name,
            album_id: t.album_id,
            album_name: t.album_name,
            duration_ms: t.duration_ms,
            ownership_status: t.ownership_status,
            added_at: None,
        })
        .collect();

    Ok(Json(PaginatedTracksResponse {
        tracks,
        has_more,
        total,
        next_offset: query.offset + query.limit,
    }))
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

    // Use precomputed owned_count if available
    let total_count = updated.total_tracks.unwrap_or(0) as i64;
    let owned_count = if let Some(precomputed) = updated.owned_count {
        precomputed as i64
    } else {
        playlist_stats::recalculate_playlist_owned_count(&state.db, updated.id)
            .await
            .unwrap_or(0) as i64
    };

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
