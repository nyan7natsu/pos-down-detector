pub struct LeaseObject {
    pub lease_id: String,
    pub expiration_time: std::time::Instant,
}

pub struct AppState {
    pub session_store: std::sync::Mutex<Option<LeaseObject>>,
}
