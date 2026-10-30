use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
    routing::{get, post},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

pub mod app;
pub mod auth;
pub mod db;
pub mod error;
pub mod models;

use app::AppState;
use error::AppError;
use models::{Post, PostSummary};

#[derive(Debug, Default, Deserialize)]
pub struct PostFilters {
    pub genre: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Deserialize)]
pub struct PostInput {
    pub title: String,
    pub content: String,
    #[serde(default)]
    pub genre: Option<String>,
    #[serde(default)]
    pub excerpt: Option<String>,
    #[serde(default = "default_published")]
    pub published: bool,
    #[serde(default)]
    pub published_at: Option<DateTime<Utc>>,
}

fn default_limit() -> i64 {
    20
}

fn default_published() -> bool {
    true
}

pub async fn app_with_db(db: PgPool) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/auth/google", get(auth::google_login))
        .route("/auth/google/callback", get(auth::google_callback))
        .route("/auth/github", get(auth::github_login))
        .route("/auth/github/callback", get(auth::github_callback))
        .route("/auth/me", get(auth::current_user))
        .route("/auth/logout", post(auth::logout))
        .route("/posts", get(list_posts))
        .route(
            "/posts/{id}",
            get(get_post)
                .post(create_post)
                .put(update_post)
                .delete(delete_post),
        )
        .with_state(AppState { db })
}

async fn auth_not_implemented() -> impl IntoResponse {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(serde_json::json!({"error": "authentication is not available yet"})),
    )
}

async fn health_check(State(state): State<AppState>) -> impl IntoResponse {
    match sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.db)
        .await
    {
        Ok(_) => (StatusCode::OK, Json(serde_json::json!({"status": "ok"}))),
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"status": "database unavailable"})),
        ),
    }
}

async fn list_posts(
    State(state): State<AppState>,
    Query(filters): Query<PostFilters>,
) -> Result<Json<PaginatedResponse<PostSummary>>, AppError> {
    let limit = filters.limit.clamp(1, 100);
    let offset = filters.offset.max(0);
    let genre = filters
        .genre
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());

    let (items, total) = if let Some(genre) = genre {
        let items = sqlx::query_as::<_, PostSummary>(
            "SELECT id, title, genre, excerpt, published, published_at, created_at, updated_at \
             FROM posts WHERE published = TRUE AND LOWER(genre) = LOWER($1) \
             ORDER BY published_at DESC, id ASC LIMIT $2 OFFSET $3",
        )
        .bind(genre)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.db)
        .await?;
        let total = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM posts WHERE published = TRUE AND LOWER(genre) = LOWER($1)",
        )
        .bind(genre)
        .fetch_one(&state.db)
        .await?;
        (items, total)
    } else {
        let items = sqlx::query_as::<_, PostSummary>(
            "SELECT id, title, genre, excerpt, published, published_at, created_at, updated_at \
             FROM posts WHERE published = TRUE ORDER BY published_at DESC, id ASC LIMIT $1 OFFSET $2",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.db)
        .await?;
        let total =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM posts WHERE published = TRUE")
                .fetch_one(&state.db)
                .await?;
        (items, total)
    };

    Ok(Json(PaginatedResponse {
        items,
        total,
        limit,
        offset,
    }))
}

async fn get_post(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Post>, AppError> {
    let post = sqlx::query_as::<_, Post>("SELECT * FROM posts WHERE id = $1 AND published = TRUE")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(Json(post))
}

async fn create_post(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(input): Json<PostInput>,
) -> Result<impl IntoResponse, AppError> {
    authorize_sync(&headers)?;
    validate_post(&id, &input)?;
    let result = sqlx::query(
        "INSERT INTO posts (id, title, content, genre, excerpt, published, published_at) \
         VALUES ($1, $2, $3, $4, $5, $6, COALESCE($7, NOW()))",
    )
    .bind(id)
    .bind(input.title.trim())
    .bind(input.content)
    .bind(
        input
            .genre
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty()),
    )
    .bind(
        input
            .excerpt
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty()),
    )
    .bind(input.published)
    .bind(input.published_at)
    .execute(&state.db)
    .await;

    match result {
        Ok(_) => Ok(StatusCode::CREATED),
        Err(sqlx::Error::Database(error)) if error.is_unique_violation() => Err(AppError::Conflict),
        Err(error) => Err(error.into()),
    }
}

async fn update_post(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(input): Json<PostInput>,
) -> Result<StatusCode, AppError> {
    authorize_sync(&headers)?;
    validate_post(&id, &input)?;
    sqlx::query(
        "INSERT INTO posts (id, title, content, genre, excerpt, published, published_at) \
         VALUES ($1, $2, $3, $4, $5, $6, COALESCE($7, NOW())) \
         ON CONFLICT (id) DO UPDATE SET title = EXCLUDED.title, content = EXCLUDED.content, \
         genre = EXCLUDED.genre, excerpt = EXCLUDED.excerpt, published = EXCLUDED.published, \
         published_at = EXCLUDED.published_at, updated_at = NOW()",
    )
    .bind(id)
    .bind(input.title.trim())
    .bind(input.content)
    .bind(
        input
            .genre
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty()),
    )
    .bind(
        input
            .excerpt
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty()),
    )
    .bind(input.published)
    .bind(input.published_at)
    .execute(&state.db)
    .await?;

    Ok(StatusCode::OK)
}

async fn delete_post(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<StatusCode, AppError> {
    authorize_sync(&headers)?;
    let result = sqlx::query("DELETE FROM posts WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

fn authorize_sync(headers: &HeaderMap) -> Result<(), AppError> {
    let expected = std::env::var("BLOG_SYNC_TOKEN").map_err(|_| AppError::SyncNotConfigured)?;
    let provided = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or(AppError::Unauthorized)?;
    if expected.is_empty() || provided != expected {
        return Err(AppError::Unauthorized);
    }
    Ok(())
}

fn validate_post(id: &str, input: &PostInput) -> Result<(), AppError> {
    let is_slug = !id.is_empty()
        && id.len() <= 200
        && id.split('-').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        });
    if !is_slug || input.title.trim().is_empty() || input.content.trim().is_empty() {
        return Err(AppError::InvalidInput);
    }
    Ok(())
}
