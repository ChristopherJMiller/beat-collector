use axum::{
    extract::{Path, State},
    Json,
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, EntityTrait, QueryOrder, Set};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    db::entities::{job, Job},
    error::{AppError, Result},
    state::AppState,
};

#[derive(Serialize)]
pub struct JobResponse {
    pub id: Uuid,
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
    pub job_id: Uuid,
    pub status: String,
}

pub async fn list_jobs(State(state): State<AppState>) -> Result<Json<Vec<JobResponse>>> {
    let jobs = Job::find()
        .order_by_desc(job::Column::CreatedAt)
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
    Path(id): Path<Uuid>,
) -> Result<Json<JobResponse>> {
    let job_record = Job::find_by_id(id)
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
    let job_id = Uuid::new_v4();
    let new_job = job::ActiveModel {
        id: Set(job_id),
        job_type: Set(job::JobType::SpotifySync),
        status: Set(job::JobStatus::Pending),
        created_at: Set(Utc::now().into()),
        ..Default::default()
    };

    new_job.insert(&state.db).await?;

    // TODO: Trigger actual background job processing

    Ok(Json(JobCreatedResponse {
        job_id,
        status: "pending".to_string(),
    }))
}

pub async fn trigger_musicbrainz_match(
    State(state): State<AppState>,
) -> Result<Json<JobCreatedResponse>> {
    // Create a new job record
    let job_id = Uuid::new_v4();
    let new_job = job::ActiveModel {
        id: Set(job_id),
        job_type: Set(job::JobType::MusicbrainzMatch),
        status: Set(job::JobStatus::Pending),
        created_at: Set(Utc::now().into()),
        ..Default::default()
    };

    new_job.insert(&state.db).await?;

    // TODO: Trigger actual background job processing

    Ok(Json(JobCreatedResponse {
        job_id,
        status: "pending".to_string(),
    }))
}
