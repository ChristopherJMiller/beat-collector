use sea_orm_migration::prelude::*;

use super::m20240101_000002_create_albums_table::Albums;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Tracks::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Tracks::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()"),
                    )
                    .col(
                        ColumnDef::new(Tracks::AlbumId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Tracks::Title)
                            .string_len(500)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Tracks::TrackNumber)
                            .integer(),
                    )
                    .col(
                        ColumnDef::new(Tracks::DiscNumber)
                            .integer()
                            .default(1),
                    )
                    .col(
                        ColumnDef::new(Tracks::DurationMs)
                            .integer(),
                    )
                    .col(
                        ColumnDef::new(Tracks::SpotifyId)
                            .string_len(100),
                    )
                    .col(
                        ColumnDef::new(Tracks::MusicbrainzId)
                            .uuid(),
                    )
                    .col(
                        ColumnDef::new(Tracks::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT NOW()"),
                    )
                    .col(
                        ColumnDef::new(Tracks::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT NOW()"),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_tracks_album_id")
                            .from(Tracks::Table, Tracks::AlbumId)
                            .to(Albums::Table, Albums::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_tracks_album_id")
                    .table(Tracks::Table)
                    .col(Tracks::AlbumId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_tracks_spotify_id")
                    .table(Tracks::Table)
                    .col(Tracks::SpotifyId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Tracks::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum Tracks {
    Table,
    Id,
    AlbumId,
    Title,
    TrackNumber,
    DiscNumber,
    DurationMs,
    SpotifyId,
    MusicbrainzId,
    CreatedAt,
    UpdatedAt,
}
