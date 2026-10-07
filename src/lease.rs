// /lease

use crate::state::AppState;
use axum::{Json, Router, extract::Query, extract::State, http::StatusCode};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::sync::Arc;

#[derive(Deserialize)]
struct LeaseRequest {
    service_id: String,
    lease_id: String,
    lease_label: String,
}

#[derive(Deserialize)]
struct StatusLeaseQuery {
    service_id: String,
}

async fn lease_acquire(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LeaseRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let service_id = payload.service_id.clone();
    let lease_id = hex::encode(Sha256::digest(payload.lease_id.as_bytes()));

    let mut session_store = state.session_store.lock().unwrap();

    if session_store.get(&service_id).is_none() {
        // リースがないので新規取得
        let expiration_time = std::time::Instant::now() + std::time::Duration::from_secs(30);
        session_store.insert(
            service_id,
            crate::state::LeaseObject {
                lease_id: lease_id,
                lease_label: payload.lease_label,
                expiration_time,
            },
        );
        return Ok(StatusCode::OK);
    }

    let existing_lease = session_store.get_mut(&service_id).unwrap();

    if existing_lease.lease_id == lease_id {
        // リース更新
        let expiration_time = std::time::Instant::now() + std::time::Duration::from_secs(30);
        existing_lease.expiration_time = expiration_time;
        return Ok(StatusCode::OK);
    } else {
        let lease_expired = existing_lease.expiration_time <= std::time::Instant::now();
        if lease_expired {
            // リースが期限切れなので新規取得
            let expiration_time = std::time::Instant::now() + std::time::Duration::from_secs(30);
            *existing_lease = crate::state::LeaseObject {
                lease_id: lease_id,
                lease_label: payload.lease_label,
                expiration_time,
            };
            return Ok(StatusCode::OK);
        }

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

    if session_store.get(&payload.service_id).is_none() {
        return Err((StatusCode::NOT_FOUND, "No lease to release".to_string()));
    }

    let existing_lease = session_store.get(&payload.service_id).unwrap();

    if existing_lease.lease_id == lease_id {
        // リースを解放
        session_store.remove(&payload.service_id);
        return Ok(StatusCode::OK);
    } else {
        let lease_expired = existing_lease.expiration_time <= std::time::Instant::now();
        if lease_expired {
            // リースが期限切れなので解放扱い
            session_store.remove(&payload.service_id);
            return Ok(StatusCode::OK);
        }

        // 別のデバイスによってリースされている
        return Err((StatusCode::LOCKED, existing_lease.lease_id.clone()));
    }
}

async fn lease_status(
    State(state): State<Arc<AppState>>,
    Query(query): Query<StatusLeaseQuery>,
) -> String {
    {
        let id = query.service_id.clone();

        let session_store = state.session_store.lock().unwrap();

        if let Some(existing_lease) = session_store.get(&id) {
            if existing_lease.expiration_time > std::time::Instant::now() {
                return format!(
                    "Lease is active: {}\nExpired in: {}",
                    existing_lease.lease_label,
                    existing_lease
                        .expiration_time
                        .duration_since(std::time::Instant::now())
                        .as_secs()
                );
            } else {
                return "Lease has expired".to_string();
            }
        } else {
            return "No lease found".to_string();
        }
    }
}

pub fn api_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/status", axum::routing::get(lease_status))
        .route("/acquire", axum::routing::post(lease_acquire))
        .route("/release", axum::routing::post(lease_release))
        .with_state(state)
}
