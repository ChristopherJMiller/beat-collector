use axum::{
    extract::{Path, State},
    Json,
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, EntityTrait, QueryOrder, QuerySelect, Set};
use serde::Serialize;

use crate::{
    db::{
        entities::jobs,
        enums::{JobStatus, JobType},
    },
    error::{AppError, Result},
    state::AppState,
};

#[derive(Serialize)]
pub struct JobResponse {
    pub id: i32,
    pub job_type: String,
    pub status: String,
    pub progress: Option<i32>,
    pub processed_items: Option<i32>,
    pub total_items: Option<i32>,
    pub error_message: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct JobCreatedResponse {
    pub job_id: i32,
    pub status: String,
}

pub async fn list_jobs(State(state): State<AppState>) -> Result<Json<Vec<JobResponse>>> {
    let jobs = jobs::Entity::find()
        .order_by_desc(jobs::Column::CreatedAt)
        .limit(50)
        .all(&state.db)
        .await?;

    let responses: Vec<JobResponse> = jobs
        .into_iter()
        .map(|j| JobResponse {
            id: j.id,
            job_type: format!("{:?}", j.job_type),
            status: format!("{:?}", j.status),
            progress: j.progress,
            processed_items: j.processed_items,
            total_items: j.total_items,
            error_message: j.error_message,
            started_at: j.started_at.map(|dt| dt.to_string()),
            completed_at: j.completed_at.map(|dt| dt.to_string()),
            created_at: j.created_at.to_string(),
        })
        .collect();

    Ok(Json(responses))
}

pub async fn get_job_status(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<JobResponse>> {
    let job_record = jobs::Entity::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Job not found".to_string()))?;

    Ok(Json(JobResponse {
        id: job_record.id,
        job_type: format!("{:?}", job_record.job_type),
        status: format!("{:?}", job_record.status),
        progress: job_record.progress,
        processed_items: job_record.processed_items,
        total_items: job_record.total_items,
        error_message: job_record.error_message,
        started_at: job_record.started_at.map(|dt| dt.to_string()),
        completed_at: job_record.completed_at.map(|dt| dt.to_string()),
        created_at: job_record.created_at.to_string(),
    }))
}

pub async fn trigger_spotify_sync(
    State(state): State<AppState>,
) -> Result<Json<JobCreatedResponse>> {
    // Create a new job record
    let now = Utc::now().into();
    let new_job = jobs::ActiveModel {
        job_type: Set(JobType::SpotifySync.as_str().to_string()),
        status: Set(JobStatus::Pending.as_str().to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };

    let inserted_job = new_job.insert(&state.db).await?;

    // Submit job to the queue
    state.job_queue.submit(crate::jobs::queue::JobMessage {
        job_id: inserted_job.id,
        job_type: JobType::SpotifySync,
        entity_id: None,
    })?;

    Ok(Json(JobCreatedResponse {
        job_id: inserted_job.id,
        status: "pending".to_string(),
    }))
}

pub async fn trigger_musicbrainz_match(
    State(state): State<AppState>,
) -> Result<Json<JobCreatedResponse>> {
    // Create a new job record
    let now = Utc::now().into();
    let new_job = jobs::ActiveModel {
        job_type: Set(JobType::MusicbrainzMatch.as_str().to_string()),
        status: Set(JobStatus::Pending.as_str().to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };

    let inserted_job = new_job.insert(&state.db).await?;

    // Submit job to the queue
    state.job_queue.submit(crate::jobs::queue::JobMessage {
        job_id: inserted_job.id,
        job_type: JobType::MusicbrainzMatch,
        entity_id: None,
    })?;

    Ok(Json(JobCreatedResponse {
        job_id: inserted_job.id,
        status: "pending".to_string(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;
    use sea_orm::EntityTrait;

    #[tokio::test]
    async fn test_trigger_spotify_sync_creates_job() {
        let (state, _receiver) = setup_test_app_state_with_queue().await;

        let response = trigger_spotify_sync(State(state.clone()))
            .await
            .expect("Should successfully create job");

        let job_response = response.0;
        assert!(job_response.job_id > 0);
        assert_eq!(job_response.status, "pending");

        // Verify job was created in database
        let job = jobs::Entity::find_by_id(job_response.job_id)
            .one(&state.db)
            .await
            .expect("Query should succeed")
            .expect("Job should exist");

        assert_eq!(job.job_type, JobType::SpotifySync.as_str());
        assert_eq!(job.status, JobStatus::Pending.as_str());
    }

    #[tokio::test]
    async fn test_trigger_spotify_sync_sets_timestamps() {
        let (state, _receiver) = setup_test_app_state_with_queue().await;

        let response = trigger_spotify_sync(State(state.clone()))
            .await
            .expect("Should successfully create job");

        let job_id = response.0.job_id;

        // Verify both created_at and updated_at are set (this was the bug)
        let job = jobs::Entity::find_by_id(job_id)
            .one(&state.db)
            .await
            .expect("Query should succeed")
            .expect("Job should exist");

        assert!(job.created_at.timestamp() > 0, "created_at must be set");
        assert!(job.updated_at.timestamp() > 0, "updated_at must be set");
        // They should be the same on creation
        assert_eq!(job.created_at.timestamp(), job.updated_at.timestamp());
    }

    #[tokio::test]
    async fn test_trigger_musicbrainz_match_creates_job() {
        let (state, _receiver) = setup_test_app_state_with_queue().await;

        let response = trigger_musicbrainz_match(State(state.clone()))
            .await
            .expect("Should successfully create job");

        let job_response = response.0;
        assert!(job_response.job_id > 0);
        assert_eq!(job_response.status, "pending");

        // Verify job was created with correct type
        let job = jobs::Entity::find_by_id(job_response.job_id)
            .one(&state.db)
            .await
            .expect("Query should succeed")
            .expect("Job should exist");

        assert_eq!(job.job_type, JobType::MusicbrainzMatch.as_str());
        assert_eq!(job.status, JobStatus::Pending.as_str());
        assert!(job.updated_at.timestamp() > 0, "updated_at must be set");
    }

    #[tokio::test]
    async fn test_list_jobs_returns_recent_jobs() {
        let state = setup_test_app_state().await;

        // Create multiple jobs
        create_test_job(&state.db, JobType::SpotifySync, JobStatus::Pending).await;
        create_test_job(&state.db, JobType::MusicbrainzMatch, JobStatus::Running).await;
        create_test_job(&state.db, JobType::SpotifySync, JobStatus::Completed).await;

        let response = list_jobs(State(state.clone()))
            .await
            .expect("Should successfully list jobs");

        let jobs = response.0;
        assert_eq!(jobs.len(), 3);

        // Jobs should be ordered by created_at DESC (most recent first)
        // The last created job should be first in the list
        // Note: status is formatted with Debug which wraps the string in quotes
        assert_eq!(jobs[0].status, "\"completed\"");
        assert_eq!(jobs[1].status, "\"running\"");
        assert_eq!(jobs[2].status, "\"pending\"");
    }

    #[tokio::test]
    async fn test_get_job_status_returns_job() {
        let state = setup_test_app_state().await;

        let job = create_test_job(&state.db, JobType::SpotifySync, JobStatus::Running).await;

        let response = get_job_status(State(state.clone()), Path(job.id))
            .await
            .expect("Should successfully get job");

        let job_response = response.0;
        assert_eq!(job_response.id, job.id);
        // Note: job_type and status are formatted with Debug which wraps the string in quotes
        assert_eq!(job_response.job_type, "\"spotify_sync\"");
        assert_eq!(job_response.status, "\"running\"");
    }

    #[tokio::test]
    async fn test_get_job_status_not_found() {
        let state = setup_test_app_state().await;

        let result = get_job_status(State(state.clone()), Path(99999)).await;

        assert!(result.is_err(), "Should return error for non-existent job");
    }
}
