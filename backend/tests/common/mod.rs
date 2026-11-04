use axum::Router;
use sqlx::PgPool;
use nyanko_blog::app_with_db; // Replace with your actual crate name

pub const TEST_TOKEN: &str = "super-secret-test-token";

/// Standardizes app initialization for integration tests
pub async fn spawn_test_app(pool: PgPool) -> Router {
    // Configure the environment token required by authorize_sync
    std::env::set_var("BLOG_SYNC_TOKEN", TEST_TOKEN);
    
    // Build and return the router bound to your isolated sqlx sandbox pool
    app_with_db(pool).await
}
