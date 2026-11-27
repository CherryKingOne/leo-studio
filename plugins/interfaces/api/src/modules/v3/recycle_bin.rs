use axum::Json;
use axum::Router;
use axum::extract::{Path, Query, State};
use axum::routing::{delete, get, post};

use crate::error::AppResult;
use crate::http::{Pagination, page_count};
use crate::modules::projects::schema::{
    ProjectClearRecycleBinResponse, ProjectHardDeleteResponse, ProjectListResponse,
    ProjectRestoreResponse,
};
use crate::modules::projects::service as project_service;
use crate::response::ApiResponse;
use crate::state::AppState;

/// 回收站管理路由：`/api/v3/recycle-bin`。
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_recycle_bin).delete(clear_recycle_bin))
        .route("/{project_id}/restore", post(restore_project))
        .route("/{project_id}", delete(hard_delete_project))
}

async fn list_recycle_bin(
    State(state): State<AppState>,
    Query(pagination): Query<Pagination>,
) -> AppResult<Json<ApiResponse<ProjectListResponse>>> {
    let (page, page_size) = pagination.resolve()?;
    let store = state.store.lock().await;
    let (items, total) = project_service::list_projects(&store, page, page_size, true);
    Ok(Json(ApiResponse::success(ProjectListResponse {
        total,
        page,
        page_size,
        pages: page_count(total, page_size),
        items,
    })))
}

async fn clear_recycle_bin(
    State(state): State<AppState>,
) -> AppResult<Json<ApiResponse<ProjectClearRecycleBinResponse>>> {
    let mut store = state.store.lock().await;
    let cleared_count = project_service::clear_recycle_bin(&mut store)?;
    Ok(Json(ApiResponse::success_with_message(
        ProjectClearRecycleBinResponse {
            cleared_count,
            cleared: true,
        },
        format!("回收站已清空，共彻底删除 {cleared_count} 个项目"),
    )))
}

async fn restore_project(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> AppResult<Json<ApiResponse<ProjectRestoreResponse>>> {
    let mut store = state.store.lock().await;
    let info = project_service::restore_project(&mut store, &project_id)?;
    Ok(Json(ApiResponse::success_with_message(
        ProjectRestoreResponse {
            project_id: info.project_id,
            name: info.name,
            restored: true,
        },
        "项目已成功恢复",
    )))
}

async fn hard_delete_project(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> AppResult<Json<ApiResponse<ProjectHardDeleteResponse>>> {
    let mut store = state.store.lock().await;
    let info = project_service::hard_delete_project(&mut store, &project_id)?;
    Ok(Json(ApiResponse::success_with_message(
        ProjectHardDeleteResponse {
            project_id: info.project_id,
            name: info.name,
            hard_deleted: true,
        },
        "项目已彻底物理删除",
    )))
}
