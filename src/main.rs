use anyhow::Result;
use axum::{extract::Path, routing::get, Json, Router};
use serde::{Deserialize, Serialize};
use tower_http::cors::{Any, CorsLayer};
use tracing::info;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/stream/:id", get(stream))
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8008").await?;
    info!("Started server on port 8008");
    axum::serve(listener, app).await?;
    Ok(())
}

#[derive(Debug, Deserialize, Serialize)]
struct StreamResponse {
    url: String,
}

async fn stream(Path(id): Path<Uuid>) -> Json<StreamResponse> {
    info!("Reading stream {id}");
    Json(StreamResponse {
        url: "https://stream.mux.com/VZtzUzGRv02OhRnZCxcNg49OilvolTqdnFLEqBsTwaxU.m3u8".to_string(),
    })
}
