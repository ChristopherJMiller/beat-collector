use sea_orm_migration::prelude::*;

use super::m20240101_000001_create_artists_table::Artists;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Albums::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Albums::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Albums::Title)
                            .string_len(500)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Albums::ArtistId)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Albums::SpotifyId)
                            .string_len(100)
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(Albums::MusicbrainzReleaseGroupId)
                            .string_len(100),
                    )
                    .col(
                        ColumnDef::new(Albums::ReleaseDate)
                            .date(),
                    )
                    .col(
                        ColumnDef::new(Albums::TotalTracks)
                            .integer(),
                    )
                    .col(
                        ColumnDef::new(Albums::CoverArtUrl)
                            .text(),
                    )
                    .col(
                        ColumnDef::new(Albums::Genres)
                            .text(),
                    )
                    .col(
                        ColumnDef::new(Albums::OwnershipStatus)
                            .string_len(20)
                            .not_null()
                            .default("not_owned"),
                    )
                    .col(
                        ColumnDef::new(Albums::AcquisitionSource)
                            .string_len(20),
                    )
                    .col(
                        ColumnDef::new(Albums::LocalPath)
                            .text(),
                    )
                    .col(
                        ColumnDef::new(Albums::MatchScore)
                            .integer(),
                    )
                    .col(
                        ColumnDef::new(Albums::MatchStatus)
                            .string_len(20)
                            .default("pending"),
                    )
                    .col(
                        ColumnDef::new(Albums::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Albums::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Albums::LastSyncedAt)
                            .timestamp_with_time_zone(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_albums_artist_id")
                            .from(Albums::Table, Albums::ArtistId)
                            .to(Artists::Table, Artists::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create indexes
        manager
            .create_index(
                Index::create()
                    .name("idx_albums_artist_id")
                    .table(Albums::Table)
                    .col(Albums::ArtistId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_albums_spotify_id")
                    .table(Albums::Table)
                    .col(Albums::SpotifyId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_albums_musicbrainz_id")
                    .table(Albums::Table)
                    .col(Albums::MusicbrainzReleaseGroupId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_albums_ownership_status")
                    .table(Albums::Table)
                    .col(Albums::OwnershipStatus)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_albums_match_status")
                    .table(Albums::Table)
                    .col(Albums::MatchStatus)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Albums::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum Albums {
    Table,
    Id,
    Title,
    ArtistId,
    SpotifyId,
    MusicbrainzReleaseGroupId,
    ReleaseDate,
    TotalTracks,
    CoverArtUrl,
    Genres,
    OwnershipStatus,
    AcquisitionSource,
    LocalPath,
    MatchScore,
    MatchStatus,
    CreatedAt,
    UpdatedAt,
    LastSyncedAt,
}
