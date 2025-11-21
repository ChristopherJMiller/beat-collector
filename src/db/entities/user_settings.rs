use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "user_settings")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    #[serde(skip_serializing)]
    pub spotify_access_token: Option<String>,
    #[serde(skip_serializing)]
    pub spotify_refresh_token: Option<String>,
    pub spotify_token_expires_at: Option<DateTimeWithTimeZone>,
    pub lidarr_url: Option<String>,
    #[serde(skip_serializing)]
    pub lidarr_api_key: Option<String>,
    pub music_folder_path: Option<String>,
    pub auto_sync_enabled: Option<bool>,
    pub sync_interval_hours: Option<i32>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
