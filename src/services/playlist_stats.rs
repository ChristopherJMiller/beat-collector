use anyhow::Result;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, FromQueryResult, JoinType,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, RelationTrait, Set,
};
use std::collections::HashSet;
use tracing::info;

use crate::db::entities::{albums, playlist_tracks, playlists, tracks};

/// Recalculate and update owned_count for playlists containing tracks from a specific album
pub async fn update_playlists_for_album(db: &DatabaseConnection, album_id: i32) -> Result<()> {
    // Find all tracks belonging to this album
    let track_ids: Vec<i32> = tracks::Entity::find()
        .filter(tracks::Column::AlbumId.eq(album_id))
        .select_only()
        .column(tracks::Column::Id)
        .into_tuple()
        .all(db)
        .await?;

    if track_ids.is_empty() {
        return Ok(());
    }

    // Find all unique playlist IDs containing these tracks
    let playlist_ids: Vec<i32> = playlist_tracks::Entity::find()
        .filter(playlist_tracks::Column::TrackId.is_in(track_ids))
        .select_only()
        .column(playlist_tracks::Column::PlaylistId)
        .distinct()
        .into_tuple()
        .all(db)
        .await?;

    info!(
        "Updating owned_count for {} playlists affected by album {}",
        playlist_ids.len(),
        album_id
    );

    // Recalculate owned_count for each affected playlist
    for playlist_id in playlist_ids {
        let owned_count = recalculate_playlist_owned_count(db, playlist_id).await?;

        if let Some(playlist) = playlists::Entity::find_by_id(playlist_id).one(db).await? {
            let mut active: playlists::ActiveModel = playlist.into();
            active.owned_count = Set(Some(owned_count));
            active.updated_at = Set(Utc::now().into());
            active.update(db).await?;
        }
    }

    Ok(())
}

/// Calculate owned track count for a single playlist
pub async fn recalculate_playlist_owned_count(
    db: &DatabaseConnection,
    playlist_id: i32,
) -> Result<i32> {
    // Get all playlist tracks with their album ownership status
    #[derive(FromQueryResult)]
    struct TrackOwnership {
        ownership_status: String,
    }

    let results: Vec<TrackOwnership> = playlist_tracks::Entity::find()
        .filter(playlist_tracks::Column::PlaylistId.eq(playlist_id))
        .select_only()
        .column(albums::Column::OwnershipStatus)
        .join(JoinType::InnerJoin, playlist_tracks::Relation::Tracks.def())
        .join(JoinType::InnerJoin, tracks::Relation::Albums.def())
        .into_model::<TrackOwnership>()
        .all(db)
        .await?;

    let owned_count = results
        .iter()
        .filter(|t| t.ownership_status == "owned")
        .count() as i32;

    Ok(owned_count)
}

/// Recalculate owned_count for ALL playlists (for initial backfill after migration)
pub async fn recalculate_all_playlist_stats(db: &DatabaseConnection) -> Result<u64> {
    let all_playlists = playlists::Entity::find().all(db).await?;
    let count = all_playlists.len() as u64;

    info!("Recalculating owned_count for {} playlists", count);

    for (i, playlist) in all_playlists.into_iter().enumerate() {
        let owned_count = recalculate_playlist_owned_count(db, playlist.id).await?;

        let mut active: playlists::ActiveModel = playlist.into();
        active.owned_count = Set(Some(owned_count));
        active.updated_at = Set(Utc::now().into());
        active.update(db).await?;

        if (i + 1) % 100 == 0 {
            info!("Processed {}/{} playlists", i + 1, count);
        }
    }

    info!("Completed recalculating owned_count for {} playlists", count);
    Ok(count)
}

