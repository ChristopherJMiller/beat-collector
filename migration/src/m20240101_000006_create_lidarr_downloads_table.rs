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
                    .table(LidarrDownloads::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(LidarrDownloads::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()"),
                    )
                    .col(
                        ColumnDef::new(LidarrDownloads::AlbumId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(LidarrDownloads::LidarrAlbumId)
                            .integer(),
                    )
                    .col(
                        ColumnDef::new(LidarrDownloads::DownloadId)
                            .string_len(100),
                    )
                    .col(
                        ColumnDef::new(LidarrDownloads::Status)
                            .string_len(20)
                            .not_null()
                            .default("pending"),
                    )
                    .col(
                        ColumnDef::new(LidarrDownloads::QualityProfile)
                            .string_len(50),
                    )
                    .col(
                        ColumnDef::new(LidarrDownloads::EstimatedCompletionAt)
                            .timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(LidarrDownloads::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT NOW()"),
                    )
                    .col(
                        ColumnDef::new(LidarrDownloads::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT NOW()"),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_lidarr_downloads_album_id")
                            .from(LidarrDownloads::Table, LidarrDownloads::AlbumId)
                            .to(Albums::Table, Albums::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_lidarr_downloads_album_id")
                    .table(LidarrDownloads::Table)
                    .col(LidarrDownloads::AlbumId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_lidarr_downloads_status")
                    .table(LidarrDownloads::Table)
                    .col(LidarrDownloads::Status)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(LidarrDownloads::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum LidarrDownloads {
    Table,
    Id,
    AlbumId,
    LidarrAlbumId,
    DownloadId,
    Status,
    QualityProfile,
    EstimatedCompletionAt,
    CreatedAt,
    UpdatedAt,
}
