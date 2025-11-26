use axum::{
    extract::{Path, Query, State},
    Json,
};
use sea_orm::{
    ColumnTrait, EntityTrait, FromQueryResult, JoinType, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, RelationTrait,
};
use serde::{Deserialize, Serialize};

use crate::{
    db::entities::{albums, artists},
    error::{AppError, Result},
    state::AppState,
};

#[derive(Deserialize)]
pub struct ListArtistsQuery {
    pub search: Option<String>,
    #[serde(default = "default_sort_by")]
    pub sort_by: String,
    #[serde(default = "default_sort_order")]
    pub sort_order: String,
    #[serde(default = "default_page")]
    pub page: u64,
    #[serde(default = "default_page_size")]
    pub page_size: u64,
}

fn default_sort_by() -> String {
    "name".to_string()
}

fn default_sort_order() -> String {
    "asc".to_string()
}

fn default_page() -> u64 {
    1
}

fn default_page_size() -> u64 {
    50
}

#[derive(Serialize, Clone)]
pub struct ArtistResponse {
    pub id: i32,
    pub name: String,
    pub album_count: i64,
    pub owned_count: i64,
    pub not_owned_count: i64,
    pub ownership_percentage: f64,
}

#[derive(Serialize)]
pub struct PaginatedArtistsResponse {
    pub artists: Vec<ArtistResponse>,
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
pub struct ArtistDetailResponse {
    pub artist: ArtistResponse,
    pub albums: Vec<ArtistAlbumResponse>,
}

#[derive(Serialize)]
pub struct ArtistAlbumResponse {
    pub id: i32,
    pub title: String,
    pub cover_art_url: Option<String>,
    pub release_date: Option<String>,
    pub ownership_status: String,
    pub match_score: Option<i32>,
}

/// Internal struct for querying artist with album stats
#[derive(FromQueryResult)]
struct ArtistWithStats {
    id: i32,
    name: String,
    album_count: i64,
    owned_count: i64,
}

/// List artists with album statistics
pub async fn list_artists(
    State(state): State<AppState>,
    Query(query): Query<ListArtistsQuery>,
) -> Result<Json<PaginatedArtistsResponse>> {
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

    // Get total count of matching artists
    let total_items = base_filter.clone().count(&state.db).await?;
    let total_pages = (total_items + page_size - 1) / page_size;

    // Get paginated artist IDs first (for proper pagination with aggregates)
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
        return Ok(Json(PaginatedArtistsResponse {
            artists: vec![],
            pagination: PaginationInfo {
                page,
                page_size,
                total_items,
                total_pages,
            },
        }));
    }

    // Query artists with aggregate stats
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

    // Convert to response and apply sorting
    let mut artist_responses: Vec<ArtistResponse> = artists_with_stats
        .into_iter()
        .map(|a| {
            let ownership_percentage = if a.album_count > 0 {
                (a.owned_count as f64 / a.album_count as f64) * 100.0
            } else {
                0.0
            };
            ArtistResponse {
                id: a.id,
                name: a.name,
                album_count: a.album_count,
                owned_count: a.owned_count,
                not_owned_count: a.album_count - a.owned_count,
                ownership_percentage,
            }
        })
        .collect();

    // Sort based on query params
    match query.sort_by.as_str() {
        "album_count" => {
            if query.sort_order == "desc" {
                artist_responses.sort_by(|a, b| b.album_count.cmp(&a.album_count));
            } else {
                artist_responses.sort_by(|a, b| a.album_count.cmp(&b.album_count));
            }
        }
        "ownership" => {
            if query.sort_order == "desc" {
                artist_responses.sort_by(|a, b| {
                    b.ownership_percentage
                        .partial_cmp(&a.ownership_percentage)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            } else {
                artist_responses.sort_by(|a, b| {
                    a.ownership_percentage
                        .partial_cmp(&b.ownership_percentage)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
        }
        _ => {
            // Default: sort by name
            if query.sort_order == "desc" {
                artist_responses.sort_by(|a, b| b.name.to_lowercase().cmp(&a.name.to_lowercase()));
            } else {
                artist_responses.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
            }
        }
    }

    Ok(Json(PaginatedArtistsResponse {
        artists: artist_responses,
        pagination: PaginationInfo {
            page,
            page_size,
            total_items,
            total_pages,
        },
    }))
}

/// Get a single artist with their albums
pub async fn get_artist(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ArtistDetailResponse>> {
    // Get the artist
    let artist = artists::Entity::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Artist not found".to_string()))?;

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

    let artist_response = ArtistResponse {
        id: artist.id,
        name: artist.name,
        album_count,
        owned_count,
        not_owned_count: album_count - owned_count,
        ownership_percentage,
    };

    let album_responses: Vec<ArtistAlbumResponse> = artist_albums
        .into_iter()
        .map(|album| ArtistAlbumResponse {
            id: album.id,
            title: album.title,
            cover_art_url: album.cover_art_url,
            release_date: album.release_date.map(|d| d.to_string()),
            ownership_status: album.ownership_status,
            match_score: album.match_score,
        })
        .collect();

    Ok(Json(ArtistDetailResponse {
        artist: artist_response,
        albums: album_responses,
    }))
}
