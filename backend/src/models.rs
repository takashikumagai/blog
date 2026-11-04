use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Post {
    pub id: String,
    pub title: String,
    pub content: String,
    pub genre: Option<String>,
    pub excerpt: Option<String>,
    pub published: bool,
    pub published_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct PostSummary {
    pub id: String,
    pub title: String,
    pub genre: Option<String>,
    pub excerpt: Option<String>,
    pub published: bool,
    pub published_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
