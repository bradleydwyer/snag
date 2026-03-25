use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use axum::Router;
use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Json, Response};
use axum::routing::{get, post};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tower_http::services::ServeDir;

use crate::{encode, pipeline};

struct AppState {
    video_dir: PathBuf,
    gif_dir: PathBuf,
    video_path: Mutex<Option<PathBuf>>,
    stream_url: Mutex<Option<String>>,
    width: u32,
    fps: u32,
}

#[derive(Deserialize)]
struct ResolveRequest {
    url: String,
}

#[derive(Serialize)]
struct ResolveResponse {
    video_url: String,
    duration: f64,
}

#[derive(Deserialize)]
struct EncodeRequest {
    start: f64,
    end: f64,
    width: Option<u32>,
    fps: Option<u32>,
}

#[derive(Serialize)]
struct EncodeResponse {
    gif_url: String,
    file_size: u64,
}

async fn index() -> impl IntoResponse {
    let html = include_str!("../web/index.html");
    Response::builder()
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .body(html.to_string())
        .unwrap()
}

async fn serve_gif(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(filename): axum::extract::Path<String>,
) -> impl IntoResponse {
    let path = state.gif_dir.join(&filename);
    match tokio::fs::read(&path).await {
        Ok(bytes) => Response::builder()
            .header(header::CONTENT_TYPE, "image/gif")
            .body(axum::body::Body::from(bytes))
            .unwrap(),
        Err(_) => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(axum::body::Body::from("not found"))
            .unwrap(),
    }
}

async fn resolve(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ResolveRequest>,
) -> Result<Json<ResolveResponse>, (StatusCode, String)> {
    let url = req.url.clone();
    let video_dir = state.video_dir.clone();

    let result = tokio::task::spawn_blocking(move || -> anyhow::Result<(String, f64, PathBuf)> {
        let info = pipeline::resolve_stream(&url)?;
        let video_path = pipeline::download_video(&url, &video_dir)?;
        Ok((info.stream_url, info.duration_secs, video_path))
    })
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let (stream_url, duration, video_path) = result;

    *state.video_path.lock().await = Some(video_path);
    *state.stream_url.lock().await = Some(stream_url);

    Ok(Json(ResolveResponse {
        video_url: "/video/video.mp4".to_string(),
        duration,
    }))
}

async fn encode_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<EncodeRequest>,
) -> Result<Json<EncodeResponse>, (StatusCode, String)> {
    let video_path = state
        .video_path
        .lock()
        .await
        .clone()
        .ok_or((StatusCode::BAD_REQUEST, "no video resolved yet".to_string()))?;

    let width = req.width.unwrap_or(state.width);
    let fps = req.fps.unwrap_or(state.fps);
    let output_path = state.gif_dir.join("output.gif");
    let output_str = output_path
        .to_str()
        .ok_or((
            StatusCode::INTERNAL_SERVER_ERROR,
            "invalid path".to_string(),
        ))?
        .to_string();
    let video_str = video_path
        .to_str()
        .ok_or((
            StatusCode::INTERNAL_SERVER_ERROR,
            "invalid video path".to_string(),
        ))?
        .to_string();

    let start = req.start;
    let end = req.end;

    tokio::task::spawn_blocking(move || {
        encode::encode_gif(&video_str, start, end, width, fps, &output_str)
    })
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let metadata = std::fs::metadata(&output_path)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(EncodeResponse {
        gif_url: "/gif/output.gif".to_string(),
        file_size: metadata.len(),
    }))
}

pub async fn run(port: u16, width: u32, fps: u32) -> Result<()> {
    let video_dir = tempfile::tempdir().context("failed to create video temp dir")?;
    let gif_dir = tempfile::tempdir().context("failed to create gif temp dir")?;

    let video_dir_path = video_dir.path().to_path_buf();
    let gif_dir_path = gif_dir.path().to_path_buf();

    let state = Arc::new(AppState {
        video_dir: video_dir_path.clone(),
        gif_dir: gif_dir_path.clone(),
        video_path: Mutex::new(None),
        stream_url: Mutex::new(None),
        width,
        fps,
    });

    let app = Router::new()
        .route("/", get(index))
        .route("/gif/{filename}", get(serve_gif))
        .route("/api/resolve", post(resolve))
        .route("/api/encode", post(encode_handler))
        .nest_service("/video", ServeDir::new(&video_dir_path))
        .with_state(state);

    let addr = format!("0.0.0.0:{port}");
    eprintln!("Listening on http://localhost:{port}");

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .context(format!("failed to bind to {addr}"))?;

    // Keep temp dirs alive for the lifetime of the server
    let _video_dir = video_dir;
    let _gif_dir = gif_dir;

    axum::serve(listener, app).await.context("server error")?;

    Ok(())
}
