use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::error::{AppError, Result};

const API_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone)]
pub struct LidarrService {
    client: Client,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LidarrAlbum {
    pub id: i32,
    pub title: String,
    pub artist: LidarrArtist,
    pub release_date: Option<String>,
    pub monitored: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LidarrArtist {
    pub id: i32,
    pub artist_name: String,
    pub foreign_artist_id: String, // MusicBrainz ID
}

#[derive(Debug, Serialize)]
pub struct SearchAlbumCommand {
    pub name: String,
    #[serde(rename = "albumIds")]
    pub album_ids: Vec<i32>,
}

#[derive(Debug, Deserialize)]
pub struct CommandResponse {
    pub id: i32,
    pub name: String,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct QueueItem {
    pub id: i32,
    pub album: LidarrAlbum,
    pub status: String,
    pub download_id: Option<String>,
    #[serde(rename = "estimatedCompletionTime")]
    pub estimated_completion_time: Option<String>,
    pub size: Option<f64>,
    pub sizeleft: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "eventType")]
pub enum LidarrWebhook {
    #[serde(rename = "Grab")]
    Grab {
        artist: LidarrArtist,
        albums: Vec<LidarrAlbum>,
        download_id: String,
    },
    #[serde(rename = "Download")]
    Download {
        artist: LidarrArtist,
        albums: Vec<LidarrAlbum>,
        #[serde(rename = "trackFiles")]
        track_files: Vec<TrackFile>,
        #[serde(rename = "isUpgrade")]
        is_upgrade: bool,
    },
    #[serde(rename = "AlbumDownload")]
    AlbumDownload {
        artist: LidarrArtist,
        album: LidarrAlbum,
    },
    #[serde(rename = "DownloadFailure")]
    DownloadFailure {
        artist: LidarrArtist,
        albums: Vec<LidarrAlbum>,
        message: String,
    },
}

#[derive(Debug, Deserialize)]
pub struct TrackFile {
    pub id: i32,
    pub path: String,
    pub quality: Quality,
}

#[derive(Debug, Deserialize)]
pub struct Quality {
    pub quality: QualityDefinition,
}

#[derive(Debug, Deserialize)]
pub struct QualityDefinition {
    pub name: String,
}

impl LidarrService {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(API_TIMEOUT)
            .build()
            .expect("Failed to build HTTP client");

        Self { client }
    }

    /// Test connection to Lidarr instance
    pub async fn test_connection(&self, base_url: &str, api_key: &str) -> Result<bool> {
        let url = format!("{}/api/v1/system/status", base_url.trim_end_matches('/'));

        let response = self
            .client
            .get(&url)
            .header("X-Api-Key", api_key)
            .send()
            .await?;

        Ok(response.status().is_success())
    }

    /// Search for an album in Lidarr
    pub async fn search_album(
        &self,
        base_url: &str,
        api_key: &str,
        album_id: i32,
    ) -> Result<CommandResponse> {
        let url = format!("{}/api/v1/command", base_url.trim_end_matches('/'));

        let command = SearchAlbumCommand {
            name: "AlbumSearch".to_string(),
            album_ids: vec![album_id],
        };

        let response = self
            .client
            .post(&url)
            .header("X-Api-Key", api_key)
            .json(&command)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;
            return Err(AppError::ExternalApi(format!(
                "Lidarr API error ({}): {}",
                status, error_text
            )));
        }

        Ok(response.json().await?)
    }

    /// Get current download queue
    pub async fn get_queue(&self, base_url: &str, api_key: &str) -> Result<Vec<QueueItem>> {
        let url = format!("{}/api/v1/queue", base_url.trim_end_matches('/'));

        let response = self
            .client
            .get(&url)
            .header("X-Api-Key", api_key)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;
            return Err(AppError::ExternalApi(format!(
                "Lidarr queue fetch error ({}): {}",
                status, error_text
            )));
        }

        Ok(response.json().await?)
    }

    /// Lookup album by MusicBrainz ID
    pub async fn lookup_album(
        &self,
        base_url: &str,
        api_key: &str,
        musicbrainz_id: &str,
    ) -> Result<Option<LidarrAlbum>> {
        let url = format!(
            "{}/api/v1/album/lookup?term=lidarr:{}",
            base_url.trim_end_matches('/'),
            musicbrainz_id
        );

        let response = self
            .client
            .get(&url)
            .header("X-Api-Key", api_key)
            .send()
            .await?;

        if response.status().as_u16() == 404 {
            return Ok(None);
        }

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;
            return Err(AppError::ExternalApi(format!(
                "Lidarr lookup error ({}): {}",
                status, error_text
            )));
        }

        let albums: Vec<LidarrAlbum> = response.json().await?;
        Ok(albums.into_iter().next())
    }

    /// Add album to Lidarr
    pub async fn add_album(
        &self,
        base_url: &str,
        api_key: &str,
        album: &LidarrAlbum,
    ) -> Result<LidarrAlbum> {
        let url = format!("{}/api/v1/album", base_url.trim_end_matches('/'));

        let response = self
            .client
            .post(&url)
            .header("X-Api-Key", api_key)
            .json(album)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;
            return Err(AppError::ExternalApi(format!(
                "Lidarr add album error ({}): {}",
                status, error_text
            )));
        }

        Ok(response.json().await?)
    }
}

impl Default for LidarrService {
    fn default() -> Self {
        Self::new()
    }
}
