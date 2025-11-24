use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Artists::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Artists::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Artists::Name)
                            .string_len(500)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Artists::SpotifyId)
                            .string_len(100)
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(Artists::MusicbrainzId)
                            .string_len(100),
                    )
                    .col(
                        ColumnDef::new(Artists::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Artists::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_artists_spotify_id")
                    .table(Artists::Table)
                    .col(Artists::SpotifyId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_artists_musicbrainz_id")
                    .table(Artists::Table)
                    .col(Artists::MusicbrainzId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Artists::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum Artists {
    Table,
    Id,
    Name,
    SpotifyId,
    MusicbrainzId,
    CreatedAt,
    UpdatedAt,
}
