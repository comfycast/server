use std::path::PathBuf;

use anyhow::Result;
use axum::{extract::Path, http::header, response::IntoResponse, routing::get, Json, Router};
use fancy_ffmpeg::async_ffprobe;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use tokio::fs;
use tower::ServiceBuilder;
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    decompression::RequestDecompressionLayer,
};
use tracing::info;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/videos", get(videos))
        .route("/stream/create/:video", get(create_stream))
        .route("/stream/:id/master.m3u8", get(stream))
        .route("/stream/:id/:segment.ts", get(generate_segment))
        .layer(
            ServiceBuilder::new()
                .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any))
                .layer(RequestDecompressionLayer::new())
                .layer(CompressionLayer::new()),
        );

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8008").await?;
    info!("Started server on port 8008");
    axum::serve(listener, app).await?;
    Ok(())
}

#[derive(Debug, Deserialize, Serialize)]
struct Video {
    name: String,
    id: String,
}

async fn videos() -> Json<Vec<Video>> {
    let num = 30usize;
    let mut list = Vec::with_capacity(10);

    for e in 0..num {
        list.push(Video {
            name: format!("A very cool video number {e}"),
            id: format!("real-id-{e}"),
        });
    }

    Json(list)
}

#[derive(Debug, Deserialize, Serialize)]
struct CreateResponse {
    id: Uuid,
}

async fn create_stream(Path(video): Path<String>) -> Json<CreateResponse> {
    info!("Creating stream for video {video}");
    Json(CreateResponse { id: Uuid::new_v4() })
}

const SEGMENT_DURATION: u32 = 6;
const INPUT_PATH: &str = "temp/BigBuckBunny.mp4";

async fn stream(Path(id): Path<Uuid>) -> impl IntoResponse {
    let probe = async_ffprobe(INPUT_PATH).await.unwrap();
    let total_segments = (probe.format.duration / SEGMENT_DURATION as f64).ceil() as u32;

    let segments = (0..total_segments).into_iter().format_with("\n", |i, f| {
        f(&format_args!("#EXTINF:{SEGMENT_DURATION}.0,\n{i}.ts"))
    });

    let playlist = format!(
        r#"#EXTM3U
#EXT-X-VERSION:3
#EXT-X-TARGETDURATION:{SEGMENT_DURATION}
#EXT-X-MEDIA-SEQUENCE:0
{segments}
#EXT-X-ENDLIST
"#,
    );

    (
        [(header::CONTENT_TYPE, "application/vnd.apple.mpegurl")],
        playlist,
    )
}

async fn generate_segment(Path((stream_id, segment)): Path<(Uuid, String)>) -> impl IntoResponse {
    println!("Generating segment {segment}");

    let cache_dir = PathBuf::from("temp/cache").join(stream_id.to_string());
    fs::create_dir_all(&cache_dir).await.unwrap();

    let segment_number: u32 = segment.strip_suffix(".ts").unwrap().parse().unwrap();

    let segment_path = cache_dir.join(format!("segment_{}.ts", segment_number));
    if !segment_path.exists() {
        let start_time = segment_number * SEGMENT_DURATION;

        let output = tokio::process::Command::new("ffmpeg")
            .args([
                "-i",
                INPUT_PATH,
                "-ss",
                &start_time.to_string(),
                "-t",
                &SEGMENT_DURATION.to_string(),
                "-c:v",
                "libx264",
                "-c:a",
                "aac",
                "-map",
                "0",
                "-f",
                "mpegts",
                "-copyts",
                "-avoid_negative_ts",
                "make_zero",
                "-y",
            ])
            .arg(&segment_path)
            .output()
            .await
            .unwrap();

        // if !output.status.success() {
        //     // FIXME: handle ffmpeg errors
        //     println!("{}", String::from_utf8_lossy(&output.stderr));
        //     panic!("aaaaaaaaa");
        // }

        // println!("{}", String::from_utf8_lossy(&output.stdout));
        // println!("{}", String::from_utf8_lossy(&output.stderr));
    }

    let segment = fs::read(segment_path).await.unwrap();

    ([(header::CONTENT_TYPE, "video/mp2t")], segment)
}

// ffmpeg -i temp/BigBuckBunny.mp4 \
//        -ss 30 \
//        -t 6 \
//        -copyts \
//        -avoid_negative_ts make_zero \
//        -c:v libx264 \
//        -preset ultrafast \
//        -tune zerolatency \
//        -profile:v baseline \
//        -x264opts no-scenecut:keyint=60:min-keyint=60 \
//         -maxrate 3000k \
//         -bufsize 6000k \
//         -c:a aac \
//         -ac 2 \
//         -ar 44100 \
//         -b:a 128k \
//        -vsync cfr \
//        -copyinkf \
//        -individual_header_trailer 0 \
//        -flush_packets 1 \
//        -fflags +genpts \
//        -hls_segment_type mpegts \
//        -hls_flags single_file \
//        -f segment \
//        "segment_%d.ts"
