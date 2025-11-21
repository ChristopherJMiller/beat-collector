use axum::{
    extract::{Path, Query, State},
    response::Html,
};
use maud::Markup;
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};
use uuid::Uuid;

use crate::{
    db::entities::{album, artist, Album, Artist, UserSettings},
    error::Result,
    state::AppState,
    templates::{
        album_detail_modal, album_grid_partial, home_page, jobs_page, settings_page, stats_page,
        AlbumCardData,
    },
};

use super::albums::ListAlbumsQuery;

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

    let album_data: Vec<AlbumCardData> = albums
        .into_iter()
        .filter_map(|(album, artist)| {
            artist.map(|a| AlbumCardData {
                id: album.id,
                title: album.title,
                artist_name: a.name,
                cover_art_url: album.cover_art_url,
                release_date: album.release_date.map(|d| d.to_string()),
                ownership_status: album.ownership_status,
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
    Path(id): Path<Uuid>,
) -> Result<Html<String>> {
    let album_with_artist = Album::find_by_id(id)
        .find_also_related(Artist)
        .one(&state.db)
        .await?;

    if let Some((album, Some(artist))) = album_with_artist {
        let album_data = AlbumCardData {
            id: album.id,
            title: album.title.clone(),
            artist_name: artist.name.clone(),
            cover_art_url: album.cover_art_url.clone(),
            release_date: album.release_date.map(|d| d.to_string()),
            ownership_status: album.ownership_status,
            match_score: album.match_score,
        };

        let markup = album_detail_modal(
            &album_data,
            &artist.name,
            &album.genres,
            album.total_tracks,
        );
        Ok(Html(markup.into_string()))
    } else {
        Ok(Html("<div class='p-4 text-red-600'>Album not found</div>".to_string()))
    }
}

/// Settings page
pub async fn settings(State(state): State<AppState>) -> Html<String> {
    let settings_result = UserSettings::find().one(&state.db).await;

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
