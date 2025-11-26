use sea_orm_migration::prelude::*;

use super::m20240101_000003_create_tracks_table::Tracks;
use super::m20240101_000007_create_playlists_table::Playlists;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(PlaylistTracks::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PlaylistTracks::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(PlaylistTracks::PlaylistId)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PlaylistTracks::TrackId)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PlaylistTracks::Position)
                            .integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(PlaylistTracks::AddedAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(PlaylistTracks::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PlaylistTracks::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_playlist_tracks_playlist_id")
                            .from(PlaylistTracks::Table, PlaylistTracks::PlaylistId)
                            .to(Playlists::Table, Playlists::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_playlist_tracks_track_id")
                            .from(PlaylistTracks::Table, PlaylistTracks::TrackId)
                            .to(Tracks::Table, Tracks::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_playlist_tracks_playlist_id")
                    .table(PlaylistTracks::Table)
                    .col(PlaylistTracks::PlaylistId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_playlist_tracks_track_id")
                    .table(PlaylistTracks::Table)
                    .col(PlaylistTracks::TrackId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_playlist_tracks_unique")
                    .table(PlaylistTracks::Table)
                    .col(PlaylistTracks::PlaylistId)
                    .col(PlaylistTracks::TrackId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(PlaylistTracks::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum PlaylistTracks {
    Table,
    Id,
    PlaylistId,
    TrackId,
    Position,
    AddedAt,
    CreatedAt,
    UpdatedAt,
}
