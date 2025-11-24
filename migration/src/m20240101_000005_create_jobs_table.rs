use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Jobs::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Jobs::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key()
                    )
                    .col(
                        ColumnDef::new(Jobs::JobType)
                            .string_len(50)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Jobs::Status)
                            .string_len(20)
                            .not_null()
                            .default("pending"),
                    )
                    .col(
                        ColumnDef::new(Jobs::EntityId)
                            .integer(),
                    )
                    .col(
                        ColumnDef::new(Jobs::Progress)
                            .integer()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(Jobs::TotalItems)
                            .integer(),
                    )
                    .col(
                        ColumnDef::new(Jobs::ProcessedItems)
                            .integer()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(Jobs::ErrorMessage)
                            .text(),
                    )
                    .col(
                        ColumnDef::new(Jobs::StartedAt)
                            .timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(Jobs::CompletedAt)
                            .timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(Jobs::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                    )
                    .col(
                        ColumnDef::new(Jobs::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_jobs_status")
                    .table(Jobs::Table)
                    .col(Jobs::Status)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_jobs_type")
                    .table(Jobs::Table)
                    .col(Jobs::JobType)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_jobs_created_at")
                    .table(Jobs::Table)
                    .col(Jobs::CreatedAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Jobs::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum Jobs {
    Table,
    Id,
    JobType,
    Status,
    EntityId,
    Progress,
    TotalItems,
    ProcessedItems,
    ErrorMessage,
    StartedAt,
    CompletedAt,
    CreatedAt,
    UpdatedAt,
}
