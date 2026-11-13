use std::sync::Arc;

use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::http::{HeaderName, Method};
use axum::middleware;
use tokio::sync::Mutex;
use tower_http::cors::{AllowOrigin, CorsLayer};

use crate::config::Settings;
use crate::middleware::request_id_middleware;
use crate::modules::{projects, system, text_files};
use crate::state::AppState;
use crate::storage::store::Store;

pub fn build_router(settings: Arc<Settings>, store: Store) -> Router {
    let state = AppState {
        settings: settings.clone(),
        store: Arc::new(Mutex::new(store)),
    };

    let cors = if settings.cors_origins.iter().any(|origin| origin == "*") {
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
    };

    Router::new()
        .nest("/api/v1/text-files", text_files::router())
        .nest("/api/v1/projects", projects::router())
        .nest("/api/v1", system::router())
        .merge(system::root_router())
        .layer(middleware::from_fn(request_id_middleware))
        .layer(cors)
        .layer(DefaultBodyLimit::max(
            settings.max_upload_size_mb.saturating_mul(1024 * 1024) + 2 * 1024 * 1024,
        ))
        .with_state(state)
}
