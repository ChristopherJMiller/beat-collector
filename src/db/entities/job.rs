use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "jobs")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub job_type: JobType,
    pub status: JobStatus,
    pub entity_id: Option<Uuid>,
    pub progress: Option<i32>,
    pub total_items: Option<i32>,
    pub processed_items: Option<i32>,
    pub error_message: Option<String>,
    pub started_at: Option<DateTimeWithTimeZone>,
    pub completed_at: Option<DateTimeWithTimeZone>,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(50))")]
pub enum JobType {
    #[sea_orm(string_value = "spotify_sync")]
    SpotifySync,
    #[sea_orm(string_value = "musicbrainz_match")]
    MusicbrainzMatch,
    #[sea_orm(string_value = "lidarr_search")]
    LidarrSearch,
    #[sea_orm(string_value = "cover_art_fetch")]
    CoverArtFetch,
    #[sea_orm(string_value = "filesystem_scan")]
    FilesystemScan,
}

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
pub enum JobStatus {
    #[sea_orm(string_value = "pending")]
    Pending,
    #[sea_orm(string_value = "running")]
    Running,
    #[sea_orm(string_value = "completed")]
    Completed,
    #[sea_orm(string_value = "failed")]
    Failed,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
