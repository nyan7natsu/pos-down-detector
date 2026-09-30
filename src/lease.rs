// /lease

use crate::state::AppState;
use axum::{Json, Router, extract::State, http::StatusCode};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::sync::Arc;

#[derive(Deserialize)]
struct LeaseRequest {
    lease_id: String,
    lease_label: String,
}

async fn lease_acquire(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LeaseRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let lease_id = hex::encode(Sha256::digest(payload.lease_id.as_bytes()));

    let mut session_store = state.session_store.lock().unwrap();

    if session_store.is_none() {
        // リースがないので新規取得
        let expiration_time = std::time::Instant::now() + std::time::Duration::from_secs(30);
        *session_store = Some(crate::state::LeaseObject {
            lease_id: lease_id,
            lease_label: payload.lease_label,
            expiration_time,
        });
        return Ok(StatusCode::OK);
    }

    let existing_lease = session_store.as_ref().unwrap();

    if existing_lease.lease_id == lease_id {
        // リース更新
        let expiration_time = std::time::Instant::now() + std::time::Duration::from_secs(30);
        *session_store = Some(crate::state::LeaseObject {
            lease_id: lease_id,
            lease_label: payload.lease_label,
            expiration_time,
        });
        return Ok(StatusCode::OK);
    } else {
        // 別のデバイスによってリースされている
        return Err((StatusCode::LOCKED, lease_id));
    }
}

async fn lease_release(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LeaseRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let lease_id = hex::encode(Sha256::digest(payload.lease_id.as_bytes()));

    let mut session_store = state.session_store.lock().unwrap();

    if session_store.is_none() {
        return Err((StatusCode::NOT_FOUND, "No lease to release".to_string()));
    }

    let existing_lease = session_store.as_ref().unwrap();

    if existing_lease.lease_id == lease_id {
        // リースを解放
        *session_store = None;
        return Ok(StatusCode::OK);
    } else {
        // 別のデバイスによってリースされている
        return Err((StatusCode::LOCKED, existing_lease.lease_id.clone()));
    }
}

async fn lease_status(State(state): State<Arc<AppState>>) -> String {
    {
        state
            .session_store
            .lock()
            .unwrap()
            .as_ref()
            .map(|lease| {
                format!(
                    "Lease ID (hash): {:?}\nLease Label: {:?}\nExpiration Time: {:?}",
                    lease.lease_id, lease.lease_label, lease.expiration_time
                )
            })
            .unwrap_or_else(|| "No lease information available.".to_string())
    }
}

pub fn api_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/status", axum::routing::get(lease_status))
        .route("/acquire", axum::routing::post(lease_acquire))
        .route("/release", axum::routing::post(lease_release))
        .with_state(state)
}
