use anyhow::Context;
use base64::{engine::general_purpose, Engine as _};
use chrono::{DateTime, Duration, Utc};
use governor::{Quota, RateLimiter, clock::DefaultClock, state::InMemoryState, state::direct::NotKeyed};
use nonzero_ext::nonzero;
use rand::Rng;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;

use crate::error::{AppError, Result};

const SPOTIFY_AUTH_URL: &str = "https://accounts.spotify.com/authorize";
const SPOTIFY_TOKEN_URL: &str = "https://accounts.spotify.com/api/token";
const SPOTIFY_API_BASE: &str = "https://api.spotify.com/v1";

#[derive(Clone)]
pub struct SpotifyService {
    client: Client,
    client_id: String,
    redirect_uri: String,
    rate_limiter: Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthorizationUrl {
    pub url: String,
    pub code_verifier: String,
    pub state: String,
}

#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub refresh_token: Option<String>,
    pub scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotifyAlbum {
    pub id: String,
    pub name: String,
    pub artists: Vec<SpotifyArtist>,
    pub release_date: String,
    pub total_tracks: i32,
    pub images: Vec<SpotifyImage>,
    pub genres: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotifyArtist {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotifyImage {
    pub url: String,
    pub height: Option<i32>,
    pub width: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct SavedAlbumsResponse {
    items: Vec<SavedAlbumItem>,
    next: Option<String>,
    total: i32,
}

#[derive(Debug, Deserialize)]
struct SavedAlbumItem {
    album: SpotifyAlbum,
}

// Playlist-related types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotifyPlaylist {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub owner: SpotifyPlaylistOwner,
    pub collaborative: bool,
    pub tracks: SpotifyPlaylistTracksRef,
    #[serde(default, deserialize_with = "deserialize_null_as_empty_vec")]
    pub images: Vec<SpotifyImage>,
    pub snapshot_id: String,
}

/// Deserialize null or missing as empty vec
fn deserialize_null_as_empty_vec<'de, D, T>(deserializer: D) -> std::result::Result<Vec<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    let opt: Option<Vec<T>> = Option::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotifyPlaylistOwner {
    pub id: String,
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotifyPlaylistTracksRef {
    pub total: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotifyPlaylistTrack {
    pub track: Option<SpotifyTrack>,
    pub added_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotifyTrack {
    pub id: Option<String>,
    pub name: String,
    pub track_number: i32,
    pub disc_number: i32,
    pub duration_ms: i32,
    pub album: SpotifyAlbum,
    pub artists: Vec<SpotifyArtist>,
}

#[derive(Debug, Deserialize)]
struct PlaylistsResponse {
    items: Vec<SpotifyPlaylist>,
    next: Option<String>,
    total: i32,
}

#[derive(Debug, Deserialize)]
struct PlaylistTracksResponse {
    items: Vec<SpotifyPlaylistTrack>,
    next: Option<String>,
    total: i32,
}

impl SpotifyService {
    pub fn new(client_id: String, redirect_uri: String) -> Self {
        // Rate limiter: 2 requests per second to stay under Spotify's ~3 req/sec limit
        let quota = Quota::per_second(nonzero!(2u32));
        let rate_limiter = Arc::new(RateLimiter::direct(quota));

        Self {
            client: Client::new(),
            client_id,
            redirect_uri,
            rate_limiter,
        }
    }

    /// Generate authorization URL with PKCE
    pub fn generate_authorization_url(&self) -> Result<AuthorizationUrl> {
        // Generate code verifier (43-128 characters)
        let code_verifier = self.generate_code_verifier();

        // Generate code challenge
        let code_challenge = self.generate_code_challenge(&code_verifier);

        // Generate random state for CSRF protection and verifier lookup
        let state = uuid::Uuid::new_v4().to_string();

        // Build authorization URL
        let scopes = vec![
            "user-library-read",
            "playlist-read-private",
            "playlist-read-collaborative",
        ];

        let url = format!(
            "{}?client_id={}&response_type=code&redirect_uri={}&code_challenge_method=S256&code_challenge={}&scope={}&state={}",
            SPOTIFY_AUTH_URL,
            urlencoding::encode(&self.client_id),
            urlencoding::encode(&self.redirect_uri),
            code_challenge,
            urlencoding::encode(&scopes.join(" ")),
            state
        );

        Ok(AuthorizationUrl {
            url,
            code_verifier,
            state,
        })
    }

    /// Exchange authorization code for access token
    pub async fn exchange_code(
        &self,
        code: &str,
        code_verifier: &str,
    ) -> Result<TokenResponse> {
        self.rate_limiter.until_ready().await;

        let params = [
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", &self.redirect_uri),
            ("client_id", &self.client_id),
            ("code_verifier", code_verifier),
        ];

        let response = self
            .client
            .post(SPOTIFY_TOKEN_URL)
            .form(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(AppError::Authentication(format!(
                "Failed to exchange code: {}",
                error_text
            )));
        }

        Ok(response.json().await?)
    }

    /// Refresh access token
    pub async fn refresh_token(&self, refresh_token: &str) -> Result<TokenResponse> {
        self.rate_limiter.until_ready().await;

        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", &self.client_id),
        ];

        let response = self
            .client
            .post(SPOTIFY_TOKEN_URL)
            .form(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(AppError::Authentication(format!(
                "Failed to refresh token: {}",
                error_text
            )));
        }

        Ok(response.json().await?)
    }

    /// Fetch all saved albums from user's library
    pub async fn fetch_saved_albums(&self, access_token: &str) -> Result<Vec<SpotifyAlbum>> {
        let mut albums = Vec::new();
        let mut next_url = Some(format!("{}/me/albums?limit=50", SPOTIFY_API_BASE));

        while let Some(url) = next_url {
            self.rate_limiter.until_ready().await;

            let response = self
                .client
                .get(&url)
                .header("Authorization", format!("Bearer {}", access_token))
                .send()
                .await?;

            if !response.status().is_success() {
                let status = response.status();
                let error_text = response.text().await?;
                return Err(AppError::ExternalApi(format!(
                    "Spotify API error ({}): {}",
                    status, error_text
                )));
            }

            let mut data: SavedAlbumsResponse = response.json().await?;
            albums.append(&mut data.items.into_iter().map(|item| item.album).collect());
            next_url = data.next;

            tracing::debug!("Fetched {} albums so far", albums.len());
        }

        Ok(albums)
    }

    /// Fetch all user's playlists (owned and followed)
    pub async fn fetch_user_playlists(&self, access_token: &str) -> Result<Vec<SpotifyPlaylist>> {
        let mut playlists = Vec::new();
        let mut next_url = Some(format!("{}/me/playlists?limit=50", SPOTIFY_API_BASE));

        while let Some(url) = next_url {
            self.rate_limiter.until_ready().await;

            let response = self
                .client
                .get(&url)
                .header("Authorization", format!("Bearer {}", access_token))
                .send()
                .await?;

            if !response.status().is_success() {
                let status = response.status();
                let error_text = response.text().await?;
                return Err(AppError::ExternalApi(format!(
                    "Spotify API error ({}): {}",
                    status, error_text
                )));
            }

            // Get raw text first to enable better error messages
            let text = response.text().await?;
            let data: PlaylistsResponse = match serde_json::from_str(&text) {
                Ok(data) => data,
                Err(e) => {
                    // Find the problematic area in the response
                    let col = e.column();
                    let start = col.saturating_sub(100);
                    let end = (col + 100).min(text.len());
                    let context = &text[start..end];
                    tracing::error!(
                        "Failed to parse playlists response at column {}: {}. Context: ...{}...",
                        col, e, context
                    );
                    return Err(AppError::ExternalApi(format!(
                        "Failed to parse Spotify playlists: {} at column {}",
                        e, col
                    )));
                }
            };

            playlists.append(&mut data.items.into_iter().collect());
            next_url = data.next;

            tracing::debug!("Fetched {} playlists so far", playlists.len());
        }

        Ok(playlists)
    }

    /// Fetch all tracks in a specific playlist
    pub async fn fetch_playlist_tracks(
        &self,
        access_token: &str,
        playlist_id: &str,
    ) -> Result<Vec<SpotifyPlaylistTrack>> {
        let mut tracks = Vec::new();
        let mut next_url = Some(format!(
            "{}/playlists/{}/tracks?limit=100",
            SPOTIFY_API_BASE, playlist_id
        ));

        while let Some(url) = next_url {
            self.rate_limiter.until_ready().await;

            let response = self
                .client
                .get(&url)
                .header("Authorization", format!("Bearer {}", access_token))
                .send()
                .await?;

            if !response.status().is_success() {
                let status = response.status();
                let error_text = response.text().await?;
                return Err(AppError::ExternalApi(format!(
                    "Spotify API error ({}): {}",
                    status, error_text
                )));
            }

            let mut data: PlaylistTracksResponse = response.json().await?;
            tracks.append(&mut data.items);
            next_url = data.next;

            tracing::debug!(
                "Fetched {} tracks so far for playlist {}",
                tracks.len(),
                playlist_id
            );
        }

        Ok(tracks)
    }

    /// Fetch all saved tracks from user's library (Liked Songs)
    pub async fn fetch_saved_tracks(&self, access_token: &str) -> Result<Vec<SpotifyPlaylistTrack>> {
        let mut tracks = Vec::new();
        let mut next_url = Some(format!("{}/me/tracks?limit=50", SPOTIFY_API_BASE));

        while let Some(url) = next_url {
            self.rate_limiter.until_ready().await;

            let response = self
                .client
                .get(&url)
                .header("Authorization", format!("Bearer {}", access_token))
                .send()
                .await?;

            if !response.status().is_success() {
                let status = response.status();
                let error_text = response.text().await?;
                return Err(AppError::ExternalApi(format!(
                    "Spotify API error ({}): {}",
                    status, error_text
                )));
            }

            // Reuse PlaylistTracksResponse - the /me/tracks format is compatible
            let mut data: PlaylistTracksResponse = response.json().await?;
            tracks.append(&mut data.items);
            next_url = data.next;

            tracing::debug!("Fetched {} saved tracks so far", tracks.len());
        }

        Ok(tracks)
    }

    /// Get total count of saved tracks (for quick metadata updates)
    pub async fn get_saved_tracks_total(&self, access_token: &str) -> Result<i32> {
        self.rate_limiter.until_ready().await;

        let response = self
            .client
            .get(&format!("{}/me/tracks?limit=1", SPOTIFY_API_BASE))
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;
            return Err(AppError::ExternalApi(format!(
                "Spotify API error ({}): {}",
                status, error_text
            )));
        }

        let data: PlaylistTracksResponse = response.json().await?;
        Ok(data.total)
    }

    /// Generate a random code verifier
    fn generate_code_verifier(&self) -> String {
        let mut rng = rand::thread_rng();
        let random_bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
        general_purpose::URL_SAFE_NO_PAD.encode(random_bytes)
    }

    /// Generate code challenge from verifier using SHA256
    fn generate_code_challenge(&self, verifier: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(verifier.as_bytes());
        let result = hasher.finalize();
        general_purpose::URL_SAFE_NO_PAD.encode(result)
    }

    /// Check if token is expired or about to expire (within 5 minutes)
    pub fn is_token_expired(&self, expires_at: DateTime<Utc>) -> bool {
        Utc::now() + Duration::minutes(5) >= expires_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_verifier_generation() {
        let service = SpotifyService::new(
            "test_client_id".to_string(),
            "http://localhost:3000/callback".to_string(),
        );
        let verifier = service.generate_code_verifier();
        assert!(verifier.len() >= 43 && verifier.len() <= 128);
    }

    #[test]
    fn test_code_challenge_generation() {
        let service = SpotifyService::new(
            "test_client_id".to_string(),
            "http://localhost:3000/callback".to_string(),
        );
        let verifier = "test_verifier_1234567890_abcdefghijklmnop";
        let challenge = service.generate_code_challenge(verifier);
        assert!(!challenge.is_empty());
    }
}
