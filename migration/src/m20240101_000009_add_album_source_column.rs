use sea_orm_migration::prelude::*;

use super::m20240101_000002_create_albums_table::Albums;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Albums::Table)
                    .add_column(
                        ColumnDef::new(AlbumsAdditions::Source)
                            .string_len(20)
                            .not_null()
                            .default("saved_album"),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_albums_source")
                    .table(Albums::Table)
                    .col(AlbumsAdditions::Source)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Albums::Table)
                    .drop_column(AlbumsAdditions::Source)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum AlbumsAdditions {
    Source,
}
