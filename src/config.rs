use anyhow::{Context, Result};
use serde::Deserialize;
use std::env;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub database_url: String,
    pub redis_url: String,
    pub server_host: String,
    pub server_port: u16,
    pub spotify_client_id: String,
    pub spotify_redirect_uri: String,
    pub music_folder_path: Option<String>,
    pub lidarr_url: Option<String>,
    pub lidarr_api_key: Option<String>,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            database_url: env::var("DATABASE_URL")
                .context("DATABASE_URL must be set")?,
            redis_url: env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string()),
            server_host: env::var("SERVER_HOST")
                .unwrap_or_else(|_| "0.0.0.0".to_string()),
            server_port: env::var("SERVER_PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()
                .context("SERVER_PORT must be a valid port number")?,
            spotify_client_id: env::var("SPOTIFY_CLIENT_ID")
                .context("SPOTIFY_CLIENT_ID must be set")?,
            spotify_redirect_uri: env::var("SPOTIFY_REDIRECT_URI")
                .context("SPOTIFY_REDIRECT_URI must be set")?,
            music_folder_path: env::var("MUSIC_FOLDER").ok(),
            lidarr_url: env::var("LIDARR_URL").ok(),
            lidarr_api_key: env::var("LIDARR_API_KEY").ok(),
        })
    }
}
