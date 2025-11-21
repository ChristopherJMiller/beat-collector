use anyhow::Context;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::{sleep, Instant};
use uuid::Uuid;

use crate::error::{AppError, Result};

const MUSICBRAINZ_API_BASE: &str = "https://musicbrainz.org/ws/2";
const COVER_ART_ARCHIVE_BASE: &str = "https://coverartarchive.org";
const RATE_LIMIT_DELAY: Duration = Duration::from_secs(1); // 1 request per second

#[derive(Clone)]
pub struct MusicBrainzService {
    client: Client,
    last_request: Arc<Mutex<Option<Instant>>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MusicBrainzMatch {
    pub id: Uuid,
    pub title: String,
    pub artist_credit: Vec<ArtistCredit>,
    pub score: i32,
    pub first_release_date: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ArtistCredit {
    pub name: String,
    pub artist: Artist,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Artist {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    #[serde(rename = "release-groups")]
    release_groups: Vec<ReleaseGroup>,
}

#[derive(Debug, Deserialize)]
struct ReleaseGroup {
    id: Uuid,
    title: String,
    #[serde(rename = "artist-credit")]
    artist_credit: Vec<ArtistCredit>,
    score: i32,
    #[serde(rename = "first-release-date")]
    first_release_date: Option<String>,
}

impl MusicBrainzService {
    pub fn new(user_agent: String) -> Self {
        let client = Client::builder()
            .user_agent(user_agent)
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            last_request: Arc::new(Mutex::new(None)),
        }
    }

    /// Search for release group (album) matching artist and title
    pub async fn search_release_group(
        &self,
        artist: &str,
        album: &str,
    ) -> Result<Vec<MusicBrainzMatch>> {
        // Enforce rate limiting
        self.wait_for_rate_limit().await;

        // Try exact match first
        let query = format!(
            "artist:\"{}\" AND releasegroup:\"{}\" AND primarytype:album",
            self.normalize_artist(artist),
            album
        );

        tracing::debug!("MusicBrainz exact search query: {}", query);

        let mut matches = self.execute_search(&query).await?;

        // If no high-confidence matches, try fuzzy search
        if matches.is_empty() || matches.iter().all(|m| m.score < 80) {
            let fuzzy_query = format!(
                "artist:{}~ AND releasegroup:{}~ AND primarytype:album",
                self.normalize_artist(artist),
                album
            );

            tracing::debug!("MusicBrainz fuzzy search query: {}", fuzzy_query);
            self.wait_for_rate_limit().await;
            matches = self.execute_search(&fuzzy_query).await?;
        }

        // Filter and sort by score
        let mut filtered: Vec<MusicBrainzMatch> = matches
            .into_iter()
            .filter(|m| m.score >= 80)
            .collect();

        filtered.sort_by(|a, b| b.score.cmp(&a.score));

        Ok(filtered)
    }

    /// Fetch cover art for a release group
    pub async fn fetch_cover_art(&self, mbid: Uuid, size: CoverArtSize) -> Result<Vec<u8>> {
        let url = match size {
            CoverArtSize::Small => format!("{}/release-group/{}/front-250", COVER_ART_ARCHIVE_BASE, mbid),
            CoverArtSize::Medium => format!("{}/release-group/{}/front-500", COVER_ART_ARCHIVE_BASE, mbid),
            CoverArtSize::Large => format!("{}/release-group/{}/front-1200", COVER_ART_ARCHIVE_BASE, mbid),
        };

        // Note: Cover Art Archive has no rate limit, but we'll be respectful
        sleep(Duration::from_millis(100)).await;

        let response = self.client.get(&url).send().await?;

        if response.status().is_success() {
            Ok(response.bytes().await?.to_vec())
        } else if response.status().as_u16() == 404 {
            Err(AppError::NotFound("Cover art not found".to_string()))
        } else {
            Err(AppError::ExternalApi(format!(
                "Failed to fetch cover art: {}",
                response.status()
            )))
        }
    }

    /// Execute search query against MusicBrainz API
    async fn execute_search(&self, query: &str) -> Result<Vec<MusicBrainzMatch>> {
        let url = format!(
            "{}/release-group?query={}&fmt=json&limit=10",
            MUSICBRAINZ_API_BASE,
            urlencoding::encode(query)
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            let status = response.status();

            // Handle 503 rate limit error
            if status.as_u16() == 503 {
                tracing::warn!("MusicBrainz rate limit hit, backing off");
                sleep(Duration::from_secs(2)).await;
                return Err(AppError::ExternalApi(
                    "MusicBrainz rate limit exceeded".to_string(),
                ));
            }

            let error_text = response.text().await?;
            return Err(AppError::ExternalApi(format!(
                "MusicBrainz API error ({}): {}",
                status, error_text
            )));
        }

        let data: SearchResponse = response.json().await?;

        Ok(data
            .release_groups
            .into_iter()
            .map(|rg| MusicBrainzMatch {
                id: rg.id,
                title: rg.title,
                artist_credit: rg.artist_credit,
                score: rg.score,
                first_release_date: rg.first_release_date,
            })
            .collect())
    }

    /// Enforce 1 request per second rate limit
    async fn wait_for_rate_limit(&self) {
        let mut last_request = self.last_request.lock().await;

        if let Some(last) = *last_request {
            let elapsed = last.elapsed();
            if elapsed < RATE_LIMIT_DELAY {
                let wait_time = RATE_LIMIT_DELAY - elapsed;
                tracing::debug!("Rate limiting: waiting {:?}", wait_time);
                sleep(wait_time).await;
            }
        }

        *last_request = Some(Instant::now());
    }

    /// Normalize artist name for better matching
    fn normalize_artist(&self, artist: &str) -> String {
        let mut normalized = artist.to_string();

        // Remove featuring artists
        if let Some(pos) = normalized.find(" feat.") {
            normalized.truncate(pos);
        }
        if let Some(pos) = normalized.find(" ft.") {
            normalized.truncate(pos);
        }

        // Handle "The" prefix - try both with and without
        // For now, just return as-is and let fuzzy matching handle it
        normalized.trim().to_string()
    }
}

#[derive(Debug, Clone, Copy)]
pub enum CoverArtSize {
    Small,  // 250px
    Medium, // 500px
    Large,  // 1200px
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_artist() {
        let service = MusicBrainzService::new("Test/1.0".to_string());

        assert_eq!(
            service.normalize_artist("Artist feat. Someone"),
            "Artist"
        );

        assert_eq!(
            service.normalize_artist("Artist ft. Someone"),
            "Artist"
        );
    }
}
