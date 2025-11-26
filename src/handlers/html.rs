use axum::{
    extract::{Path, Query, State},
    response::Html,
};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect};
use serde::Deserialize;

use crate::{
    db::{
        entities::{albums, artists, playlists, user_settings},
        enums::OwnershipStatus,
    },
    error::Result,
    services::playlist_stats,
    state::AppState,
    templates::{
        album_detail_modal, album_grid_partial, artist_detail_page, artist_grid_partial,
        artists_page, home_page, jobs_page, playlists_page, playlist_detail_partial,
        playlist_grid_partial, playlist_tracks_rows, playlist_card_oob, settings_page,
        stats_page, AlbumCardData, ArtistCardData, PlaylistCardData, PlaylistTrackData,
    },
};

use super::albums::ListAlbumsQuery;
use super::artists::ListArtistsQuery;
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

    // Apply sorting
    let select = match query.sort_by.as_str() {
        "title" => {
            if query.sort_order == "asc" {
                select.order_by_asc(albums::Column::Title)
            } else {
                select.order_by_desc(albums::Column::Title)
            }
        }
        "artist" => {
            // For artist sorting, we need to join and order by artist name
            use sea_orm::{JoinType, RelationTrait};
            let select = select.join(JoinType::LeftJoin, albums::Relation::Artists.def());
            if query.sort_order == "asc" {
                select.order_by_asc(artists::Column::Name)
            } else {
                select.order_by_desc(artists::Column::Name)
            }
        }
        "release_date" => {
            if query.sort_order == "asc" {
                select.order_by_asc(albums::Column::ReleaseDate)
            } else {
                select.order_by_desc(albums::Column::ReleaseDate)
            }
        }
        _ => {
            // Default: created_at (date added)
            if query.sort_order == "asc" {
                select.order_by_asc(albums::Column::CreatedAt)
            } else {
                select.order_by_desc(albums::Column::CreatedAt)
            }
        }
    };

    // Get paginated results
    let albums = select
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
                artist_id: a.id,
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
            artist_id: artist.id,
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

    let (lidarr_url, music_folder) = match settings_result {
        Ok(Some(settings)) => (
            settings.lidarr_url,
            settings.music_folder_path,
        ),
        _ => (None, None),
    };

    Html(settings_page(lidarr_url, music_folder).into_string())
}

/// Jobs page
pub async fn jobs() -> Html<String> {
    Html(jobs_page().into_string())
}

/// Stats page
pub async fn stats() -> Html<String> {
    Html(stats_page().into_string())
}

/// Artists page
pub async fn artists() -> Html<String> {
    Html(artists_page().into_string())
}

