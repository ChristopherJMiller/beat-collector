use axum::{
    extract::{Query, State},
    http::HeaderMap,
    response::{Html, IntoResponse, Redirect},
    Json,
};
use chrono::{Duration, Utc};
use maud::html;
use redis::AsyncCommands;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use serde::{Deserialize, Serialize};

use crate::{
    db::entities::user_settings,
    error::Result,
    services::SpotifyService,
    state::AppState,
};

#[derive(Serialize)]
pub struct SpotifyStatus {
    pub connected: bool,
    pub needs_reauth: bool,
}

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

/// Check Spotify connection status and attempt token refresh if needed
pub async fn spotify_status(
    State(state): State<AppState>,
) -> Result<Json<SpotifyStatus>> {
    let settings = user_settings::Entity::find()
        .one(&state.db)
        .await?;

    let Some(settings) = settings else {
        return Ok(Json(SpotifyStatus {
            connected: false,
            needs_reauth: true,
        }));
    };

    // No token at all
    let Some(access_token) = &settings.spotify_access_token else {
        return Ok(Json(SpotifyStatus {
            connected: false,
            needs_reauth: true,
        }));
    };

    // Check if token is expired
    let is_expired = settings
        .spotify_token_expires_at
        .map(|exp| Utc::now() + Duration::minutes(5) >= exp.to_utc())
        .unwrap_or(true);

    if !is_expired {
        return Ok(Json(SpotifyStatus {
            connected: true,
            needs_reauth: false,
        }));
    }

    // Try to refresh the token
    let Some(refresh_token) = &settings.spotify_refresh_token else {
        return Ok(Json(SpotifyStatus {
            connected: false,
            needs_reauth: true,
        }));
    };

    let spotify_service = SpotifyService::new(
        state.config.spotify_client_id.clone(),
        state.config.spotify_redirect_uri.clone(),
    );

    match spotify_service.refresh_token(refresh_token).await {
        Ok(token_response) => {
            // Update tokens in database
            let expires_at = Utc::now() + Duration::seconds(token_response.expires_in);
            let mut active: user_settings::ActiveModel = settings.into();
            active.spotify_access_token = Set(Some(token_response.access_token));
            if let Some(new_refresh) = token_response.refresh_token {
                active.spotify_refresh_token = Set(Some(new_refresh));
            }
            active.spotify_token_expires_at = Set(Some(expires_at.into()));
            active.updated_at = Set(Utc::now().into());
            active.update(&state.db).await?;

            Ok(Json(SpotifyStatus {
                connected: true,
                needs_reauth: false,
            }))
        }
        Err(_) => {
            // Refresh failed, need re-auth
            Ok(Json(SpotifyStatus {
                connected: false,
                needs_reauth: true,
            }))
        }
    }
}

/// HTML partial for Spotify button - checks status and renders appropriate button
pub async fn spotify_button(
    State(state): State<AppState>,
) -> Result<Html<String>> {
    let settings = user_settings::Entity::find()
        .one(&state.db)
        .await?;

    let mut needs_auth = true;

    if let Some(settings) = settings {
        if settings.spotify_access_token.is_some() {
            // Check if expired
            let is_expired = settings
                .spotify_token_expires_at
                .map(|exp| Utc::now() + Duration::minutes(5) >= exp.to_utc())
                .unwrap_or(true);

            if !is_expired {
                needs_auth = false;
            } else if let Some(refresh_token) = &settings.spotify_refresh_token {
                // Try refresh
                let spotify_service = SpotifyService::new(
                    state.config.spotify_client_id.clone(),
                    state.config.spotify_redirect_uri.clone(),
                );

                if let Ok(token_response) = spotify_service.refresh_token(refresh_token).await {
                    let expires_at = Utc::now() + Duration::seconds(token_response.expires_in);
                    let mut active: user_settings::ActiveModel = settings.into();
                    active.spotify_access_token = Set(Some(token_response.access_token));
                    if let Some(new_refresh) = token_response.refresh_token {
                        active.spotify_refresh_token = Set(Some(new_refresh));
                    }
                    active.spotify_token_expires_at = Set(Some(expires_at.into()));
                    active.updated_at = Set(Utc::now().into());
                    let _ = active.update(&state.db).await;
                    needs_auth = false;
                }
            }
        }
    }

    let markup = if needs_auth {
        html! {
            button
                class="px-4 py-2 bg-green-500 hover:bg-green-600 text-white font-semibold rounded-md flex items-center space-x-2"
                hx-get="/api/auth/spotify/authorize"
                hx-swap="none" {
                span { "🔗" }
                span { "Login with Spotify" }
            }
        }
    } else {
        html! {
            button
                class="px-4 py-2 bg-primary hover:bg-green-600 text-white font-semibold rounded-md flex items-center space-x-2"
                hx-post="/api/jobs/spotify-sync"
                hx-target="#notification-area"
                hx-swap="innerHTML" {
                span { "🔄" }
                span { "Sync with Spotify" }
            }
        }
    };

    Ok(Html(markup.into_string()))
}
