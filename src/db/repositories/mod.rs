use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, ColumnTrait, Set};
use uuid::Uuid;
use crate::error::Result;
use crate::db::entities::{album, artist, track, user_settings, job};

pub struct AlbumRepository {
    db: DatabaseConnection,
}

impl AlbumRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<album::Model>> {
        Ok(album::Entity::find_by_id(id).one(&self.db).await?)
    }

    pub async fn find_by_spotify_id(&self, spotify_id: &str) -> Result<Option<album::Model>> {
        Ok(album::Entity::find()
            .filter(album::Column::SpotifyId.eq(spotify_id))
            .one(&self.db)
            .await?)
    }

    pub async fn create(&self, album: album::ActiveModel) -> Result<album::Model> {
        Ok(album.insert(&self.db).await?)
    }

    pub async fn update(&self, album: album::ActiveModel) -> Result<album::Model> {
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

    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<artist::Model>> {
        Ok(artist::Entity::find_by_id(id).one(&self.db).await?)
    }

    pub async fn find_by_spotify_id(&self, spotify_id: &str) -> Result<Option<artist::Model>> {
        Ok(artist::Entity::find()
            .filter(artist::Column::SpotifyId.eq(spotify_id))
            .one(&self.db)
            .await?)
    }

    pub async fn create(&self, artist: artist::ActiveModel) -> Result<artist::Model> {
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

    pub async fn create(&self, job: job::ActiveModel) -> Result<job::Model> {
        Ok(job.insert(&self.db).await?)
    }

    pub async fn update(&self, job: job::ActiveModel) -> Result<job::Model> {
        Ok(job.update(&self.db).await?)
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<job::Model>> {
        Ok(job::Entity::find_by_id(id).one(&self.db).await?)
    }

    pub async fn find_recent(&self, limit: u64) -> Result<Vec<job::Model>> {
        Ok(job::Entity::find()
            .order_by_desc(job::Column::CreatedAt)
            .limit(limit)
            .all(&self.db)
            .await?)
    }
}
