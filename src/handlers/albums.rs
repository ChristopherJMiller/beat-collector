use axum::{
    extract::{Path, Query, State},
    Json,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    db::entities::{album, artist, Album, Artist},
    error::{AppError, Result},
    state::AppState,
};

#[derive(Deserialize)]
pub struct ListAlbumsQuery {
    pub ownership_status: Option<String>,
    pub match_status: Option<String>,
    pub artist_id: Option<Uuid>,
    pub search: Option<String>,
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
pub struct AlbumResponse {
    pub id: Uuid,
    pub title: String,
    pub artist: ArtistResponse,
    pub cover_art_url: Option<String>,
    pub release_date: Option<String>,
    pub ownership_status: String,
    pub match_score: Option<i32>,
    pub genres: Option<Vec<String>>,
}

#[derive(Serialize)]
pub struct ArtistResponse {
    pub id: Uuid,
    pub name: String,
}

#[derive(Serialize)]
pub struct PaginatedAlbumsResponse {
    pub albums: Vec<AlbumResponse>,
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
pub struct StatsResponse {
    pub total_albums: u64,
    pub owned_albums: u64,
    pub not_owned_albums: u64,
    pub downloading_albums: u64,
    pub matched_albums: u64,
    pub unmatched_albums: u64,
    pub total_artists: u64,
}

#[derive(Deserialize)]
pub struct UpdateAlbumRequest {
    pub ownership_status: Option<String>,
    pub acquisition_source: Option<String>,
    pub local_path: Option<String>,
}

pub async fn list_albums(
    State(state): State<AppState>,
    Query(query): Query<ListAlbumsQuery>,
) -> Result<Json<PaginatedAlbumsResponse>> {
    let page = query.page.max(1);
    let page_size = query.page_size.min(200).max(1);

    let mut select = Album::find();

    // Apply filters
    if let Some(status) = &query.ownership_status {
        select = select.filter(album::Column::OwnershipStatus.eq(status));
    }

    if let Some(match_status) = &query.match_status {
        select = select.filter(album::Column::MatchStatus.eq(match_status));
    }

    if let Some(artist_id) = query.artist_id {
        select = select.filter(album::Column::ArtistId.eq(artist_id));
    }

    if let Some(search) = &query.search {
        select = select.filter(
            album::Column::Title
                .contains(search)
                .or(album::Column::Title.like(&format!("%{}%", search))),
        );
    }

    // Get total count
    let total_items = select.clone().count(&state.db).await?;
    let total_pages = (total_items + page_size - 1) / page_size;

    // Get paginated results
    let albums = select
        .order_by_desc(album::Column::CreatedAt)
        .offset((page - 1) * page_size)
        .limit(page_size)
        .find_also_related(Artist)
        .all(&state.db)
        .await?;

    let album_responses: Vec<AlbumResponse> = albums
        .into_iter()
        .filter_map(|(album, artist)| {
            artist.map(|a| AlbumResponse {
                id: album.id,
                title: album.title,
                artist: ArtistResponse {
                    id: a.id,
                    name: a.name,
                },
                cover_art_url: album.cover_art_url,
                release_date: album.release_date.map(|d| d.to_string()),
                ownership_status: format!("{:?}", album.ownership_status),
                match_score: album.match_score,
                genres: album.genres,
            })
        })
        .collect();

    Ok(Json(PaginatedAlbumsResponse {
        albums: album_responses,
        pagination: PaginationInfo {
            page,
            page_size,
            total_items,
            total_pages,
        },
    }))
}

pub async fn get_album(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<AlbumResponse>> {
    let album_with_artist = Album::find_by_id(id)
        .find_also_related(Artist)
        .one(&state.db)
        .await?;

    match album_with_artist {
        Some((album, Some(artist))) => Ok(Json(AlbumResponse {
            id: album.id,
            title: album.title,
            artist: ArtistResponse {
                id: artist.id,
                name: artist.name,
            },
            cover_art_url: album.cover_art_url,
            release_date: album.release_date.map(|d| d.to_string()),
            ownership_status: format!("{:?}", album.ownership_status),
            match_score: album.match_score,
            genres: album.genres,
        })),
        _ => Err(AppError::NotFound("Album not found".to_string())),
    }
}

pub async fn update_album(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateAlbumRequest>,
) -> Result<Json<AlbumResponse>> {
    let album = Album::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Album not found".to_string()))?;

    let mut active: album::ActiveModel = album.into();

    if let Some(status) = payload.ownership_status {
        // Parse the ownership status
        let ownership_status = match status.as_str() {
            "not_owned" => album::OwnershipStatus::NotOwned,
            "owned" => album::OwnershipStatus::Owned,
            "downloading" => album::OwnershipStatus::Downloading,
            _ => return Err(AppError::Internal("Invalid ownership status".to_string())),
        };
        active.ownership_status = Set(ownership_status);
    }

    if let Some(source) = payload.acquisition_source {
        let acquisition_source = match source.as_str() {
            "bandcamp" => Some(album::AcquisitionSource::Bandcamp),
            "physical" => Some(album::AcquisitionSource::Physical),
            "lidarr" => Some(album::AcquisitionSource::Lidarr),
            "unknown" => Some(album::AcquisitionSource::Unknown),
            _ => None,
        };
        active.acquisition_source = Set(acquisition_source);
    }

    if let Some(path) = payload.local_path {
        active.local_path = Set(Some(path));
    }

    active.updated_at = Set(chrono::Utc::now().into());
    let updated = active.update(&state.db).await?;

    // Fetch with artist for response
    get_album(State(state), Path(id)).await
}

pub async fn trigger_match(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    // This would trigger a background job to match this specific album
    // For now, return a placeholder
    Ok(Json(serde_json::json!({
        "message": "Match job queued",
        "album_id": id
    })))
}

pub async fn search_lidarr(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    // This would trigger a Lidarr search for this album
    // For now, return a placeholder
    Ok(Json(serde_json::json!({
        "message": "Lidarr search triggered",
        "album_id": id
    })))
}

pub async fn get_stats(State(state): State<AppState>) -> Result<Json<StatsResponse>> {
    let total_albums = Album::find().count(&state.db).await?;

    let owned_albums = Album::find()
        .filter(album::Column::OwnershipStatus.eq("owned"))
        .count(&state.db)
        .await?;

    let not_owned_albums = Album::find()
        .filter(album::Column::OwnershipStatus.eq("not_owned"))
        .count(&state.db)
        .await?;

    let downloading_albums = Album::find()
        .filter(album::Column::OwnershipStatus.eq("downloading"))
        .count(&state.db)
        .await?;

    let matched_albums = Album::find()
        .filter(album::Column::MatchStatus.eq("matched"))
        .count(&state.db)
        .await?;

    let unmatched_albums = Album::find()
        .filter(album::Column::MatchStatus.eq("pending"))
        .count(&state.db)
        .await?;

    let total_artists = Artist::find().count(&state.db).await?;

    Ok(Json(StatsResponse {
        total_albums,
        owned_albums,
        not_owned_albums,
        downloading_albums,
        matched_albums,
        unmatched_albums,
        total_artists,
    }))
}
