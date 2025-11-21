use axum::{extract::State, Json};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    db::entities::{user_settings, UserSettings},
    error::{AppError, Result},
    services::LidarrService,
    state::AppState,
};

#[derive(Serialize)]
pub struct SettingsResponse {
    pub id: Uuid,
    pub lidarr_url: Option<String>,
    pub music_folder_path: Option<String>,
    pub auto_sync_enabled: Option<bool>,
    pub sync_interval_hours: Option<i32>,
    pub spotify_connected: bool,
}

#[derive(Deserialize)]
pub struct UpdateSettingsRequest {
    pub lidarr_url: Option<String>,
    pub lidarr_api_key: Option<String>,
    pub music_folder_path: Option<String>,
    pub auto_sync_enabled: Option<bool>,
    pub sync_interval_hours: Option<i32>,
}

#[derive(Serialize)]
pub struct TestConnectionResponse {
    pub success: bool,
    pub message: String,
}

pub async fn get_settings(State(state): State<AppState>) -> Result<Json<SettingsResponse>> {
    let settings = UserSettings::find()
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Settings not found".to_string()))?;

    Ok(Json(SettingsResponse {
        id: settings.id,
        lidarr_url: settings.lidarr_url,
        music_folder_path: settings.music_folder_path,
        auto_sync_enabled: settings.auto_sync_enabled,
        sync_interval_hours: settings.sync_interval_hours,
        spotify_connected: settings.spotify_access_token.is_some(),
    }))
}

pub async fn update_settings(
    State(state): State<AppState>,
    Json(payload): Json<UpdateSettingsRequest>,
) -> Result<Json<SettingsResponse>> {
    // Get existing settings or create new
    let existing = UserSettings::find().one(&state.db).await?;

    let settings = if let Some(existing_settings) = existing {
        let mut active: user_settings::ActiveModel = existing_settings.into();

        if let Some(url) = payload.lidarr_url {
            active.lidarr_url = Set(Some(url));
        }

        if let Some(key) = payload.lidarr_api_key {
            active.lidarr_api_key = Set(Some(key));
        }

        if let Some(path) = payload.music_folder_path {
            active.music_folder_path = Set(Some(path));
        }

        if let Some(enabled) = payload.auto_sync_enabled {
            active.auto_sync_enabled = Set(Some(enabled));
        }

        if let Some(interval) = payload.sync_interval_hours {
            active.sync_interval_hours = Set(Some(interval));
        }

        active.updated_at = Set(Utc::now().into());
        active.update(&state.db).await?
    } else {
        let new_settings = user_settings::ActiveModel {
            id: Set(Uuid::new_v4()),
            lidarr_url: Set(payload.lidarr_url),
            lidarr_api_key: Set(payload.lidarr_api_key),
            music_folder_path: Set(payload.music_folder_path),
            auto_sync_enabled: Set(payload.auto_sync_enabled),
            sync_interval_hours: Set(payload.sync_interval_hours),
            created_at: Set(Utc::now().into()),
            updated_at: Set(Utc::now().into()),
            ..Default::default()
        };
        new_settings.insert(&state.db).await?
    };

    Ok(Json(SettingsResponse {
        id: settings.id,
        lidarr_url: settings.lidarr_url,
        music_folder_path: settings.music_folder_path,
        auto_sync_enabled: settings.auto_sync_enabled,
        sync_interval_hours: settings.sync_interval_hours,
        spotify_connected: settings.spotify_access_token.is_some(),
    }))
}

pub async fn test_lidarr_connection(
    State(state): State<AppState>,
) -> Result<Json<TestConnectionResponse>> {
    let settings = UserSettings::find()
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::Configuration("Settings not configured".to_string()))?;

    let lidarr_url = settings
        .lidarr_url
        .ok_or_else(|| AppError::Configuration("Lidarr URL not configured".to_string()))?;

    let lidarr_api_key = settings
        .lidarr_api_key
        .ok_or_else(|| AppError::Configuration("Lidarr API key not configured".to_string()))?;

    let lidarr_service = LidarrService::new();

    match lidarr_service
        .test_connection(&lidarr_url, &lidarr_api_key)
        .await
    {
        Ok(true) => Ok(Json(TestConnectionResponse {
            success: true,
            message: "Successfully connected to Lidarr".to_string(),
        })),
        Ok(false) => Ok(Json(TestConnectionResponse {
            success: false,
            message: "Failed to connect to Lidarr".to_string(),
        })),
        Err(e) => Ok(Json(TestConnectionResponse {
            success: false,
            message: format!("Connection error: {}", e),
        })),
    }
}
