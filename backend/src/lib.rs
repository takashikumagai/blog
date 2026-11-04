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
pub use models::{Post, PostSummary};

#[derive(Debug, Default, Deserialize)]
pub struct PostFilters {
    pub genre: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Default, Deserialize)]
pub struct PostInput {
    pub title: String,
    pub content: String,
    #[serde(default)]
    pub genre: Option<String>,
    #[serde(default)]
    pub excerpt: Option<String>,
    // This attribute is only triggered when Serde is actively parsing a JSON string
    // into the struct. If the published key is completely missing from the incoming
    // JSON payload, Serde will call default_published() instead of the standard
    // boolean default.
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

// async fn auth_not_implemented() -> impl IntoResponse {
//     (
//         StatusCode::NOT_IMPLEMENTED,
//         Json(serde_json::json!({"error": "authentication is not available yet"})),
//     )
// }

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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::extract::State;
    use axum::response::IntoResponse;
    use axum::http::StatusCode;

    // This macro automatically provisions an isolated database pool for this test,
    // runs migrations, and passes the pool into your test function argument.
    #[sqlx::test]
    async fn test_health_check_success(pool: sqlx::PgPool) {
        // Arrange
        let state = AppState { db: pool };

        // Act
        let response = health_check(State(state)).await.into_response();

        // Assert
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn test_validate_post_valid_cases() {
        let input = PostInput {
            title: "Valid Title".to_string(),
            content: "Valid Content".to_string(),
            genre: None,
            excerpt: None,
            published: true,
            published_at: None,
        };

        // Standard valid slugs
        assert!(validate_post("hello-world", &input).is_ok());
        assert!(validate_post("nyanko-blog-2026", &input).is_ok());
        assert!(validate_post("simple", &input).is_ok());
    }

    #[test]
    fn test_validate_post_invalid_slugs() {
        let input = PostInput {
            title: "Valid Title".to_string(),
            content: "Valid Content".to_string(),
            ..Default::default()
        };

        // Uppercase characters are not allowed
        assert!(validate_post("Hello-World", &input).is_err());
        // Trailing/leading dashes result in empty parts via .split('-')
        assert!(validate_post("hello-world-", &input).is_err());
        assert!(validate_post("-hello-world", &input).is_err());
        // Double dashes leave an empty chunk between them
        assert!(validate_post("hello--world", &input).is_err());
        // Special characters or spaces are invalid
        assert!(validate_post("hello_world", &input).is_err());
        assert!(validate_post("hello world", &input).is_err());
        assert!(validate_post("hello!", &input).is_err());
        // Empty slugs are caught
        assert!(validate_post("", &input).is_err());
        // Exceeding 200 characters limit
        assert!(validate_post(&"a".repeat(201), &input).is_err());
    }

    #[test]
    fn test_validate_post_invalid_payloads() {
        let invalid_title = PostInput {
            title: "   ".to_string(), // Whitespace only
            content: "Valid content".to_string(),
            ..Default::default()
        };
        assert!(validate_post("valid-slug", &invalid_title).is_err());

        let invalid_content = PostInput {
            title: "Valid Title".to_string(),
            content: "".to_string(), // Empty string
            ..Default::default()
        };
        assert!(validate_post("valid-slug", &invalid_content).is_err());
    }
}
