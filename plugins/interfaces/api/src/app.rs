use std::sync::Arc;

use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::http::{HeaderName, Method};
use axum::middleware;
use tokio::sync::Mutex;
use tower_http::cors::{AllowOrigin, CorsLayer};

use crate::config::Settings;
use crate::middleware::request_id_middleware;
use crate::modules::v3;
use crate::state::AppState;
use crate::storage::store::Store;

/// 构建 API 应用状态。
pub async fn build_app_state(settings: Arc<Settings>, store: Store) -> AppState {
    build_app_state_with(settings, store)
}

/// 用设置与存储构建状态。
pub fn build_app_state_with(settings: Arc<Settings>, store: Store) -> AppState {
    AppState {
        settings,
        store: Arc::new(Mutex::new(store)),
    }
}

pub fn create_cors_layer(settings: &Settings) -> CorsLayer {
    if settings.cors_origins.iter().any(|origin| origin == "*") {
        CorsLayer::new()
            .allow_origin(AllowOrigin::mirror_request())
            .allow_methods([
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::PATCH,
                Method::DELETE,
                Method::OPTIONS,
            ])
            .allow_headers([
                HeaderName::from_static("content-type"),
                HeaderName::from_static("x-request-id"),
            ])
            .allow_credentials(true)
    } else {
        CorsLayer::new()
            .allow_origin(AllowOrigin::list(
                settings
                    .cors_origins
                    .iter()
                    .filter_map(|origin| origin.parse().ok()),
            ))
            .allow_methods([
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::PATCH,
                Method::DELETE,
                Method::OPTIONS,
            ])
            .allow_headers([
                HeaderName::from_static("content-type"),
                HeaderName::from_static("x-request-id"),
            ])
            .allow_credentials(true)
    }
}

pub async fn build_router(settings: Arc<Settings>, store: Store) -> Router {
    let state = build_app_state(settings, store).await;
    build_router_from_state(state)
}

pub fn build_router_from_state(state: AppState) -> Router {
    let cors = create_cors_layer(&state.settings);
    let max_upload = state
        .settings
        .max_upload_size_mb
        .saturating_mul(1024 * 1024)
        + 2 * 1024 * 1024;

    let api = Router::new()
        .nest("/api/v3", v3::router())
        .layer(middleware::from_fn(request_id_middleware))
        .with_state(state);

    api.layer(cors).layer(DefaultBodyLimit::max(max_upload))
}
