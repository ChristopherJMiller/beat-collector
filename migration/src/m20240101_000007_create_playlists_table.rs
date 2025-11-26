use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Playlists::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Playlists::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Playlists::Name)
                            .string_len(500)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Playlists::SpotifyId)
                            .string_len(100)
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Playlists::Description).text())
                    .col(ColumnDef::new(Playlists::OwnerName).string_len(255))
                    .col(
                        ColumnDef::new(Playlists::IsCollaborative)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(ColumnDef::new(Playlists::TotalTracks).integer())
                    .col(ColumnDef::new(Playlists::CoverImageUrl).text())
                    .col(ColumnDef::new(Playlists::SnapshotId).string_len(100))
                    .col(
                        ColumnDef::new(Playlists::IsEnabled)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(Playlists::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Playlists::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Playlists::LastSyncedAt).timestamp_with_time_zone())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_playlists_spotify_id")
                    .table(Playlists::Table)
                    .col(Playlists::SpotifyId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_playlists_is_enabled")
                    .table(Playlists::Table)
                    .col(Playlists::IsEnabled)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Playlists::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum Playlists {
    Table,
    Id,
    Name,
    SpotifyId,
    Description,
    OwnerName,
    IsCollaborative,
    TotalTracks,
    CoverImageUrl,
    SnapshotId,
    IsEnabled,
    CreatedAt,
    UpdatedAt,
    LastSyncedAt,
}
