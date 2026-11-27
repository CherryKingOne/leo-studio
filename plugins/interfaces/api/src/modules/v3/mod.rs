pub mod chapters;
pub mod characters;
pub mod dialogues;
pub mod polyphones;
pub mod projects;
pub mod recycle_bin;

use axum::Router;

use crate::state::AppState;

/// v3 路由：`/api/v3`。
pub fn router() -> Router<AppState> {
    Router::new()
        .nest("/projects", projects::router())
        .merge(chapters::router())
        .merge(characters::router())
        .merge(dialogues::router())
        .merge(polyphones::router())
        .nest("/recycle-bin", recycle_bin::router())
}
