use axum::Json;
use axum::Router;
use axum::extract::{Multipart, Path, Query, State};
use axum::routing::{delete, get, post};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::error::{AppError, AppResult};
use crate::http::page_count;
use crate::modules::projects::model::{ChapterContentInfo, ProjectChapterAppendInfo, ProjectInfo};
use crate::modules::projects::schema::{
    ChapterDeleteResponse, ChapterOutlineListResponse, ChapterPolyphoneListResponse,
    ProjectClearRecycleBinResponse, ProjectDeleteResponse, ProjectHardDeleteResponse,
    ProjectListResponse, ProjectRestoreResponse, SentencePolyphoneDeleteResponse,
    SentencePolyphoneItemDeleteResponse, SentencePolyphoneListResponse,
    SentencePolyphoneSaveRequest,
};
use crate::modules::projects::service as project_service;
use crate::response::ApiResponse;
use crate::state::AppState;

/// v2 路由：`/api/v2`。
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(index))
        .route("/health", get(health))
        .nest("/projects", projects_router())
        .merge(crate::modules::exchange::router())
}

fn projects_router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_projects).post(create_project))
        .route(
            "/recycle-bin",
            get(list_recycle_bin).delete(clear_recycle_bin),
        )
        .route(
            "/{project_id}",
            get(get_project)
                .patch(update_project)
                .delete(delete_project),
        )
        .route("/{project_id}/restore", post(restore_project))
        .route("/{project_id}/hard", delete(hard_delete_project))
        .route(
            "/{project_id}/chapters",
            get(list_chapters).post(append_chapters),
        )
        .route(
            "/{project_id}/chapters/{chapter_id}",
            get(get_chapter_content).delete(delete_chapter),
        )
        .route(
            "/{project_id}/chapters/{chapter_id}/polyphones",
            get(get_chapter_polyphones),
        )
        .route(
            "/{project_id}/chapters/{chapter_id}/sentences/{sentence_id}/polyphones",
            get(get_chapter_sentence_polyphones)
                .put(save_chapter_sentence_polyphones)
                .delete(delete_chapter_sentence_polyphones),
        )
        .route(
            "/{project_id}/chapters/{chapter_id}/sentences/{sentence_id}/polyphones/{char_index}",
            delete(delete_chapter_sentence_polyphone),
        )
        .route(
            "/{project_id}/sentences/{sentence_id}/polyphones",
            get(get_sentence_polyphones)
                .put(save_sentence_polyphones)
                .delete(delete_sentence_polyphones),
        )
}

/// 从 multipart 中读取所有字段。
struct MultipartBody {
    fields: Vec<(String, String)>,
    file_name: Option<String>,
    file_content_type: Option<String>,
    file_content: Vec<u8>,
}

impl MultipartBody {
    fn field(&self, name: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }

    fn bool_field(&self, name: &str, default: bool) -> bool {
        match self.field(name) {
            Some(value) => matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            ),
            None => default,
        }
    }
}

async fn read_multipart(mut multipart: Multipart) -> AppResult<MultipartBody> {
    let mut fields = Vec::new();
    let mut file_name = None;
    let mut file_content_type = None;
    let mut file_content = Vec::new();
    let mut found_file = false;

    while let Some(mut field) = multipart
        .next_field()
        .await
        .map_err(|err| AppError::invalid_parameter(format!("表单解析失败：{err}")))?
    {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" {
            found_file = true;
            file_name = field.file_name().map(|value| value.to_string());
            file_content_type = field.content_type().map(|value| value.to_string());
            while let Ok(Some(chunk)) = field.chunk().await {
                file_content.extend_from_slice(&chunk);
            }
        } else {
            let value = field
                .text()
                .await
                .map_err(|err| AppError::invalid_parameter(format!("表单字段读取失败：{err}")))?;
            fields.push((name, value));
        }
    }

    if !found_file {
        return Err(AppError::invalid_parameter("上传文件不能为空"));
    }

    Ok(MultipartBody {
        fields,
        file_name,
        file_content_type,
        file_content,
    })
}

async fn health(State(state): State<AppState>) -> Json<Value> {
    Json(json!({
        "status": "ok",
        "app": &state.settings.app_name,
        "version": &state.settings.app_version,
    }))
}

async fn index(State(state): State<AppState>) -> Json<Value> {
    let health = health_result(&state);
    Json(json!({
        "app": health["app"],
        "version": health["version"],
        "api_prefix": "/api/v2",
        "modules": ["system", "projects", "recycle-bin", "chapters", "polyphones"],
    }))
}

