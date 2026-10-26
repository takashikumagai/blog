use axum::{
    Json,
    extract::{FromRef, FromRequestParts, Query, State},
    http::{HeaderMap, HeaderValue, StatusCode, header, request::Parts},
    response::{IntoResponse, Redirect, Response},
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{app, error::AppError};

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct AuthUser {
    pub id: i64,
    pub github_login: Option<String>,
    pub display_name: String,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GithubCallbackQuery {
    code: String,
    state: String,
}

#[derive(Debug, Deserialize)]
struct GithubTokenResponse {
    access_token: String,
}

#[derive(Debug, Deserialize)]
struct GithubProfile {
    id: i64,
    login: String,
    name: Option<String>,
    avatar_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GoogleCallbackQuery {
    code: String,
    state: String,
}

#[derive(Debug, Deserialize)]
struct GoogleTokenResponse {
    id_token: String,
}

#[derive(Debug, Deserialize)]
struct GoogleIdTokenClaims {
    sub: String,
    nonce: String,
    name: Option<String>,
    picture: Option<String>,
}

fn google_auth_configuration() -> Option<(String, String, String)> {
    Some((
        std::env::var("GOOGLE_CLIENT_ID")
            .ok()
            .filter(|value| !value.is_empty())?,
        std::env::var("GOOGLE_CLIENT_SECRET")
            .ok()
            .filter(|value| !value.is_empty())?,
        std::env::var("GOOGLE_REDIRECT_URL")
            .ok()
            .filter(|value| !value.is_empty())?,
    ))
}

async fn verify_google_id_token(
    client: &reqwest::Client,
    id_token: &str,
    client_id: &str,
) -> Result<GoogleIdTokenClaims, String> {
    let token_header = jsonwebtoken::decode_header(id_token)
        .map_err(|error| format!("invalid Google ID token header: {error}"))?;
    let key_id = token_header
        .kid
        .ok_or_else(|| "Google ID token is missing a key ID".to_string())?;
    let key_set = client
        .get("https://www.googleapis.com/oauth2/v3/certs")
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|error| format!("failed to load Google signing keys: {error}"))?
        .json::<jsonwebtoken::jwk::JwkSet>()
        .await
        .map_err(|error| format!("failed to decode Google signing keys: {error}"))?;
    let jwk = key_set
        .keys
        .iter()
        .find(|key| key.common.key_id.as_deref() == Some(key_id.as_str()))
        .ok_or_else(|| "Google signing key was not found".to_string())?;
    let decoding_key = jsonwebtoken::DecodingKey::from_jwk(jwk)
        .map_err(|error| format!("invalid Google signing key: {error}"))?;
    let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::RS256);
    validation.set_issuer(&["https://accounts.google.com", "accounts.google.com"]);
    validation.set_audience(&[client_id]);
    let claims = jsonwebtoken::decode::<GoogleIdTokenClaims>(id_token, &decoding_key, &validation)
        .map_err(|error| format!("Google ID token validation failed: {error}"))?;
    Ok(claims.claims)
}

pub async fn google_login(State(state): State<app::AppState>) -> Response {
    let Some((client_id, _, redirect_url)) = google_auth_configuration() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "Google login is not configured" })),
        )
            .into_response();
    };

    let oauth_state = Uuid::new_v4().to_string();
    let nonce = Uuid::new_v4().to_string();
    if let Err(error) = sqlx::query(
        "INSERT INTO oauth_states (state_hash, nonce_hash, expires_at) VALUES ($1, $2, NOW() + INTERVAL '10 minutes')",
    )
    .bind(hash_secret(&oauth_state))
    .bind(hash_secret(&nonce))
    .execute(&state.db)
    .await
    {
        tracing::error!(error = %error, "failed to create Google OAuth state");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    let mut authorization_url =
        match reqwest::Url::parse("https://accounts.google.com/o/oauth2/v2/auth") {
            Ok(url) => url,
            Err(error) => {
                tracing::error!(error = %error, "invalid Google authorization URL");
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        };
    authorization_url
        .query_pairs_mut()
        .append_pair("client_id", &client_id)
        .append_pair("redirect_uri", &redirect_url)
        .append_pair("response_type", "code")
        .append_pair("scope", "openid email profile")
        .append_pair("state", &oauth_state)
        .append_pair("nonce", &nonce);

    Redirect::to(authorization_url.as_str()).into_response()
}

pub async fn google_callback(
    State(state): State<app::AppState>,
    Query(query): Query<GoogleCallbackQuery>,
) -> Response {
    let Some((client_id, client_secret, redirect_url)) = google_auth_configuration() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "Google login is not configured" })),
        )
            .into_response();
    };

    let stored_nonce_hash = match sqlx::query_scalar::<_, Option<String>>(
        "DELETE FROM oauth_states WHERE state_hash = $1 AND expires_at > NOW() RETURNING nonce_hash",
    )
    .bind(hash_secret(&query.state))
    .fetch_optional(&state.db)
    .await
    {
        Ok(Some(Some(nonce_hash))) => nonce_hash,
        Ok(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "Invalid or expired OAuth state" })),
            )
                .into_response();
        }
        Err(error) => {
            tracing::error!(error = %error, "failed to consume Google OAuth state");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let client = reqwest::Client::new();
    let token = match client
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("code", query.code.as_str()),
            ("redirect_uri", redirect_url.as_str()),
            ("grant_type", "authorization_code"),
        ])
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
    {
        Ok(response) => match response.json::<GoogleTokenResponse>().await {
            Ok(token) => token,
            Err(error) => {
                tracing::error!(error = %error, "failed to decode Google token response");
                return StatusCode::BAD_GATEWAY.into_response();
            }
        },
        Err(error) => {
            tracing::error!(error = %error, "Google token exchange failed");
            return StatusCode::BAD_GATEWAY.into_response();
        }
    };

    let profile = match verify_google_id_token(&client, &token.id_token, &client_id).await {
        Ok(profile) if hash_secret(&profile.nonce) == stored_nonce_hash => profile,
        Ok(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "Invalid Google ID token nonce" })),
            )
                .into_response();
        }
        Err(error) => {
            tracing::error!(error = %error, "Google ID token verification failed");
            return StatusCode::BAD_GATEWAY.into_response();
        }
    };

    let display_name = profile.name.unwrap_or_else(|| profile.sub.clone());
    let user_id = sqlx::query_scalar::<_, i64>(
        r#"INSERT INTO users (google_sub, display_name, avatar_url)
           VALUES ($1, $2, $3)
           ON CONFLICT (google_sub) DO UPDATE SET
               display_name = EXCLUDED.display_name,
               avatar_url = EXCLUDED.avatar_url,
               updated_at = NOW()
           RETURNING id"#,
    )
    .bind(&profile.sub)
    .bind(display_name)
    .bind(profile.picture)
    .fetch_one(&state.db)
    .await;
    let user_id = match user_id {
        Ok(user_id) => user_id,
        Err(error) => {
            tracing::error!(error = %error, "failed to upsert Google user");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    create_session_response(&state.db, user_id).await
}

pub fn hash_secret(secret: &str) -> String {
    format!("{:x}", Sha256::digest(secret.as_bytes()))
}

fn auth_configuration() -> Option<(String, String, String)> {
    Some((
        std::env::var("GITHUB_CLIENT_ID")
            .ok()
            .filter(|value| !value.is_empty())?,
        std::env::var("GITHUB_CLIENT_SECRET")
            .ok()
            .filter(|value| !value.is_empty())?,
        std::env::var("GITHUB_REDIRECT_URL")
            .ok()
            .filter(|value| !value.is_empty())?,
    ))
}

pub async fn github_login(State(state): State<app::AppState>) -> Response {
    let Some((client_id, _, redirect_url)) = auth_configuration() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "GitHub login is not configured" })),
        )
            .into_response();
    };

    let oauth_state = Uuid::new_v4().to_string();
    if let Err(error) = sqlx::query(
        "INSERT INTO oauth_states (state_hash, expires_at) VALUES ($1, NOW() + INTERVAL '10 minutes')",
    )
    .bind(hash_secret(&oauth_state))
    .execute(&state.db)
    .await
    {
        tracing::error!(error = %error, "failed to create OAuth state");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    let mut authorization_url =
        match reqwest::Url::parse("https://github.com/login/oauth/authorize") {
            Ok(url) => url,
            Err(error) => {
                tracing::error!(error = %error, "invalid GitHub authorization URL");
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        };
    authorization_url
        .query_pairs_mut()
        .append_pair("client_id", &client_id)
        .append_pair("redirect_uri", &redirect_url)
        .append_pair("scope", "read:user")
        .append_pair("state", &oauth_state);

    Redirect::to(authorization_url.as_str()).into_response()
}

pub async fn github_callback(
    State(state): State<app::AppState>,
    Query(query): Query<GithubCallbackQuery>,
) -> Response {
    let Some((client_id, client_secret, _)) = auth_configuration() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "GitHub login is not configured" })),
        )
            .into_response();
    };

    let consumed_state = sqlx::query_scalar::<_, String>(
        "DELETE FROM oauth_states WHERE state_hash = $1 AND expires_at > NOW() RETURNING state_hash",
    )
    .bind(hash_secret(&query.state))
    .fetch_optional(&state.db)
    .await;
    if !matches!(consumed_state, Ok(Some(_))) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Invalid or expired OAuth state" })),
        )
            .into_response();
    }

    let client = reqwest::Client::new();
    let token = match client
        .post("https://github.com/login/oauth/access_token")
        .header(header::ACCEPT, "application/json")
        .form(&[
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("code", query.code.as_str()),
            ("state", query.state.as_str()),
        ])
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
    {
        Ok(response) => match response.json::<GithubTokenResponse>().await {
            Ok(token) => token,
            Err(error) => {
                tracing::error!(error = %error, "failed to decode GitHub token response");
                return StatusCode::BAD_GATEWAY.into_response();
            }
        },
        Err(error) => {
            tracing::error!(error = %error, "GitHub token exchange failed");
            return StatusCode::BAD_GATEWAY.into_response();
        }
    };

    let profile = match client
        .get("https://api.github.com/user")
        .bearer_auth(token.access_token)
        .header(header::USER_AGENT, "Nyankospace-Blog")
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
    {
        Ok(response) => match response.json::<GithubProfile>().await {
            Ok(profile) => profile,
            Err(error) => {
                tracing::error!(error = %error, "failed to decode GitHub profile");
                return StatusCode::BAD_GATEWAY.into_response();
            }
        },
        Err(error) => {
            tracing::error!(error = %error, "GitHub profile request failed");
            return StatusCode::BAD_GATEWAY.into_response();
        }
    };

    let display_name = profile.name.unwrap_or_else(|| profile.login.clone());
    let user_id = sqlx::query_scalar::<_, i64>(
        r#"INSERT INTO users (github_id, github_login, display_name, avatar_url)
           VALUES ($1, $2, $3, $4)
           ON CONFLICT (github_id) DO UPDATE SET
               github_login = EXCLUDED.github_login,
               display_name = EXCLUDED.display_name,
               avatar_url = EXCLUDED.avatar_url,
               updated_at = NOW()
           RETURNING id"#,
    )
    .bind(profile.id)
    .bind(&profile.login)
    .bind(display_name)
    .bind(profile.avatar_url)
    .fetch_one(&state.db)
    .await;
    let user_id = match user_id {
        Ok(user_id) => user_id,
        Err(error) => {
            tracing::error!(error = %error, "failed to upsert user");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    create_session_response(&state.db, user_id).await
}

async fn create_session_response(db: &PgPool, user_id: i64) -> Response {
    let session_token = Uuid::new_v4().to_string();
    if let Err(error) = sqlx::query(
        "INSERT INTO sessions (token_hash, user_id, expires_at) VALUES ($1, $2, NOW() + INTERVAL '30 days')",
    )
    .bind(hash_secret(&session_token))
    .bind(user_id)
    .execute(db)
    .await
    {
        tracing::error!(error = %error, "failed to create session");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    let mut response = Redirect::to("/").into_response();
    let cookie = session_cookie(&session_token, 2_592_000);
    match HeaderValue::from_str(&cookie) {
        Ok(value) => {
            response.headers_mut().insert(header::SET_COOKIE, value);
            response
        }
        Err(error) => {
            tracing::error!(error = %error, "failed to construct session cookie");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// The `Set-Cookie` value for the session cookie. `Secure` is on unless COOKIE_SECURE=false
/// (needed for plain-http local development).
fn session_cookie(value: &str, max_age_seconds: u32) -> String {
    let secure = if std::env::var("COOKIE_SECURE").is_ok_and(|value| value == "false") {
        ""
    } else {
        "; Secure"
    };
    format!("nyanko_space_blog_session={value}; Path=/; HttpOnly; SameSite=Lax; Max-Age={max_age_seconds}{secure}")
}

pub fn session_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .find_map(|cookie| {
            let (name, value) = cookie.trim().split_once('=')?;
            (name == "nyanko_space_blog_session").then_some(value)
        })
}

pub async fn authenticate_user(db: &PgPool, headers: &HeaderMap) -> Result<AuthUser, AppError> {
    let token = session_token(headers).ok_or(AppError::Unauthorized)?;
    sqlx::query_as::<_, AuthUser>(
        r#"SELECT users.id, users.github_login, users.display_name, users.avatar_url
           FROM sessions JOIN users ON users.id = sessions.user_id
           WHERE sessions.token_hash = $1 AND sessions.expires_at > NOW()"#,
    )
    .bind(hash_secret(token))
    .fetch_optional(db)
    .await?
    .ok_or(AppError::Unauthorized)
}

impl<S> FromRequestParts<S> for AuthUser
where
    app::AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let app_state = app::AppState::from_ref(state);
        authenticate_user(&app_state.db, &parts.headers).await
    }
}

pub async fn current_user(user: AuthUser) -> Response {
    (StatusCode::OK, Json(user)).into_response()
}

pub async fn logout(State(state): State<app::AppState>, headers: HeaderMap) -> Response {
    if let Some(token) = session_token(&headers) {
        if let Err(error) = sqlx::query("DELETE FROM sessions WHERE token_hash = $1")
            .bind(hash_secret(token))
            .execute(&state.db)
            .await
        {
            tracing::error!(error = %error, "failed to delete session");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }

    let mut response = StatusCode::NO_CONTENT.into_response();
    let cookie = session_cookie("", 0);
    if let Ok(value) = HeaderValue::from_str(&cookie) {
        response.headers_mut().insert(header::SET_COOKIE, value);
    }
    response
}
