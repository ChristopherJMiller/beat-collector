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
}

#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub refresh_token: Option<String>,
    pub scope: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SpotifyAlbum {
    pub id: String,
    pub name: String,
    pub artists: Vec<SpotifyArtist>,
    pub release_date: String,
    pub total_tracks: i32,
    pub images: Vec<SpotifyImage>,
    pub genres: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SpotifyArtist {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
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

        // Build authorization URL
        let scopes = vec![
            "user-library-read",
            "playlist-read-private",
            "playlist-read-collaborative",
        ];

        let url = format!(
            "{}?client_id={}&response_type=code&redirect_uri={}&code_challenge_method=S256&code_challenge={}&scope={}",
            SPOTIFY_AUTH_URL,
            urlencoding::encode(&self.client_id),
            urlencoding::encode(&self.redirect_uri),
            code_challenge,
            urlencoding::encode(&scopes.join(" "))
        );

        Ok(AuthorizationUrl {
            url,
            code_verifier,
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