fn health_result(state: &AppState) -> Value {
    json!({
        "status": "ok",
        "app": &state.settings.app_name,
        "version": &state.settings.app_version,
    })
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

    let text = state.text_pipeline().await?;
    let mut store = state.store.lock().await;
    let info = project_service::create_project(
        &mut store,
        &state.settings,
        &text,
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

async fn list_recycle_bin(
    State(state): State<AppState>,
    Query(query): Query<ProjectListQuery>,
) -> AppResult<Json<ApiResponse<ProjectListResponse>>> {
    let (page, page_size) = query.resolve()?;
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
        .filter(|v| !v.is_empty())
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

async fn get_chapter_polyphones(
    State(state): State<AppState>,
    Path((project_id, chapter_id)): Path<(String, String)>,
) -> AppResult<Json<ApiResponse<ChapterPolyphoneListResponse>>> {
    let store = state.store.lock().await;
    let info = project_service::get_chapter_polyphones(&store, &project_id, &chapter_id)?;
    Ok(Json(ApiResponse::success(info)))
}

async fn get_chapter_sentence_polyphones(
    State(state): State<AppState>,
    Path((project_id, chapter_id, sentence_id)): Path<(String, String, String)>,
) -> AppResult<Json<ApiResponse<SentencePolyphoneListResponse>>> {
    let store = state.store.lock().await;
    let items = project_service::get_chapter_sentence_polyphones(
        &store,
        &project_id,
        &chapter_id,
        &sentence_id,
    )?;
    let total = items.len();
    Ok(Json(ApiResponse::success(SentencePolyphoneListResponse {
        sentence_id,
        total,
        items,
    })))
}

async fn save_chapter_sentence_polyphones(
    State(state): State<AppState>,
    Path((project_id, chapter_id, sentence_id)): Path<(String, String, String)>,
    Json(payload): Json<SentencePolyphoneSaveRequest>,
) -> AppResult<Json<ApiResponse<SentencePolyphoneListResponse>>> {
    let mut store = state.store.lock().await;
    let items = project_service::save_chapter_sentence_polyphones(
        &mut store,
        &project_id,
        &chapter_id,
        &sentence_id,
        &payload.annotations,
    )?;
    let total = items.len();
    Ok(Json(ApiResponse::success_with_message(
        SentencePolyphoneListResponse {
            sentence_id,
            total,
            items,
        },
        "多音字标注保存成功",
    )))
}

async fn delete_chapter_sentence_polyphones(
    State(state): State<AppState>,
    Path((project_id, chapter_id, sentence_id)): Path<(String, String, String)>,
) -> AppResult<Json<ApiResponse<SentencePolyphoneDeleteResponse>>> {
    let mut store = state.store.lock().await;
    let deleted_count = project_service::delete_chapter_sentence_polyphones(
        &mut store,
        &project_id,
        &chapter_id,
        &sentence_id,
    )?;
    Ok(Json(ApiResponse::success_with_message(
        SentencePolyphoneDeleteResponse {
            sentence_id,
            deleted_count,
            deleted: true,
        },
        format!("已成功清空多音字标注，共删除 {deleted_count} 条"),
    )))
}

async fn delete_chapter_sentence_polyphone(
    State(state): State<AppState>,
    Path((project_id, chapter_id, sentence_id, char_index)): Path<(String, String, String, i64)>,
) -> AppResult<Json<ApiResponse<SentencePolyphoneItemDeleteResponse>>> {
    let mut store = state.store.lock().await;
    let info = project_service::delete_chapter_sentence_polyphone(
        &mut store,
        &project_id,
        &chapter_id,
        &sentence_id,
        char_index,
    )?;
    Ok(Json(ApiResponse::success_with_message(
        info,
        "多音字标注删除成功",
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

    let text = state.text_pipeline().await?;
    let mut store = state.store.lock().await;
    let info = project_service::append_chapters(
        &mut store,
        &state.settings,
        &text,
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

async fn get_sentence_polyphones(
    State(state): State<AppState>,
    Path((project_id, sentence_id)): Path<(String, String)>,
) -> AppResult<Json<ApiResponse<SentencePolyphoneListResponse>>> {
    let store = state.store.lock().await;
    let items = project_service::get_sentence_polyphones(&store, &project_id, &sentence_id)?;
    let total = items.len();
    Ok(Json(ApiResponse::success(SentencePolyphoneListResponse {
        sentence_id,
        total,
        items,
    })))
}

async fn save_sentence_polyphones(
    State(state): State<AppState>,
    Path((project_id, sentence_id)): Path<(String, String)>,
    Json(payload): Json<SentencePolyphoneSaveRequest>,
) -> AppResult<Json<ApiResponse<SentencePolyphoneListResponse>>> {
    let mut store = state.store.lock().await;
    let items = project_service::save_sentence_polyphones(
        &mut store,
        &project_id,
        &sentence_id,
        &payload.annotations,
    )?;
    let total = items.len();
    Ok(Json(ApiResponse::success_with_message(
        SentencePolyphoneListResponse {
            sentence_id,
            total,
            items,
        },
        "多音字标注保存成功",
    )))
}

async fn delete_sentence_polyphones(
    State(state): State<AppState>,
    Path((project_id, sentence_id)): Path<(String, String)>,
) -> AppResult<Json<ApiResponse<SentencePolyphoneDeleteResponse>>> {
    let mut store = state.store.lock().await;
    let deleted_count =
        project_service::delete_sentence_polyphones(&mut store, &project_id, &sentence_id)?;
    Ok(Json(ApiResponse::success_with_message(
        SentencePolyphoneDeleteResponse {
            sentence_id,
            deleted_count,
            deleted: true,
        },
        format!("已成功清空多音字标注，共删除 {deleted_count} 条"),
    )))
}
