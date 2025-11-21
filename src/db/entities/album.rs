use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "albums")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub title: String,
    pub artist_id: Uuid,
    pub spotify_id: Option<String>,
    pub musicbrainz_release_group_id: Option<Uuid>,
    pub release_date: Option<Date>,
    pub total_tracks: Option<i32>,
    pub cover_art_url: Option<String>,
    #[sea_orm(column_type = "Array(ColumnType::Text)")]
    pub genres: Option<Vec<String>>,
    pub ownership_status: OwnershipStatus,
    pub acquisition_source: Option<AcquisitionSource>,
    pub local_path: Option<String>,
    pub match_score: Option<i32>,
    pub match_status: MatchStatus,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    pub last_synced_at: Option<DateTimeWithTimeZone>,
}

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(Some(20))")]
pub enum OwnershipStatus {
    #[sea_orm(string_value = "not_owned")]
    NotOwned,
    #[sea_orm(string_value = "owned")]
    Owned,
    #[sea_orm(string_value = "downloading")]
    Downloading,
}

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(Some(20))")]
pub enum AcquisitionSource {
    #[sea_orm(string_value = "bandcamp")]
    Bandcamp,
    #[sea_orm(string_value = "physical")]
    Physical,
    #[sea_orm(string_value = "lidarr")]
    Lidarr,
    #[sea_orm(string_value = "unknown")]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(Some(20))")]
pub enum MatchStatus {
    #[sea_orm(string_value = "pending")]
    Pending,
    #[sea_orm(string_value = "matched")]
    Matched,
    #[sea_orm(string_value = "manual_review")]
    ManualReview,
    #[sea_orm(string_value = "no_match")]
    NoMatch,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::artist::Entity",
        from = "Column::ArtistId",
        to = "super::artist::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Artist,
    #[sea_orm(has_many = "super::track::Entity")]
    Tracks,
    #[sea_orm(has_many = "super::lidarr_download::Entity")]
    LidarrDownloads,
}

impl Related<super::artist::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Artist.def()
    }
}

impl Related<super::track::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Tracks.def()
    }
}

impl Related<super::lidarr_download::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::LidarrDownloads.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
