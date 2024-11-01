use anyhow::Result;
use axum::{routing::get, Router};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let app = Router::new().route("/", get(|| async { "Hello, World!" }));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8008").await?;
    info!("Started server on port 8008");
    axum::serve(listener, app).await?;

    Ok(())
}
