use std::env;
use std::net::SocketAddr;

use blog::{app_with_db, db};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "nyanko_blog=debug,axum=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let _ = dotenvy::dotenv();

    let db = db::connect().await?;
    db::run_migrations(&db).await?;

    let app = app_with_db(db.clone()).await;

    let port = env::var("BNS_PORT")
        .unwrap_or_else(|_| "3210".to_string())
        .parse::<u16>()
        .expect("BNS_PORT must be a valid number");

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!(%addr, "starting backend server");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