/// Artists grid partial (for HTMX)
pub async fn artists_grid(
    State(state): State<AppState>,
    Query(query): Query<ListArtistsQuery>,
) -> Result<Html<String>> {
    use sea_orm::{FromQueryResult, JoinType, RelationTrait};

    let page = query.page.max(1);
    let page_size = query.page_size.min(200).max(1);

    // Build base query for filtering
    let mut base_filter = artists::Entity::find();

    if let Some(search) = &query.search {
        if !search.is_empty() {
            base_filter = base_filter.filter(
                artists::Column::Name
                    .contains(search)
                    .or(artists::Column::Name.like(&format!("%{}%", search))),
            );
        }
    }

    // Get total count
    let total_items = base_filter.clone().count(&state.db).await?;
    let total_pages = (total_items + page_size - 1) / page_size;

    // Get paginated artist IDs
    let artist_ids: Vec<i32> = base_filter
        .select_only()
        .column(artists::Column::Id)
        .order_by_asc(artists::Column::Name)
        .offset((page - 1) * page_size)
        .limit(page_size)
        .into_tuple()
        .all(&state.db)
        .await?;

    if artist_ids.is_empty() {
        let markup = artist_grid_partial(vec![], page, total_pages);
        return Ok(Html(markup.into_string()));
    }

    // Query artists with aggregate stats
    #[derive(FromQueryResult)]
    struct ArtistWithStats {
        id: i32,
        name: String,
        album_count: i64,
        owned_count: i64,
    }

    // Use raw SQL for the conditional count since SeaORM's CASE doesn't directly support .sum()
    let artists_with_stats: Vec<ArtistWithStats> = artists::Entity::find()
        .filter(artists::Column::Id.is_in(artist_ids.clone()))
        .select_only()
        .column(artists::Column::Id)
        .column(artists::Column::Name)
        .column_as(albums::Column::Id.count(), "album_count")
        .column_as(
            sea_orm::prelude::Expr::cust("SUM(CASE WHEN albums.ownership_status = 'owned' THEN 1 ELSE 0 END)"),
            "owned_count",
        )
        .join(JoinType::LeftJoin, artists::Relation::Albums.def())
        .group_by(artists::Column::Id)
        .group_by(artists::Column::Name)
        .into_model::<ArtistWithStats>()
        .all(&state.db)
        .await?;

    // Convert to card data and apply sorting
    let mut artist_data: Vec<ArtistCardData> = artists_with_stats
        .into_iter()
        .map(|a| {
            let ownership_percentage = if a.album_count > 0 {
                (a.owned_count as f64 / a.album_count as f64) * 100.0
            } else {
                0.0
            };
            ArtistCardData {
                id: a.id,
                name: a.name,
                album_count: a.album_count,
                owned_count: a.owned_count,
                ownership_percentage,
            }
        })
        .collect();

    // Sort based on query params
    match query.sort_by.as_str() {
        "album_count" => {
            if query.sort_order == "desc" {
                artist_data.sort_by(|a, b| b.album_count.cmp(&a.album_count));
            } else {
                artist_data.sort_by(|a, b| a.album_count.cmp(&b.album_count));
            }
        }
        "ownership" => {
            if query.sort_order == "desc" {
                artist_data.sort_by(|a, b| {
                    b.ownership_percentage
                        .partial_cmp(&a.ownership_percentage)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            } else {
                artist_data.sort_by(|a, b| {
                    a.ownership_percentage
                        .partial_cmp(&b.ownership_percentage)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
        }
        _ => {
            // Default: sort by name
            if query.sort_order == "desc" {
                artist_data.sort_by(|a, b| b.name.to_lowercase().cmp(&a.name.to_lowercase()));
            } else {
                artist_data.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
            }
        }
    }

    let markup = artist_grid_partial(artist_data, page, total_pages);
    Ok(Html(markup.into_string()))
}

/// Artist detail page (full page)
pub async fn artist_detail(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Html<String>> {
    // Get the artist
    let artist = artists::Entity::find_by_id(id)
        .one(&state.db)
        .await?;

    if let Some(artist) = artist {
        // Get all albums for this artist
        let artist_albums = albums::Entity::find()
            .filter(albums::Column::ArtistId.eq(id))
            .order_by_desc(albums::Column::ReleaseDate)
            .all(&state.db)
            .await?;

        let owned_count = artist_albums
            .iter()
            .filter(|a| a.ownership_status == "owned")
            .count() as i64;
        let album_count = artist_albums.len() as i64;
        let ownership_percentage = if album_count > 0 {
            (owned_count as f64 / album_count as f64) * 100.0
        } else {
            0.0
        };

        let artist_card_data = ArtistCardData {
            id: artist.id,
            name: artist.name.clone(),
            album_count,
            owned_count,
            ownership_percentage,
        };

        let album_data: Vec<AlbumCardData> = artist_albums
            .into_iter()
            .map(|album| AlbumCardData {
                id: album.id,
                title: album.title,
                artist_id: artist.id,
                artist_name: artist.name.clone(),
                cover_art_url: album.cover_art_url,
                release_date: album.release_date.map(|d| d.to_string()),
                ownership_status: OwnershipStatus::from_str(&album.ownership_status)
                    .unwrap_or(OwnershipStatus::NotOwned),
                match_score: album.match_score,
            })
            .collect();

        let markup = artist_detail_page(&artist_card_data, album_data);
        Ok(Html(markup.into_string()))
    } else {
        Ok(Html("<div class='p-4 text-red-600'>Artist not found</div>".to_string()))
    }
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
        .order_by_desc(playlists::Column::IsEnabled)  // Enabled playlists first
        .order_by_desc(playlists::Column::TotalTracks)
        .offset((page - 1) * page_size)
        .limit(page_size)
        .all(&state.db)
        .await?;

    // Batch fetch ownership stats for all playlists (single query!)
    let playlist_ids: Vec<i32> = playlist_models.iter().map(|p| p.id).collect();
    let stats_map = playlist_stats::get_batch_playlist_ownership_stats(&state.db, playlist_ids)
        .await
        .unwrap_or_default();

    let playlist_data: Vec<PlaylistCardData> = playlist_models
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

            PlaylistCardData {
                id: playlist.id,
                name: playlist.name,
                owner_name: playlist.owner_name,
                track_count: playlist.total_tracks.unwrap_or(0),
                owned_count: owned_count as i32,
                cover_image_url: playlist.cover_image_url,
                is_enabled: playlist.is_enabled,
                ownership_percentage,
                is_synthetic: playlist.is_synthetic,
            }
        })
        .collect();

    let markup = playlist_grid_partial(playlist_data, page, total_pages);
    Ok(Html(markup.into_string()))
}

/// Query parameters for playlist detail
#[derive(Deserialize)]
pub struct PlaylistDetailQuery {
    #[serde(default = "default_page")]
    pub page: u64,
}

fn default_page() -> u64 {
    1
}

const TRACKS_PER_PAGE: u64 = 50;

/// Playlist detail partial (for HTMX)
pub async fn playlist_detail(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Query(query): Query<PlaylistDetailQuery>,
) -> Result<Html<String>> {
    let playlist = playlists::Entity::find_by_id(id)
        .one(&state.db)
        .await?;

    if let Some(playlist) = playlist {
        // Use precomputed owned_count if available
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

        // Calculate pagination
        let page = query.page.max(1);
        let offset = (page - 1) * TRACKS_PER_PAGE;
        let total_pages = ((total_count as u64) + TRACKS_PER_PAGE - 1) / TRACKS_PER_PAGE;

        let (track_details, _total) = playlist_stats::get_playlist_tracks_paginated(
            &state.db,
            id,
            offset,
            TRACKS_PER_PAGE,
        )
        .await
        .unwrap_or_default();

        let track_data: Vec<PlaylistTrackData> = track_details
            .into_iter()
            .map(|t| PlaylistTrackData {
                position: t.position,
                track_name: t.track_name,
                artist_name: t.artist_name,
                album_id: t.album_id,
                album_name: t.album_name,
                duration_ms: t.duration_ms,
                ownership_status: OwnershipStatus::from_str(&t.ownership_status)
                    .unwrap_or(OwnershipStatus::NotOwned),
            })
            .collect();

        let markup = playlist_detail_partial(&playlist_data, track_data, page, total_pages.max(1));
        Ok(Html(markup.into_string()))
    } else {
        Ok(Html("<div class='p-4 text-red-600'>Playlist not found</div>".to_string()))
    }
}

/// Toggle playlist enabled and return updated modal (for HTMX)
pub async fn playlist_toggle(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Query(query): Query<PlaylistDetailQuery>,
) -> Result<Html<String>> {
    use sea_orm::{ActiveModelTrait, Set};

    // Find and toggle the playlist
    let playlist = playlists::Entity::find_by_id(id)
        .one(&state.db)
        .await?;

    if let Some(playlist) = playlist {
        let new_enabled = !playlist.is_enabled;

        let mut active: playlists::ActiveModel = playlist.into();
        active.is_enabled = Set(new_enabled);
        active.updated_at = Set(chrono::Utc::now().into());
        let playlist = active.update(&state.db).await?;

        // Now render the modal with updated data
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

        let page = query.page.max(1);
        let offset = (page - 1) * TRACKS_PER_PAGE;
        let total_pages = ((total_count as u64) + TRACKS_PER_PAGE - 1) / TRACKS_PER_PAGE;

        let (track_details, _total) = playlist_stats::get_playlist_tracks_paginated(
            &state.db,
            id,
            offset,
            TRACKS_PER_PAGE,
        )
        .await
        .unwrap_or_default();

        let track_data: Vec<PlaylistTrackData> = track_details
            .into_iter()
            .map(|t| PlaylistTrackData {
                position: t.position,
                track_name: t.track_name,
                artist_name: t.artist_name,
                album_id: t.album_id,
                album_name: t.album_name,
                duration_ms: t.duration_ms,
                ownership_status: OwnershipStatus::from_str(&t.ownership_status)
                    .unwrap_or(OwnershipStatus::NotOwned),
            })
            .collect();

        let modal_markup = playlist_detail_partial(&playlist_data, track_data, page, total_pages.max(1));
        let card_oob_markup = playlist_card_oob(&playlist_data);

        // Combine modal content with OOB card update
        let combined = format!("{}{}", modal_markup.into_string(), card_oob_markup.into_string());
        Ok(Html(combined))
    } else {
        Ok(Html("<div class='p-4 text-red-600'>Playlist not found</div>".to_string()))
    }
}

use super::playlists::PlaylistTracksQuery;

/// Playlist tracks partial (for HTMX infinite scroll)
pub async fn playlist_tracks_partial(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Query(query): Query<PlaylistTracksQuery>,
) -> Result<Html<String>> {
    let (track_details, total) = playlist_stats::get_playlist_tracks_paginated(
        &state.db,
        id,
        query.offset,
        query.limit,
    )
    .await?;

    let has_more = (query.offset + track_details.len() as u64) < total;

    let track_data: Vec<PlaylistTrackData> = track_details
        .into_iter()
        .map(|t| PlaylistTrackData {
            position: t.position,
            track_name: t.track_name,
            artist_name: t.artist_name,
            album_id: t.album_id,
            album_name: t.album_name,
            duration_ms: t.duration_ms,
            ownership_status: OwnershipStatus::from_str(&t.ownership_status)
                .unwrap_or(OwnershipStatus::NotOwned),
        })
        .collect();

    let markup = playlist_tracks_rows(track_data, has_more, id, query.offset + query.limit);
    Ok(Html(markup.into_string()))
}
