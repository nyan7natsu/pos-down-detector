use std::collections::HashMap;

pub struct LeaseObject {
    pub lease_id: String,
    pub lease_label: String,
    pub expiration_time: std::time::Instant,
}

pub struct AppState {
    pub session_store: std::sync::Mutex<HashMap<String, LeaseObject>>,
}
