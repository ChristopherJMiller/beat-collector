use axum::{
    extract::{Query, State},
    http::HeaderMap,
    response::{IntoResponse, Redirect},
};
use chrono::{Duration, Utc};
use redis::AsyncCommands;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use serde::Deserialize;

use crate::{
    db::entities::user_settings,
    error::Result,
    services::SpotifyService,
    state::AppState,
};

#[derive(Deserialize)]
pub struct CallbackQuery {
    pub code: String,
    pub state: String,
}

pub async fn authorize(
    State(state): State<AppState>,
) -> Result<impl IntoResponse> {
    let spotify_service = SpotifyService::new(
        state.config.spotify_client_id.clone(),
        state.config.spotify_redirect_uri.clone(),
    );

    let auth_url = spotify_service.generate_authorization_url()?;

    // Store code_verifier in Redis with state as key, short TTL (10 minutes)
    let cache_key = format!("spotify:state:{}", auth_url.state);
    let mut redis_conn = state.redis.clone();
    redis_conn.set_ex(&cache_key, &auth_url.code_verifier, 600).await?;

    // Return HX-Redirect header to redirect the browser to Spotify's authorization page
    let mut headers = HeaderMap::new();
    headers.insert("HX-Redirect", auth_url.url.parse().unwrap());

    Ok((headers, ""))
}

pub async fn callback(
    State(state): State<AppState>,
    Query(params): Query<CallbackQuery>,
) -> Result<impl IntoResponse> {
    // Retrieve code_verifier from Redis using state
    let cache_key = format!("spotify:state:{}", params.state);
    let mut redis_conn = state.redis.clone();
    let code_verifier: String = redis_conn
        .get(&cache_key)
        .await
        .map_err(|_| crate::error::AppError::Authentication("Invalid or expired state".into()))?;

    // Delete the used state
    let _: () = redis_conn.del(&cache_key).await?;

    let spotify_service = SpotifyService::new(
        state.config.spotify_client_id.clone(),
        state.config.spotify_redirect_uri.clone(),
    );

    // Exchange code for tokens
    let token_response = spotify_service
        .exchange_code(&params.code, &code_verifier)
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
            spotify_access_token: settings.spotify_access_token,
            spotify_refresh_token: settings.spotify_refresh_token,
            spotify_token_expires_at: settings.spotify_token_expires_at,
            created_at: Set(Utc::now().into()),
            updated_at: Set(Utc::now().into()),
            ..Default::default()
        };
        new_settings.insert(&state.db).await?;
    }

    // Redirect to settings page with success
    Ok(Redirect::to("/settings"))
}
