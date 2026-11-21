use axum::Json;
use axum::Router;
use axum::extract::{Multipart, Path, Query, State};
use axum::routing::{delete, get, post};

use crate::error::{AppError, AppResult};
use crate::http::{Pagination, page_count};
use crate::response::ApiResponse;
use crate::state::AppState;

use super::model::{ChapterContentInfo, ProjectChapterAppendInfo, ProjectInfo};
use super::schema::{
    ChapterOutlineListResponse, ProjectClearRecycleBinResponse, ProjectDeleteResponse,
    ProjectHardDeleteResponse, ProjectListResponse, ProjectRestoreResponse, ProjectUpdateRequest,
    SentencePolyphoneDeleteResponse, SentencePolyphoneListResponse, SentencePolyphoneSaveRequest,
};
use super::service;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_projects).post(create_project))
        .route("/upload", post(create_project))
        .route(
            "/recycle-bin",
            get(list_recycle_bin_projects).delete(clear_recycle_bin),
        )
        .route(
            "/{project_id}",
            get(get_project)
                .patch(update_project)
                .put(update_project)
                .delete(delete_project),
        )
        .route("/{project_id}/restore", post(restore_project))
        .route("/{project_id}/hard", delete(hard_delete_project))
        .route(
            "/{project_id}/chapters",
            get(list_project_chapters).post(append_project_chapters),
        )
        .route(
            "/{project_id}/chapters/upload",
            post(append_project_chapters),
        )
        .route(
            "/{project_id}/chapters/{chapter_id}",
            get(get_project_chapter_content),
        )
        .route(
            "/{project_id}/chapters/{chapter_id}/content",
            get(get_project_chapter_content),
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

/// 创建项目并上传小说。
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
    let info = service::create_project(
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

/// 分页查询项目列表。
async fn list_projects(
    State(state): State<AppState>,
    Query(pagination): Query<Pagination>,
) -> AppResult<Json<ApiResponse<ProjectListResponse>>> {
    let (page, page_size) = pagination.resolve()?;
    let store = state.store.lock().await;
    let (items, total) = service::list_projects(&store, page, page_size, false);
    Ok(Json(ApiResponse::success(ProjectListResponse {
        total,
        page,
        page_size,
        pages: page_count(total, page_size),
        items,
    })))
}

/// 查询回收站项目列表。
async fn list_recycle_bin_projects(
    State(state): State<AppState>,
    Query(pagination): Query<Pagination>,
) -> AppResult<Json<ApiResponse<ProjectListResponse>>> {
    let (page, page_size) = pagination.resolve()?;
    let store = state.store.lock().await;
    let (items, total) = service::list_projects(&store, page, page_size, true);
    Ok(Json(ApiResponse::success(ProjectListResponse {
        total,
        page,
        page_size,
        pages: page_count(total, page_size),
        items,
    })))
}

/// 清空回收站。
async fn clear_recycle_bin(
    State(state): State<AppState>,
) -> AppResult<Json<ApiResponse<ProjectClearRecycleBinResponse>>> {
    let mut store = state.store.lock().await;
    let cleared_count = service::clear_recycle_bin(&mut store)?;
    Ok(Json(ApiResponse::success_with_message(
        ProjectClearRecycleBinResponse {
            cleared_count,
            cleared: true,
        },
        format!("回收站已清空，共彻底删除 {cleared_count} 个项目"),
    )))
}

/// 查询项目详情。
async fn get_project(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> AppResult<Json<ApiResponse<ProjectInfo>>> {
    let store = state.store.lock().await;
    let info = service::get_project(&store, &project_id, false)?;
    Ok(Json(ApiResponse::success(info)))
}

/// 更新项目名称（重命名）。
async fn update_project(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Json(payload): Json<ProjectUpdateRequest>,
) -> AppResult<Json<ApiResponse<ProjectInfo>>> {
    let mut store = state.store.lock().await;
    let info = service::update_project(&mut store, &project_id, payload.name.as_deref())?;
    Ok(Json(ApiResponse::success_with_message(
        info,
        "项目更新成功",
    )))
}

/// 软删除项目（移入回收站）。
async fn delete_project(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> AppResult<Json<ApiResponse<ProjectDeleteResponse>>> {
    let mut store = state.store.lock().await;
    let info = service::delete_project(&mut store, &project_id)?;
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

/// 从回收站恢复项目。
async fn restore_project(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> AppResult<Json<ApiResponse<ProjectRestoreResponse>>> {
    let mut store = state.store.lock().await;
    let info = service::restore_project(&mut store, &project_id)?;
    Ok(Json(ApiResponse::success_with_message(
        ProjectRestoreResponse {
            project_id: info.project_id,
            name: info.name,
            restored: true,
        },
        "项目已成功恢复",
    )))
}

/// 彻底删除项目（物理硬删除）。
async fn hard_delete_project(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> AppResult<Json<ApiResponse<ProjectHardDeleteResponse>>> {
    let mut store = state.store.lock().await;
    let info = service::hard_delete_project(&mut store, &project_id)?;
    Ok(Json(ApiResponse::success_with_message(
        ProjectHardDeleteResponse {
            project_id: info.project_id,
            name: info.name,
            hard_deleted: true,
        },
        "项目已彻底物理删除",
    )))
}

/// 查询项目章节大纲列表。
async fn list_project_chapters(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> AppResult<Json<ApiResponse<ChapterOutlineListResponse>>> {
    let store = state.store.lock().await;
    let items = service::get_project_chapters(&store, &project_id)?;
    let total = items.len();
    Ok(Json(ApiResponse::success(ChapterOutlineListResponse {
        project_id,
        total,
        items,
    })))
}

/// 查询项目章节详细内容。
async fn get_project_chapter_content(
    State(state): State<AppState>,
    Path((project_id, chapter_id)): Path<(String, String)>,
) -> AppResult<Json<ApiResponse<ChapterContentInfo>>> {
    let store = state.store.lock().await;
    let info = service::get_project_chapter_content(&store, &project_id, &chapter_id)?;
    Ok(Json(ApiResponse::success(info)))
}

/// 向项目追加上传章节文件。
async fn append_project_chapters(
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
    let info = service::append_chapters(
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

/// 查询句子多音字标注列表。
async fn get_sentence_polyphones(
    State(state): State<AppState>,
    Path((project_id, sentence_id)): Path<(String, String)>,
) -> AppResult<Json<ApiResponse<SentencePolyphoneListResponse>>> {
    let store = state.store.lock().await;
    let items = service::get_sentence_polyphones(&store, &project_id, &sentence_id)?;
    let total = items.len();
    Ok(Json(ApiResponse::success(SentencePolyphoneListResponse {
        sentence_id,
        total,
        items,
    })))
}

/// 全量覆盖保存句子的多音字标注。
async fn save_sentence_polyphones(
    State(state): State<AppState>,
    Path((project_id, sentence_id)): Path<(String, String)>,
    Json(payload): Json<SentencePolyphoneSaveRequest>,
) -> AppResult<Json<ApiResponse<SentencePolyphoneListResponse>>> {
    let mut store = state.store.lock().await;
    let items = service::save_sentence_polyphones(
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

/// 清空句子多音字标注。
async fn delete_sentence_polyphones(
    State(state): State<AppState>,
    Path((project_id, sentence_id)): Path<(String, String)>,
) -> AppResult<Json<ApiResponse<SentencePolyphoneDeleteResponse>>> {
    let mut store = state.store.lock().await;
    let deleted_count = service::delete_sentence_polyphones(&mut store, &project_id, &sentence_id)?;
    Ok(Json(ApiResponse::success_with_message(
        SentencePolyphoneDeleteResponse {
            sentence_id,
            deleted_count,
            deleted: true,
        },
        format!("已成功清空多音字标注，共删除 {deleted_count} 条"),
    )))
}
