use anyhow::Result;
use tokio_cron_scheduler::{Job, JobScheduler};

use crate::state::AppState;

pub mod spotify_sync;
pub mod musicbrainz_match;
pub mod filesystem_scan;
pub mod filesystem_watcher;
pub mod cover_art;

pub async fn start_scheduler(state: AppState) -> Result<JobScheduler> {
    let scheduler = JobScheduler::new().await?;

    // Add scheduled jobs here
    // Example: Spotify sync every 12 hours (if auto_sync enabled)
    // let spotify_sync_job = Job::new_async("0 0 */12 * * *", move |_uuid, _lock| {
    //     let state = state.clone();
    //     Box::pin(async move {
    //         spotify_sync::run_spotify_sync(state).await.ok();
    //     })
    // })?;
    // scheduler.add(spotify_sync_job).await?;

    // Initialize filesystem watcher if configured
    filesystem_watcher::init_watcher_if_configured(state.clone()).await?;

    scheduler.start().await?;

    Ok(scheduler)
}
