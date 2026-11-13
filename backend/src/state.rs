use std::sync::Arc;

use tokio::sync::Mutex;

use crate::config::Settings;
use crate::storage::store::Store;

/// 应用共享状态。
#[derive(Clone)]
pub struct AppState {
    pub settings: Arc<Settings>,
    pub store: Arc<Mutex<Store>>,
}
