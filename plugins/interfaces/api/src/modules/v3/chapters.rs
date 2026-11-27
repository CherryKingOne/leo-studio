use axum::Json;
use axum::Router;
use axum::extract::{Multipart, Path, Query, State};
use axum::routing::get;
use serde::Deserialize;

use crate::error::{AppError, AppResult};
use crate::modules::projects::model::{ChapterContentInfo, ProjectChapterAppendInfo};
use crate::modules::projects::multipart::read_multipart;
use crate::modules::projects::schema::{ChapterDeleteResponse, ChapterOutlineListResponse};
use crate::modules::projects::service as project_service;
use crate::response::ApiResponse;
use crate::state::AppState;

/// 项目章节管理路由：`/api/v3/projects/{project_id}/chapters`。
pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/projects/{project_id}/chapters",
            get(list_chapters).post(append_chapters),
        )
        .route(
            "/projects/{project_id}/chapters/{chapter_id}",
            get(get_chapter_content).delete(delete_chapter),
        )
}

#[derive(Debug, Deserialize)]
struct ChapterListQuery {
    keyword: Option<String>,
}

async fn list_chapters(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Query(query): Query<ChapterListQuery>,
) -> AppResult<Json<ApiResponse<ChapterOutlineListResponse>>> {
    let store = state.store.lock().await;
    let mut items = project_service::get_project_chapters(&store, &project_id)?;
    if let Some(keyword) = query
        .keyword
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        items.retain(|item| item.chapter_title.contains(keyword));
    }
    let total = items.len();
    Ok(Json(ApiResponse::success(ChapterOutlineListResponse {
        project_id,
        total,
        items,
    })))
}

async fn get_chapter_content(
    State(state): State<AppState>,
    Path((project_id, chapter_id)): Path<(String, String)>,
) -> AppResult<Json<ApiResponse<ChapterContentInfo>>> {
    let store = state.store.lock().await;
    let info = project_service::get_project_chapter_content(&store, &project_id, &chapter_id)?;
    Ok(Json(ApiResponse::success(info)))
}

async fn delete_chapter(
    State(state): State<AppState>,
    Path((project_id, chapter_id)): Path<(String, String)>,
) -> AppResult<Json<ApiResponse<ChapterDeleteResponse>>> {
    let mut store = state.store.lock().await;
    let info = project_service::delete_chapter(&mut store, &project_id, &chapter_id)?;
    Ok(Json(ApiResponse::success_with_message(
        info,
        "章节删除成功",
    )))
}

async fn append_chapters(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    multipart: Multipart,
) -> AppResult<Json<ApiResponse<ProjectChapterAppendInfo>>> {
    let body = read_multipart(multipart).await?;
    let filename = body.file_name.clone().unwrap_or_default();
    if filename.trim().is_empty() {
        return Err(AppError::invalid_parameter("上传文件不能为空"));
    }

    let overwrite_existing = body.bool_field("overwrite_existing", false);
    let dry_run = body.bool_field("dry_run", false);

    let mut store = state.store.lock().await;
    let info = project_service::append_chapters(
        &mut store,
        &state.settings,
        &project_id,
        &filename,
        &body.file_content,
        body.file_content_type.as_deref(),
        overwrite_existing,
        dry_run,
    )?;

    let message = if dry_run {
        format!(
            "预检完成（未写入）：预计新增 {} 章，跳过 {} 章，冲突 {} 处",
            info.added_chapters,
            info.skipped_chapters,
            info.conflicts.len()
        )
    } else {
        format!(
            "章节追加成功，新增 {} 章，跳过 {} 章",
            info.added_chapters, info.skipped_chapters
        )
    };

    Ok(Json(ApiResponse::success_with_message(info, message)))
}
