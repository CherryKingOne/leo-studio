use axum::Json;
use axum::Router;
use axum::extract::{Path, State};
use axum::routing::{delete, get};

use crate::error::AppResult;
use crate::modules::projects::schema::{
    ChapterPolyphoneListResponse, SentencePolyphoneDeleteResponse,
    SentencePolyphoneItemDeleteResponse, SentencePolyphoneListResponse,
    SentencePolyphoneSaveRequest,
};
use crate::modules::projects::service as project_service;
use crate::response::ApiResponse;
use crate::state::AppState;

/// 项目多音字管理路由。
pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/projects/{project_id}/chapters/{chapter_id}/polyphones",
            get(get_chapter_polyphones),
        )
        .route(
            "/projects/{project_id}/chapters/{chapter_id}/sentences/{sentence_id}/polyphones",
            get(get_chapter_sentence_polyphones)
                .put(save_chapter_sentence_polyphones)
                .delete(delete_chapter_sentence_polyphones),
        )
        .route(
            "/projects/{project_id}/chapters/{chapter_id}/sentences/{sentence_id}/polyphones/items/{char_index}",
            delete(delete_chapter_sentence_polyphone),
        )
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
