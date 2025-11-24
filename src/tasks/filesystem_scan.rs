use anyhow::Result;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use std::collections::HashMap;
use std::path::Path;
use std::fs;

use crate::{
    db::{
        entities::{albums, artists},
        enums::{AcquisitionSource, OwnershipStatus},
    },
    state::AppState,
};

pub async fn run_filesystem_scan(state: AppState, music_path: &Path) -> Result<()> {
    tracing::info!("Starting filesystem scan: {:?}", music_path);

    if !music_path.exists() {
        return Err(anyhow::anyhow!("Music path does not exist: {:?}", music_path));
    }

    let mut found_albums = HashMap::new();

    // Walk the directory looking for <Artist>/<Album> structure
    for artist_entry in fs::read_dir(music_path)? {
        let artist_entry = artist_entry?;
        let artist_path = artist_entry.path();

        if !artist_path.is_dir() {
            continue;
        }

        let artist_name = artist_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
            .to_string();

        // Check for album directories under artist
        for album_entry in fs::read_dir(&artist_path)? {
            let album_entry = album_entry?;
            let album_path = album_entry.path();

            if !album_path.is_dir() {
                continue;
            }

            let album_name = album_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown")
                .to_string();

            // Count audio files to validate this is an album
            let audio_count = count_audio_files(&album_path)?;

            if audio_count >= 3 {
                tracing::debug!(
                    "Found album: {} by {} ({} tracks) at {:?}",
                    album_name,
                    artist_name,
                    audio_count,
                    album_path
                );

                found_albums.insert(
                    (artist_name.clone(), album_name.clone()),
                    album_path.to_string_lossy().to_string(),
                );
            }
        }
    }

    tracing::info!("Found {} potential albums in filesystem", found_albums.len());

    // Match found albums to database and update ownership
    for ((artist_name, album_title), local_path) in found_albums {
        match_and_update_album(&state, &artist_name, &album_title, &local_path).await?;
    }

    tracing::info!("Filesystem scan completed");
    Ok(())
}

/// Count audio files in a directory
fn count_audio_files(path: &Path) -> Result<usize> {
    let mut count = 0;
    let audio_extensions = ["mp3", "flac", "m4a", "ogg", "opus", "wav", "aac"];

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension() {
                if let Some(ext_str) = ext.to_str() {
                    if audio_extensions.contains(&ext_str.to_lowercase().as_str()) {
                        count += 1;
                    }
                }
            }
        }
    }

    Ok(count)
}

/// Match found album to database and update ownership status
async fn match_and_update_album(
    state: &AppState,
    artist_name: &str,
    album_title: &str,
    local_path: &str,
) -> Result<()> {
    // Try to find matching album in database by fuzzy matching artist and title
    // First, try to find artist
    let artist_matches = artists::Entity::find()
        .all(&state.db)
        .await?;

    let matching_artist = artist_matches.iter().find(|a| {
        similarity::normalized_levenshtein(&a.name.to_lowercase(), &artist_name.to_lowercase())
            > 0.8
    });

    if let Some(artist) = matching_artist {
        // Find albums by this artist
        let albums = albums::Entity::find()
            .filter(albums::Column::ArtistId.eq(artist.id))
            .all(&state.db)
            .await?;

        let matching_album = albums.iter().find(|alb| {
            similarity::normalized_levenshtein(
                &alb.title.to_lowercase(),
                &album_title.to_lowercase(),
            ) > 0.8
        });

        if let Some(album_model) = matching_album {
            // Update album ownership
            let mut active: albums::ActiveModel = album_model.clone().into();
            active.ownership_status = Set(OwnershipStatus::Owned.as_str().to_string());
            active.local_path = Set(Some(local_path.to_string()));

            // If acquisition source is not set, default to Unknown
            if album_model.acquisition_source.is_none() {
                active.acquisition_source = Set(Some(AcquisitionSource::Unknown.as_str().to_string()));
            }

            active.updated_at = Set(chrono::Utc::now().into());
            active.update(&state.db).await?;

            tracing::info!(
                "Updated album '{}' by '{}' to owned status",
                album_title,
                artist_name
            );
        } else {
            tracing::debug!(
                "No matching album found in database for: {} by {}",
                album_title,
                artist_name
            );
        }
    } else {
        tracing::debug!(
            "No matching artist found in database for: {}",
            artist_name
        );
    }

    Ok(())
}

// Simple string similarity for fuzzy matching
mod similarity {
    pub fn normalized_levenshtein(s1: &str, s2: &str) -> f64 {
        let len1 = s1.chars().count();
        let len2 = s2.chars().count();

        if len1 == 0 && len2 == 0 {
            return 1.0;
        }

        let distance = levenshtein_distance(s1, s2);
        let max_len = len1.max(len2);

        1.0 - (distance as f64 / max_len as f64)
    }

    fn levenshtein_distance(s1: &str, s2: &str) -> usize {
        let s1_chars: Vec<char> = s1.chars().collect();
        let s2_chars: Vec<char> = s2.chars().collect();
        let len1 = s1_chars.len();
        let len2 = s2_chars.len();

        let mut matrix = vec![vec![0; len2 + 1]; len1 + 1];

        for i in 0..=len1 {
            matrix[i][0] = i;
        }
        for j in 0..=len2 {
            matrix[0][j] = j;
        }

        for i in 1..=len1 {
            for j in 1..=len2 {
                let cost = if s1_chars[i - 1] == s2_chars[j - 1] {
                    0
                } else {
                    1
                };
                matrix[i][j] = (matrix[i - 1][j] + 1)
                    .min(matrix[i][j - 1] + 1)
                    .min(matrix[i - 1][j - 1] + cost);
            }
        }

        matrix[len1][len2]
    }
}
