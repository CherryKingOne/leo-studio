use axum::Json;
use axum::Router;
use axum::extract::{Path, Query, State};
use axum::routing::{delete, get};
use serde::Deserialize;

use crate::error::{AppError, AppResult};
use crate::http::page_count;
use crate::modules::projects::dialogues::model::DialogueInfo;
use crate::modules::projects::dialogues::schema::{
    DialogueListResponse, DialogueSpeakerBindRequest, DialogueSpeakerUnbindResponse,
};
use crate::modules::projects::dialogues::service::{self, DialogueListFilter};
use crate::response::ApiResponse;
use crate::state::AppState;

/// 项目台词管理路由。
pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/projects/{project_id}/chapters/{chapter_id}/dialogues",
            get(list_dialogues),
        )
        .route(
            "/projects/{project_id}/chapters/{chapter_id}/dialogues/{dialogue_id}",
            get(get_dialogue),
        )
        .route(
            "/projects/{project_id}/chapters/{chapter_id}/dialogues/{dialogue_id}/speaker",
            delete(unbind_speaker).put(bind_speaker),
        )
}

#[derive(Debug, Deserialize)]
struct DialogueListQuery {
    page: Option<usize>,
    page_size: Option<usize>,
    segment_type: Option<String>,
    binding_status: Option<String>,
    speaker_id: Option<String>,
    keyword: Option<String>,
}

impl DialogueListQuery {
    fn resolve_pagination(&self) -> AppResult<(usize, usize)> {
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

async fn list_dialogues(
    State(state): State<AppState>,
    Path((project_id, chapter_id)): Path<(String, String)>,
    Query(query): Query<DialogueListQuery>,
) -> AppResult<Json<ApiResponse<DialogueListResponse>>> {
    let (page, page_size) = query.resolve_pagination()?;
    let filter = DialogueListFilter {
        segment_type: query.segment_type.as_deref(),
        binding_status: query.binding_status.as_deref(),
        speaker_id: query.speaker_id.as_deref(),
        keyword: query.keyword.as_deref(),
    };
    let store = state.store.lock().await;
    let (items, total) =
        service::list_dialogues(&store, &project_id, &chapter_id, page, page_size, &filter)?;
    Ok(Json(ApiResponse::success(DialogueListResponse {
        project_id,
        chapter_id,
        total,
        page,
        page_size,
        pages: page_count(total, page_size),
        items,
    })))
}

async fn get_dialogue(
    State(state): State<AppState>,
    Path((project_id, chapter_id, dialogue_id)): Path<(String, String, String)>,
) -> AppResult<Json<ApiResponse<DialogueInfo>>> {
    let store = state.store.lock().await;
    let dialogue = service::get_dialogue(&store, &project_id, &chapter_id, &dialogue_id)?;
    Ok(Json(ApiResponse::success(dialogue)))
}

async fn bind_speaker(
    State(state): State<AppState>,
    Path((project_id, chapter_id, dialogue_id)): Path<(String, String, String)>,
    Json(payload): Json<DialogueSpeakerBindRequest>,
) -> AppResult<Json<ApiResponse<DialogueInfo>>> {
    let mut store = state.store.lock().await;
    let dialogue =
        service::bind_speaker(&mut store, &project_id, &chapter_id, &dialogue_id, &payload)?;
    Ok(Json(ApiResponse::success_with_message(
        dialogue,
        "说话人绑定成功",
    )))
}

async fn unbind_speaker(
    State(state): State<AppState>,
    Path((project_id, chapter_id, dialogue_id)): Path<(String, String, String)>,
) -> AppResult<Json<ApiResponse<DialogueSpeakerUnbindResponse>>> {
    let mut store = state.store.lock().await;
    let result = service::unbind_speaker(&mut store, &project_id, &chapter_id, &dialogue_id)?;
    Ok(Json(ApiResponse::success_with_message(
        result,
        "说话人已解绑",
    )))
}
