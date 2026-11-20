use axum::Json;
use axum::Router;
use axum::extract::{Path, State};
use axum::http::header::CONTENT_TYPE;
use axum::response::IntoResponse;
use axum::routing::{get, post};

use crate::error::AppResult;
use crate::response::ApiResponse;
use crate::state::AppState;

use super::schema::{ChapterSynthesizeRequest, ChapterSynthesizeResponse, PluginItem, VoiceItem};
use super::service;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/projects/{project_id}/chapters/{chapter_id}/synthesize",
            post(synthesize_chapter_handler),
        )
        .route("/audio/{clip_id}", get(get_audio_clip_handler))
        .route("/voices", get(list_voices_handler))
        .route("/plugins", get(list_plugins_handler))
}

async fn synthesize_chapter_handler(
    State(state): State<AppState>,
    Path((project_id, chapter_id)): Path<(String, String)>,
    Json(payload): Json<Option<ChapterSynthesizeRequest>>,
) -> AppResult<Json<ApiResponse<ChapterSynthesizeResponse>>> {
    let req = payload.unwrap_or_else(|| ChapterSynthesizeRequest {
        voice: "zh-CN-YunxiNeural".into(),
        speed: 1.0,
        pitch: 0.0,
        force: false,
    });
    let result = service::synthesize_chapter(&state, &project_id, &chapter_id, req).await?;
    Ok(Json(ApiResponse::success(result)))
}

async fn get_audio_clip_handler(
    State(state): State<AppState>,
    Path(clip_id): Path<String>,
) -> AppResult<impl IntoResponse> {
    let bytes = service::get_audio_clip(&state, &clip_id).await?;
    Ok(([(CONTENT_TYPE, "audio/mpeg")], bytes))
}

async fn list_voices_handler(
    State(state): State<AppState>,
) -> AppResult<Json<ApiResponse<Vec<VoiceItem>>>> {
    let voices = service::list_voices(&state).await?;
    Ok(Json(ApiResponse::success(voices)))
}

async fn list_plugins_handler(
    State(state): State<AppState>,
) -> AppResult<Json<ApiResponse<Vec<PluginItem>>>> {
    let plugins = service::list_plugins(&state).await?;
    Ok(Json(ApiResponse::success(plugins)))
}
