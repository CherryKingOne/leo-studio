use axum::Json;
use axum::Router;
use axum::extract::{Multipart, Path, Query, State};
use axum::routing::get;
use serde::Deserialize;

use crate::error::{AppError, AppResult};
use crate::http::page_count;
use crate::modules::projects::model::ProjectInfo;
use crate::modules::projects::multipart::read_multipart;
use crate::modules::projects::schema::{ProjectDeleteResponse, ProjectListResponse};
use crate::modules::projects::service as project_service;
use crate::response::ApiResponse;
use crate::state::AppState;

/// 项目管理路由：`/api/v3/projects`。
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_projects).post(create_project))
        .route(
            "/{project_id}",
            get(get_project)
                .patch(update_project)
                .delete(delete_project),
        )
}

#[derive(Debug, Deserialize)]
struct ProjectListQuery {
    page: Option<usize>,
    page_size: Option<usize>,
    status: Option<String>,
    keyword: Option<String>,
    sort: Option<String>,
    order: Option<String>,
}

impl ProjectListQuery {
    fn resolve(&self) -> AppResult<(usize, usize)> {
        let page = self.page.unwrap_or(1);
        let page_size = self.page_size.unwrap_or(20);
        if page < 1 {
            return Err(AppError::invalid_parameter("页码必须大于 0"));
        }
        if !(1..=100).contains(&page_size) {
            return Err(AppError::invalid_parameter("每页数量必须在 1 到 100 之间"));
        }
        Ok((page, page_size))
    }
}

async fn create_project(
    State(state): State<AppState>,
    multipart: Multipart,
) -> AppResult<Json<ApiResponse<ProjectInfo>>> {
    let body = read_multipart(multipart).await?;
    let name = body.field("name").unwrap_or("").trim();
    if name.is_empty() {
        return Err(AppError::invalid_parameter("项目名称不能为空"));
    }
    let filename = body.file_name.clone().unwrap_or_default();
    if filename.trim().is_empty() {
        return Err(AppError::invalid_parameter("上传文件不能为空"));
    }

    let mut store = state.store.lock().await;
    let info = project_service::create_project(
        &mut store,
        &state.settings,
        name,
        &filename,
        &body.file_content,
        body.file_content_type.as_deref(),
    )?;
    Ok(Json(ApiResponse::success_with_message(
        info,
        "项目创建成功",
    )))
}

async fn list_projects(
    State(state): State<AppState>,
    Query(query): Query<ProjectListQuery>,
) -> AppResult<Json<ApiResponse<ProjectListResponse>>> {
    let (page, page_size) = query.resolve()?;
    let sort = query.sort.as_deref().unwrap_or("created_at");
    let order = query.order.as_deref().unwrap_or("desc");

    let store = state.store.lock().await;
    let (items, total) = project_service::query_projects(
        &store,
        page,
        page_size,
        false,
        query.status.as_deref(),
        query.keyword.as_deref(),
        sort,
        order,
    );
    Ok(Json(ApiResponse::success(ProjectListResponse {
        total,
        page,
        page_size,
        pages: page_count(total, page_size),
        items,
    })))
}

async fn get_project(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> AppResult<Json<ApiResponse<ProjectInfo>>> {
    let store = state.store.lock().await;
    let info = project_service::get_project(&store, &project_id, false)?;
    Ok(Json(ApiResponse::success(info)))
}

#[derive(Debug, Deserialize)]
struct ProjectUpdateRequest {
    name: Option<String>,
    status: Option<String>,
}

async fn update_project(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Json(payload): Json<ProjectUpdateRequest>,
) -> AppResult<Json<ApiResponse<ProjectInfo>>> {
    let mut store = state.store.lock().await;
    let info = project_service::update_project_fields(
        &mut store,
        &project_id,
        payload.name.as_deref(),
        payload.status.as_deref(),
    )?;
    Ok(Json(ApiResponse::success_with_message(
        info,
        "项目更新成功",
    )))
}

async fn delete_project(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> AppResult<Json<ApiResponse<ProjectDeleteResponse>>> {
    let mut store = state.store.lock().await;
    let info = project_service::delete_project(&mut store, &project_id)?;
    Ok(Json(ApiResponse::success_with_message(
        ProjectDeleteResponse {
            project_id: info.project_id,
            name: info.name,
            deleted: true,
            deleted_at: info.deleted_at,
        },
        "项目已成功移入回收站",
    )))
}
