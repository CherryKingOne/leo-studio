use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::routing::get;
use serde_json::{Value, json};

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/system/health", get(system_health))
}

pub fn root_router() -> Router<AppState> {
    Router::new()
        .route("/", get(root))
        .route("/health", get(health))
}

async fn health(State(state): State<AppState>) -> Json<Value> {
    Json(health_result(&state))
}

async fn system_health(State(state): State<AppState>) -> Json<Value> {
    Json(health_result(&state))
}

async fn root(State(state): State<AppState>) -> Json<Value> {
    let health = health_result(&state);
    Json(json!({
        "app": health["app"],
        "version": health["version"],
        "docs": "/docs",
        "health": "/health",
        "api_prefix": "/api/v1",
        "modules": ["text-files", "projects", "system"],
    }))
}

fn health_result(state: &AppState) -> Value {
    json!({
        "status": "ok",
        "app": &state.settings.app_name,
        "version": &state.settings.app_version,
    })
}
