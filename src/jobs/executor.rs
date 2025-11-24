use anyhow::Result;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use tokio::sync::mpsc;

use crate::{
    db::{
        entities::jobs,
        enums::{JobStatus, JobType},
    },
    jobs::queue::JobMessage,
    state::AppState,
    tasks::{filesystem_scan, musicbrainz_match, spotify_sync},
};

/// Background job executor that processes jobs from the queue
pub struct JobExecutor {
    state: AppState,
    receiver: mpsc::UnboundedReceiver<JobMessage>,
}

impl JobExecutor {
    pub fn new(state: AppState, receiver: mpsc::UnboundedReceiver<JobMessage>) -> Self {
        Self { state, receiver }
    }

    /// Start the job executor loop
    pub async fn start(mut self) {
        tracing::info!("Job executor started");

        while let Some(message) = self.receiver.recv().await {
            tracing::info!(
                "Processing job {} ({:?})",
                message.job_id,
                message.job_type
            );

            // Spawn each job in its own task to allow concurrent processing
            let state = self.state.clone();
            tokio::spawn(async move {
                if let Err(e) = Self::execute_job(state, message).await {
                    tracing::error!("Job execution failed: {}", e);
                }
            });
        }

        tracing::warn!("Job executor stopped - queue closed");
    }

    /// Execute a single job
    async fn execute_job(state: AppState, message: JobMessage) -> Result<()> {
        let job_id = message.job_id;

        // Update job status to Running
        if let Err(e) = Self::update_job_status(
            &state,
            job_id,
            JobStatus::Running,
            None,
            Some(Utc::now().into()),
        )
        .await
        {
            tracing::error!("Failed to update job status to running: {}", e);
        }

        // Execute the job based on type
        let result = match message.job_type {
            JobType::SpotifySync => spotify_sync::run_spotify_sync(state.clone()).await,

            JobType::MusicbrainzMatch => {
                musicbrainz_match::run_musicbrainz_match(state.clone()).await
            }

            JobType::FilesystemScan => {
                if let Some(settings) = crate::db::entities::user_settings::Entity::find()
                    .one(&state.db)
                    .await?
                {
                    if let Some(music_path) = settings.music_folder_path {
                        filesystem_scan::run_filesystem_scan(
                            state.clone(),
                            std::path::Path::new(&music_path),
                        )
                        .await
                    } else {
                        Err(anyhow::anyhow!("Music folder path not configured"))
                    }
                } else {
                    Err(anyhow::anyhow!("User settings not found"))
                }
            }

            JobType::LidarrSearch => {
                // TODO: Implement Lidarr search job
                Err(anyhow::anyhow!("Lidarr search not yet implemented"))
            }

            JobType::CoverArtFetch => {
                // TODO: Implement cover art fetch job
                Err(anyhow::anyhow!("Cover art fetch not yet implemented"))
            }
        };

        // Update job status based on result
        match result {
            Ok(_) => {
                tracing::info!("Job {} completed successfully", job_id);
                Self::update_job_status(
                    &state,
                    job_id,
                    JobStatus::Completed,
                    None,
                    None,
                )
                .await?;
            }
            Err(e) => {
                tracing::error!("Job {} failed: {}", job_id, e);
                Self::update_job_status(
                    &state,
                    job_id,
                    JobStatus::Failed,
                    Some(e.to_string()),
                    None,
                )
                .await?;
            }
        }

        Ok(())
    }

    /// Update job status in database
    async fn update_job_status(
        state: &AppState,
        job_id: i32,
        status: JobStatus,
        error_message: Option<String>,
        started_at: Option<chrono::DateTime<chrono::FixedOffset>>,
    ) -> Result<()> {
        let job_record = jobs::Entity::find_by_id(job_id)
            .one(&state.db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Job not found: {}", job_id))?;

        let mut active: jobs::ActiveModel = job_record.into();
        active.status = Set(status.as_str().to_string());

        if let Some(msg) = error_message {
            active.error_message = Set(Some(msg));
        }

        if let Some(start) = started_at {
            active.started_at = Set(Some(start.with_timezone(&chrono::Utc).into()));
        }

        if status == JobStatus::Completed || status == JobStatus::Failed {
            active.completed_at = Set(Some(Utc::now().into()));
        }

        active.update(&state.db).await?;
        Ok(())
    }
}
