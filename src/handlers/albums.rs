use axum::{
    extract::{Path, Query, State},
    Json,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use serde::{Deserialize, Serialize};

use crate::{
    db::{
        entities::{albums, artists, user_settings},
        enums::{AcquisitionSource, OwnershipStatus},
    },
    error::{AppError, Result},
    state::AppState,
};

#[derive(Deserialize)]
pub struct ListAlbumsQuery {
    pub ownership_status: Option<String>,
    pub match_status: Option<String>,
    pub artist_id: Option<i32>,
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
    pub id: i32,
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
    pub id: i32,
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
                genres: album.genres.and_then(|g| serde_json::from_str(&g).ok()),
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
    Path(id): Path<i32>,
) -> Result<Json<AlbumResponse>> {
    let album_with_artist = albums::Entity::find_by_id(id)
        .find_also_related(artists::Entity)
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
            genres: album.genres.and_then(|g| serde_json::from_str(&g).ok()),
        })),
        _ => Err(AppError::NotFound("Album not found".to_string())),
    }
}

pub async fn update_album(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateAlbumRequest>,
) -> Result<Json<AlbumResponse>> {
    let album = albums::Entity::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Album not found".to_string()))?;

    let mut active: albums::ActiveModel = album.into();

    if let Some(status) = payload.ownership_status {
        // Parse the ownership status
        let ownership_status = match status.as_str() {
            "not_owned" => OwnershipStatus::NotOwned,
            "owned" => OwnershipStatus::Owned,
            "downloading" => OwnershipStatus::Downloading,
            _ => return Err(AppError::Internal("Invalid ownership status".to_string())),
        };
        active.ownership_status = Set(ownership_status.as_str().to_string());
    }

    if let Some(source) = payload.acquisition_source {
        let acquisition_source = match source.as_str() {
            "bandcamp" => Some(AcquisitionSource::Bandcamp),
            "physical" => Some(AcquisitionSource::Physical),
            "lidarr" => Some(AcquisitionSource::Lidarr),
            "unknown" => Some(AcquisitionSource::Unknown),
            _ => None,
        };
        active.acquisition_source = Set(acquisition_source.map(|s| s.as_str().to_string()));
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
    Path(id): Path<i32>,
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
    Path(id): Path<i32>,
) -> Result<Json<serde_json::Value>> {
    use crate::services::LidarrService;

    // Get user settings for Lidarr configuration
    let settings = user_settings::Entity::find()
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::Internal("User settings not found".to_string()))?;

    let lidarr_url = settings
        .lidarr_url
        .ok_or_else(|| AppError::Internal("Lidarr URL not configured".to_string()))?;

    let lidarr_api_key = settings
        .lidarr_api_key
        .ok_or_else(|| AppError::Internal("Lidarr API key not configured".to_string()))?;

    // Get the album from database
    let album = albums::Entity::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Album not found".to_string()))?;

    // Get MusicBrainz ID
    let mb_id = album
        .musicbrainz_release_group_id
        .clone()
        .ok_or_else(|| {
            AppError::Internal(
                "Album not matched to MusicBrainz. Please match it first.".to_string(),
            )
        })?;

    let lidarr_service = LidarrService::new();

    // Lookup album in Lidarr by MusicBrainz ID
    let lidarr_album = lidarr_service
        .lookup_album(&lidarr_url, &lidarr_api_key, &mb_id.to_string())
        .await?;

    match lidarr_album {
        Some(lidarr_alb) => {
            // Album exists in Lidarr, trigger search
            let search_result = lidarr_service
                .search_album(&lidarr_url, &lidarr_api_key, lidarr_alb.id)
                .await?;

            // Update album status to Downloading
            let mut active: albums::ActiveModel = album.into();
            active.ownership_status = Set(OwnershipStatus::Downloading.as_str().to_string());
            active.updated_at = Set(chrono::Utc::now().into());
            active.update(&state.db).await?;

            Ok(Json(serde_json::json!({
                "success": true,
                "message": "Lidarr search triggered",
                "command_id": search_result.id,
                "album_id": id
            })))
        }
        None => {
            // Album doesn't exist in Lidarr yet
            // TODO: Implement adding album to Lidarr first
            Ok(Json(serde_json::json!({
                "success": false,
                "message": "Album not found in Lidarr. Please add it to Lidarr first.",
                "album_id": id
            })))
        }
    }
}

pub async fn get_stats(State(state): State<AppState>) -> Result<Json<StatsResponse>> {
    let total_albums = albums::Entity::find().count(&state.db).await?;

    let owned_albums = albums::Entity::find()
        .filter(albums::Column::OwnershipStatus.eq("owned"))
        .count(&state.db)
        .await?;

    let not_owned_albums = albums::Entity::find()
        .filter(albums::Column::OwnershipStatus.eq("not_owned"))
        .count(&state.db)
        .await?;

    let downloading_albums = albums::Entity::find()
        .filter(albums::Column::OwnershipStatus.eq("downloading"))
        .count(&state.db)
        .await?;

    let matched_albums = albums::Entity::find()
        .filter(albums::Column::MatchStatus.eq("matched"))
        .count(&state.db)
        .await?;

    let unmatched_albums = albums::Entity::find()
        .filter(albums::Column::MatchStatus.eq("pending"))
        .count(&state.db)
        .await?;

    let total_artists = artists::Entity::find().count(&state.db).await?;

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
