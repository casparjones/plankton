// Zentraler Anwendungs-State.

use std::{collections::HashMap, sync::Arc};
use tokio::sync::{broadcast, Mutex};

use crate::store::DataStore;

/// Zentraler Anwendungs-State, der von Axum in alle Handler injiziert wird.
#[derive(Clone)]
pub struct AppState {
    pub store: DataStore,
    pub events: Arc<Mutex<HashMap<String, broadcast::Sender<String>>>>,
    pub jwt_secret: String,
}
