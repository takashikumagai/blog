use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder};

use crate::error::AppError;

pub mod app;
pub mod auth;
pub mod db;
pub mod error;
pub mod models;

#[derive(Debug, Default, Deserialize)]
pub struct ProductFilters {
    pub vehicle_type: Option<String>,
    pub max_weight_kg: Option<f64>,
    pub min_rider_height_cm: Option<f64>,
    pub max_rider_height_cm: Option<f64>,
    pub foldable: Option<bool>,
    pub min_motor_nominal_power_w: Option<f64>,
    pub max_motor_nominal_power_w: Option<f64>,
    pub min_wheel_size_mm: Option<i32>,
    pub max_wheel_size_mm: Option<i32>,
    pub battery_removable: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct Pagination {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

impl Default for Pagination {
    fn default() -> Self {
        Self {
            limit: default_limit(),
            offset: 0,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

fn default_limit() -> i64 {
    24
}

fn normalize_pagination(pagination: &Pagination) -> (i64, i64) {
    let limit = pagination.limit.clamp(1, 100);
    let offset = pagination.offset.max(0);
    (limit, offset)
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
        .with_state(app::AppState { db })
}

pub async fn health_check(State(state): State<app::AppState>) -> impl IntoResponse {
    let database_ok = sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.db)
        .await
        .is_ok();

    let status = if database_ok { "ok" } else { "error" };
    let database = if database_ok {
        "connected"
    } else {
        "unreachable"
    };
    let status_code = if database_ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (
        status_code,
        Json(serde_json::json!({
            "status": status,
            "database": database
        })),
    )
}