/// Batch fetch ownership stats for multiple playlists (for list views)
/// Returns a map of playlist_id -> (owned_count, total_count)
pub async fn get_batch_playlist_ownership_stats(
    db: &DatabaseConnection,
    playlist_ids: Vec<i32>,
) -> Result<std::collections::HashMap<i32, (i64, i64)>> {
    use std::collections::HashMap;

    if playlist_ids.is_empty() {
        return Ok(HashMap::new());
    }

    // Get all playlist tracks with their ownership status for the given playlists
    #[derive(FromQueryResult)]
    struct PlaylistTrackOwnership {
        playlist_id: i32,
        ownership_status: String,
    }

    let results: Vec<PlaylistTrackOwnership> = playlist_tracks::Entity::find()
        .filter(playlist_tracks::Column::PlaylistId.is_in(playlist_ids.clone()))
        .select_only()
        .column(playlist_tracks::Column::PlaylistId)
        .column(albums::Column::OwnershipStatus)
        .join(JoinType::InnerJoin, playlist_tracks::Relation::Tracks.def())
        .join(JoinType::InnerJoin, tracks::Relation::Albums.def())
        .into_model::<PlaylistTrackOwnership>()
        .all(db)
        .await?;

    // Aggregate results by playlist_id
    let mut stats_map: HashMap<i32, (i64, i64)> = HashMap::new();

    // Initialize all requested playlist_ids with (0, 0)
    for id in &playlist_ids {
        stats_map.insert(*id, (0, 0));
    }

    // Count owned and total for each playlist
    for row in results {
        let entry = stats_map.entry(row.playlist_id).or_insert((0, 0));
        entry.1 += 1; // total count
        if row.ownership_status == "owned" {
            entry.0 += 1; // owned count
        }
    }

    Ok(stats_map)
}

/// Get paginated tracks for a playlist with all details (optimized single query)
#[derive(Debug, Clone)]
pub struct PlaylistTrackDetails {
    pub id: i32,
    pub position: i32,
    pub track_name: String,
    pub duration_ms: Option<i32>,
    pub album_id: i32,
    pub album_name: String,
    pub ownership_status: String,
    pub artist_name: String,
}

pub async fn get_playlist_tracks_paginated(
    db: &DatabaseConnection,
    playlist_id: i32,
    offset: u64,
    limit: u64,
) -> Result<(Vec<PlaylistTrackDetails>, u64)> {
    use crate::db::entities::artists;

    // Get total count
    let total = playlist_tracks::Entity::find()
        .filter(playlist_tracks::Column::PlaylistId.eq(playlist_id))
        .count(db)
        .await?;

    // Single JOIN query for paginated tracks
    #[derive(FromQueryResult)]
    struct TrackRow {
        id: i32,
        position: i32,
        track_name: String,
        duration_ms: Option<i32>,
        album_id: i32,
        album_name: String,
        ownership_status: String,
        artist_name: String,
    }

    let tracks: Vec<TrackRow> = playlist_tracks::Entity::find()
        .filter(playlist_tracks::Column::PlaylistId.eq(playlist_id))
        .select_only()
        .column(playlist_tracks::Column::Id)
        .column(playlist_tracks::Column::Position)
        .column_as(tracks::Column::Title, "track_name")
        .column_as(tracks::Column::DurationMs, "duration_ms")
        .column_as(albums::Column::Id, "album_id")
        .column_as(albums::Column::Title, "album_name")
        .column_as(albums::Column::OwnershipStatus, "ownership_status")
        .column_as(artists::Column::Name, "artist_name")
        .join(JoinType::InnerJoin, playlist_tracks::Relation::Tracks.def())
        .join(JoinType::InnerJoin, tracks::Relation::Albums.def())
        .join(JoinType::InnerJoin, albums::Relation::Artists.def())
        .order_by_asc(playlist_tracks::Column::Position)
        .offset(offset)
        .limit(limit)
        .into_model::<TrackRow>()
        .all(db)
        .await?;

    let details: Vec<PlaylistTrackDetails> = tracks
        .into_iter()
        .map(|t| PlaylistTrackDetails {
            id: t.id,
            position: t.position,
            track_name: t.track_name,
            duration_ms: t.duration_ms,
            album_id: t.album_id,
            album_name: t.album_name,
            ownership_status: t.ownership_status,
            artist_name: t.artist_name,
        })
        .collect();

    Ok((details, total))
}
