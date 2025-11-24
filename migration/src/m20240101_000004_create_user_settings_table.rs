use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(UserSettings::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(UserSettings::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key()
                    )
                    .col(
                        ColumnDef::new(UserSettings::SpotifyAccessToken)
                            .text(),
                    )
                    .col(
                        ColumnDef::new(UserSettings::SpotifyRefreshToken)
                            .text(),
                    )
                    .col(
                        ColumnDef::new(UserSettings::SpotifyTokenExpiresAt)
                            .timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(UserSettings::LidarrUrl)
                            .string_len(500),
                    )
                    .col(
                        ColumnDef::new(UserSettings::LidarrApiKey)
                            .string_len(100),
                    )
                    .col(
                        ColumnDef::new(UserSettings::MusicFolderPath)
                            .text(),
                    )
                    .col(
                        ColumnDef::new(UserSettings::AutoSyncEnabled)
                            .boolean()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(UserSettings::SyncIntervalHours)
                            .integer()
                            .default(24),
                    )
                    .col(
                        ColumnDef::new(UserSettings::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                    )
                    .col(
                        ColumnDef::new(UserSettings::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(UserSettings::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum UserSettings {
    Table,
    Id,
    SpotifyAccessToken,
    SpotifyRefreshToken,
    SpotifyTokenExpiresAt,
    LidarrUrl,
    LidarrApiKey,
    MusicFolderPath,
    AutoSyncEnabled,
    SyncIntervalHours,
    CreatedAt,
    UpdatedAt,
}
