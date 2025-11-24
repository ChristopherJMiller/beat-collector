use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, ColumnTrait, Set};
use crate::error::Result;
use crate::db::entities::{albums, artists, tracks, user_settings, jobs};

pub struct AlbumRepository {
    db: DatabaseConnection,
}

impl AlbumRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn find_by_id(&self, id: i32) -> Result<Option<albums::Model>> {
        Ok(albums::Entity::find_by_id(id).one(&self.db).await?)
    }

    pub async fn find_by_spotify_id(&self, spotify_id: &str) -> Result<Option<albums::Model>> {
        Ok(albums::Entity::find()
            .filter(albums::Column::SpotifyId.eq(spotify_id))
            .one(&self.db)
            .await?)
    }

    pub async fn create(&self, album: albums::ActiveModel) -> Result<albums::Model> {
        Ok(album.insert(&self.db).await?)
    }

    pub async fn update(&self, album: albums::ActiveModel) -> Result<albums::Model> {
        Ok(album.update(&self.db).await?)
    }
}

pub struct ArtistRepository {
    db: DatabaseConnection,
}

impl ArtistRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn find_by_id(&self, id: i32) -> Result<Option<artists::Model>> {
        Ok(artists::Entity::find_by_id(id).one(&self.db).await?)
    }

    pub async fn find_by_spotify_id(&self, spotify_id: &str) -> Result<Option<artists::Model>> {
        Ok(artists::Entity::find()
            .filter(artists::Column::SpotifyId.eq(spotify_id))
            .one(&self.db)
            .await?)
    }

    pub async fn create(&self, artist: artists::ActiveModel) -> Result<artists::Model> {
        Ok(artist.insert(&self.db).await?)
    }
}

pub struct UserSettingsRepository {
    db: DatabaseConnection,
}

impl UserSettingsRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn get_settings(&self) -> Result<Option<user_settings::Model>> {
        Ok(user_settings::Entity::find().one(&self.db).await?)
    }

    pub async fn create_or_update(&self, settings: user_settings::ActiveModel) -> Result<user_settings::Model> {
        // Check if settings exist
        if let Some(existing) = user_settings::Entity::find().one(&self.db).await? {
            let mut active: user_settings::ActiveModel = existing.into();
            // Update fields from new settings
            if let Set(val) = settings.spotify_access_token {
                active.spotify_access_token = Set(val);
            }
            if let Set(val) = settings.spotify_refresh_token {
                active.spotify_refresh_token = Set(val);
            }
            if let Set(val) = settings.spotify_token_expires_at {
                active.spotify_token_expires_at = Set(val);
            }
            if let Set(val) = settings.lidarr_url {
                active.lidarr_url = Set(val);
            }
            if let Set(val) = settings.lidarr_api_key {
                active.lidarr_api_key = Set(val);
            }
            if let Set(val) = settings.music_folder_path {
                active.music_folder_path = Set(val);
            }
            if let Set(val) = settings.auto_sync_enabled {
                active.auto_sync_enabled = Set(val);
            }
            if let Set(val) = settings.sync_interval_hours {
                active.sync_interval_hours = Set(val);
            }
            Ok(active.update(&self.db).await?)
        } else {
            Ok(settings.insert(&self.db).await?)
        }
    }
}

pub struct JobRepository {
    db: DatabaseConnection,
}

impl JobRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn create(&self, job: jobs::ActiveModel) -> Result<jobs::Model> {
        Ok(job.insert(&self.db).await?)
    }

    pub async fn update(&self, job: jobs::ActiveModel) -> Result<jobs::Model> {
        Ok(job.update(&self.db).await?)
    }

    pub async fn find_by_id(&self, id: i32) -> Result<Option<jobs::Model>> {
        Ok(jobs::Entity::find_by_id(id).one(&self.db).await?)
    }

    pub async fn find_recent(&self, limit: u64) -> Result<Vec<jobs::Model>> {
        Ok(jobs::Entity::find()
            .order_by_desc(jobs::Column::CreatedAt)
            .limit(limit)
            .all(&self.db)
            .await?)
    }
}
