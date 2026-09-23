use axum::{Router, routing::get, serve};
use std::sync::Arc;

use tokio::net::TcpListener;
use tracing_subscriber::prelude::*;

mod discord;
mod down;
mod lease;
mod sample;
mod state;

#[tokio::main]
async fn main() {
    // .envファイルを読み込み（エラーは無視）
    let _ = dotenvy::dotenv();

    // トレーシングの初期化
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "pos-down-detector=debug,tower_http=debug,axum=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    discord::register_commands().await;

    let state = Arc::new(state::AppState {
        session_store: std::sync::Mutex::new(None),
    });

    let app = Router::new()
        .route(
            "/",
            get(async || "POS Down Detector and Lease Management API"),
        )
        .nest("/sample", sample::api_router(Arc::clone(&state)))
        .nest("/discord", discord::api_router(Arc::clone(&state)))
        .nest("/lease", lease::api_router(Arc::clone(&state)));

    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "34000".to_string())
        .parse::<u16>()
        .expect("PORT must be a valid u16");

    let listener: TcpListener = TcpListener::bind(format!("{}:{}", host, port))
        .await
        .unwrap();
    tracing::info!(
        "Server running at http://{}:{}",
        listener.local_addr().unwrap().ip(),
        listener.local_addr().unwrap().port()
    );
    serve(listener, app).await.unwrap();
}
