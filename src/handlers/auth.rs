use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use chrono::{Duration, Utc};
use sea_orm::{ActiveModelTrait, Set};
use serde::{Deserialize, Serialize};

use crate::{
    db::entities::user_settings,
    error::Result,
    services::SpotifyService,
    state::AppState,
};

#[derive(Serialize)]
pub struct AuthorizeResponse {
    pub authorization_url: String,
}

#[derive(Deserialize)]
pub struct CallbackRequest {
    pub code: String,
    pub code_verifier: String,
}

#[derive(Serialize)]
pub struct CallbackResponse {
    pub success: bool,
    pub expires_at: String,
}

pub async fn authorize(
    State(state): State<AppState>,
) -> Result<Json<AuthorizeResponse>> {
    let spotify_service = SpotifyService::new(
        state.config.spotify_client_id.clone(),
        state.config.spotify_redirect_uri.clone(),
    );

    let auth_url = spotify_service.generate_authorization_url()?;

    // Store code_verifier in Redis with short TTL (10 minutes)
    let cache_key = format!("spotify:verifier:{}", auth_url.code_verifier);
    state
        .redis
        .clone()
        .set_ex(&cache_key, &auth_url.code_verifier, 600)
        .await?;

    Ok(Json(AuthorizeResponse {
        authorization_url: auth_url.url,
    }))
}

pub async fn callback(
    State(state): State<AppState>,
    Json(payload): Json<CallbackRequest>,
) -> Result<Json<CallbackResponse>> {
    let spotify_service = SpotifyService::new(
        state.config.spotify_client_id.clone(),
        state.config.spotify_redirect_uri.clone(),
    );

    // Exchange code for tokens
    let token_response = spotify_service
        .exchange_code(&payload.code, &payload.code_verifier)
        .await?;

    let expires_at = Utc::now() + Duration::seconds(token_response.expires_in);

    // Save tokens to database
    let settings = user_settings::ActiveModel {
        spotify_access_token: Set(Some(token_response.access_token)),
        spotify_refresh_token: Set(token_response.refresh_token),
        spotify_token_expires_at: Set(Some(expires_at.into())),
        ..Default::default()
    };

    // Get first user settings or create new
    let existing = user_settings::Entity::find()
        .one(&state.db)
        .await?;

    if let Some(existing_settings) = existing {
        let mut active: user_settings::ActiveModel = existing_settings.into();
        active.spotify_access_token = settings.spotify_access_token;
        active.spotify_refresh_token = settings.spotify_refresh_token;
        active.spotify_token_expires_at = settings.spotify_token_expires_at;
        active.update(&state.db).await?;
    } else {
        let new_settings = user_settings::ActiveModel {
            id: Set(uuid::Uuid::new_v4()),
            spotify_access_token: settings.spotify_access_token,
            spotify_refresh_token: settings.spotify_refresh_token,
            spotify_token_expires_at: settings.spotify_token_expires_at,
            created_at: Set(Utc::now().into()),
            updated_at: Set(Utc::now().into()),
            ..Default::default()
        };
        new_settings.insert(&state.db).await?;
    }

    Ok(Json(CallbackResponse {
        success: true,
        expires_at: expires_at.to_rfc3339(),
    }))
}
