use axum::Json;
use axum::Router;
use axum::extract::{Path, Query, State};
use axum::routing::get;
use serde::Deserialize;

use crate::error::{AppError, AppResult};
use crate::http::page_count;
use crate::modules::projects::characters::model::CharacterInfo;
use crate::modules::projects::characters::schema::{
    CharacterCreateRequest, CharacterDeleteResponse, CharacterListResponse, CharacterUpdateRequest,
};
use crate::modules::projects::characters::service::{self, CharacterListFilter};
use crate::response::ApiResponse;
use crate::state::AppState;

/// 项目角色管理路由。
pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/projects/{project_id}/characters",
            get(list_characters).post(create_character),
        )
        .route(
            "/projects/{project_id}/characters/{character_id}",
            get(get_character)
                .patch(update_character)
                .delete(delete_character),
        )
}

#[derive(Debug, Deserialize)]
struct CharacterListQuery {
    page: Option<usize>,
    page_size: Option<usize>,
    keyword: Option<String>,
    gender: Option<String>,
    role_type: Option<String>,
    status: Option<String>,
    tag: Option<String>,
    sort: Option<String>,
    order: Option<String>,
}

impl CharacterListQuery {
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

async fn create_character(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Json(payload): Json<CharacterCreateRequest>,
) -> AppResult<Json<ApiResponse<CharacterInfo>>> {
    let mut store = state.store.lock().await;
    let character = service::create_character(&mut store, &project_id, &payload)?;
    Ok(Json(ApiResponse::success_with_message(
        character,
        "角色创建成功",
    )))
}

async fn list_characters(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Query(query): Query<CharacterListQuery>,
) -> AppResult<Json<ApiResponse<CharacterListResponse>>> {
    let (page, page_size) = query.resolve_pagination()?;
    let gender = validate_filter_enum(
        query.gender.as_deref(),
        &["male", "female", "unknown", "other"],
        "性别",
    )?;
    let role_type = validate_filter_enum(
        query.role_type.as_deref(),
        &[
            "protagonist",
            "supporting",
            "antagonist",
            "narrator",
            "other",
        ],
        "角色类型",
    )?;
    let status = match query
        .status
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        None => Some("active".to_string()),
        Some("all") => None,
        Some(value) if value == "active" || value == "archived" => Some(value.to_string()),
        Some(value) => {
            return Err(AppError::invalid_parameter(format!(
                "角色状态不支持 {value}，可选值：active, archived, all"
            )));
        }
    };
    let sort = query.sort.as_deref().unwrap_or("created_at");
    let order = query.order.as_deref().unwrap_or("desc");
    let filter = CharacterListFilter {
        keyword: query.keyword.as_deref(),
        gender: gender.as_deref(),
        role_type: role_type.as_deref(),
        status: status.as_deref(),
        tag: query.tag.as_deref(),
    };

    let store = state.store.lock().await;
    let (items, total) =
        service::list_characters(&store, &project_id, page, page_size, sort, order, &filter)?;
    Ok(Json(ApiResponse::success(CharacterListResponse {
        project_id,
        total,
        page,
        page_size,
        pages: page_count(total, page_size),
        items,
    })))
}

async fn get_character(
    State(state): State<AppState>,
    Path((project_id, character_id)): Path<(String, String)>,
) -> AppResult<Json<ApiResponse<CharacterInfo>>> {
    let store = state.store.lock().await;
    let character = service::get_character(&store, &project_id, &character_id)?;
    Ok(Json(ApiResponse::success(character)))
}

async fn update_character(
    State(state): State<AppState>,
    Path((project_id, character_id)): Path<(String, String)>,
    Json(payload): Json<CharacterUpdateRequest>,
) -> AppResult<Json<ApiResponse<CharacterInfo>>> {
    let mut store = state.store.lock().await;
    let character = service::update_character(&mut store, &project_id, &character_id, &payload)?;
    Ok(Json(ApiResponse::success_with_message(
        character,
        "角色更新成功",
    )))
}

async fn delete_character(
    State(state): State<AppState>,
    Path((project_id, character_id)): Path<(String, String)>,
) -> AppResult<Json<ApiResponse<CharacterDeleteResponse>>> {
    let mut store = state.store.lock().await;
    let result = service::delete_character(&mut store, &project_id, &character_id)?;
    Ok(Json(ApiResponse::success_with_message(
        result,
        "角色删除成功",
    )))
}

fn validate_filter_enum(
    value: Option<&str>,
    allowed: &[&str],
    field: &str,
) -> AppResult<Option<String>> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    if !allowed.contains(&value) {
        return Err(AppError::invalid_parameter(format!(
            "{field}不支持 {value}，可选值：{}",
            allowed.join(", ")
        )));
    }
    Ok(Some(value.to_string()))
}
