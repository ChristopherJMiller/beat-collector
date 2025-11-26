use sea_orm_migration::prelude::*;

use super::m20240101_000007_create_playlists_table::Playlists;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Playlists::Table)
                    .add_column(
                        ColumnDef::new(PlaylistsAdditions::IsSynthetic)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_playlists_is_synthetic")
                    .table(Playlists::Table)
                    .col(PlaylistsAdditions::IsSynthetic)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(Index::drop().name("idx_playlists_is_synthetic").to_owned())
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Playlists::Table)
                    .drop_column(PlaylistsAdditions::IsSynthetic)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum PlaylistsAdditions {
    IsSynthetic,
}
