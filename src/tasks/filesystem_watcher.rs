use anyhow::Result;
use notify_debouncer_full::{new_debouncer, notify::*, DebounceEventResult};
use sea_orm::EntityTrait;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::sync::mpsc;

use crate::state::AppState;
use super::filesystem_scan::run_filesystem_scan;

/// Start the filesystem watcher for monitoring music directory changes
pub async fn start_watcher(state: AppState, music_path: PathBuf) -> Result<()> {
    tracing::info!("Starting filesystem watcher for: {:?}", music_path);

    // Create a channel to receive file system events
    let (tx, mut rx) = mpsc::unbounded_channel();

    // Create debouncer with 5-second delay to batch events
    let mut debouncer = new_debouncer(
        Duration::from_secs(5),
        None,
        move |result: DebounceEventResult| {
            match result {
                Ok(events) => {
                    for event in events {
                        if let Err(e) = tx.send(event) {
                            tracing::error!("Failed to send filesystem event: {}", e);
                        }
                    }
                }
                Err(errors) => {
                    for error in errors {
                        tracing::error!("Filesystem watch error: {:?}", error);
                    }
                }
            }
        },
    )?;

    // Watch the music directory recursively
    debouncer
        .watcher()
        .watch(&music_path, RecursiveMode::Recursive)?;

    tracing::info!("Filesystem watcher started successfully");

    // Process events in a loop
    tokio::task::spawn(async move {
        while let Some(event) = rx.recv().await {
            tracing::debug!("Filesystem event: {:?}", event);

            // Check if this is a creation or modification of a directory (album added)
            if event.kind.is_create() || event.kind.is_modify() {
                // Trigger a rescan when changes are detected
                // We use debouncing so this won't fire too frequently
                let state_clone = state.clone();
                let music_path_clone = music_path.clone();

                tokio::spawn(async move {
                    tracing::info!("Filesystem changes detected, triggering rescan");
                    if let Err(e) = run_filesystem_scan(state_clone, &music_path_clone).await {
                        tracing::error!("Filesystem scan failed: {}", e);
                    }
                });
            }
        }
    });

    Ok(())
}

/// Initialize the filesystem watcher if music folder is configured
pub async fn init_watcher_if_configured(state: AppState) -> Result<()> {
    // Check if music folder is configured
    if let Some(settings) = crate::db::entities::user_settings::Entity::find()
        .one(&state.db)
        .await?
    {
        if let Some(music_path) = settings.music_folder_path {
            let path = PathBuf::from(music_path);
            if path.exists() && path.is_dir() {
                start_watcher(state, path).await?;
            } else {
                tracing::warn!(
                    "Music folder path configured but doesn't exist: {:?}",
                    path
                );
            }
        } else {
            tracing::info!("No music folder configured, filesystem watcher not started");
        }
    }

    Ok(())
}
