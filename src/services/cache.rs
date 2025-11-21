use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use serde::{de::DeserializeOwned, Serialize};
use std::time::Duration;

use crate::error::Result;

const DEFAULT_TTL: usize = 86400; // 24 hours in seconds

pub struct CacheService {
    redis: ConnectionManager,
}

impl CacheService {
    pub fn new(redis: ConnectionManager) -> Self {
        Self { redis }
    }

    /// Get a value from cache
    pub async fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        let mut conn = self.redis.clone();
        let data: Option<String> = conn.get(key).await?;

        match data {
            Some(json) => {
                let value: T = serde_json::from_str(&json)?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    /// Set a value in cache with TTL
    pub async fn set<T: Serialize>(
        &self,
        key: &str,
        value: &T,
        ttl_seconds: Option<usize>,
    ) -> Result<()> {
        let mut conn = self.redis.clone();
        let json = serde_json::to_string(value)?;
        let ttl = ttl_seconds.unwrap_or(DEFAULT_TTL);

        conn.set_ex(key, json, ttl).await?;
        Ok(())
    }

    /// Delete a key from cache
    pub async fn delete(&self, key: &str) -> Result<()> {
        let mut conn = self.redis.clone();
        conn.del(key).await?;
        Ok(())
    }

    /// Check if a key exists
    pub async fn exists(&self, key: &str) -> Result<bool> {
        let mut conn = self.redis.clone();
        let exists: bool = conn.exists(key).await?;
        Ok(exists)
    }

    /// Set a value with no expiration
    pub async fn set_permanent<T: Serialize>(&self, key: &str, value: &T) -> Result<()> {
        let mut conn = self.redis.clone();
        let json = serde_json::to_string(value)?;
        conn.set(key, json).await?;
        Ok(())
    }

    /// Cache key builders for consistent naming
    pub fn musicbrainz_match_key(artist: &str, album: &str) -> String {
        format!("mb:match:{}:{}", artist, album)
    }

    pub fn spotify_album_key(spotify_id: &str) -> String {
        format!("spotify:album:{}", spotify_id)
    }

    pub fn cover_art_key(musicbrainz_id: &str) -> String {
        format!("cover:mb:{}", musicbrainz_id)
    }
}
