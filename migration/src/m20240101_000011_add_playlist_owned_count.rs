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
                        ColumnDef::new(PlaylistsAdditions::OwnedCount)
                            .integer()
                            .null()
                            .default(0),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Playlists::Table)
                    .drop_column(PlaylistsAdditions::OwnedCount)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum PlaylistsAdditions {
    OwnedCount,
}
