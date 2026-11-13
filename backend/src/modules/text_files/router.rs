use axum::Json;
use axum::Router;
use axum::body::Body;
use axum::extract::{Multipart, Path, Query, State};
use axum::http::header::{CONTENT_DISPOSITION, CONTENT_TYPE};
use axum::http::{HeaderValue, Response};
use axum::routing::get;
use serde::Deserialize;

use crate::error::{AppError, AppResult};
use crate::http::{Pagination, page_count};
use crate::response::ApiResponse;
use crate::state::AppState;
use crate::storage::store::human_size;
use crate::storage::{int_field, str_field};

use super::model::TextFileInfo;
use super::schema::{
    TextFileContentResponse, TextFileDeleteResponse, TextFileListItem, TextFileListResponse,
    TextFileSentenceItem, TextFileSentenceListResponse, TextFileUploadResponse,
};
use super::service;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_text_files).post(upload_text_file))
        .route("/upload", get(list_text_files).post(upload_text_file))
        .route("/{file_id}", get(get_text_file).delete(delete_text_file))
        .route("/{file_id}/content", get(get_text_file_content))
        .route("/{file_id}/download", get(download_text_file))
        .route("/{file_id}/sentences", get(list_novel_sentences))
}

fn to_upload_response(info: &TextFileInfo) -> TextFileUploadResponse {
    TextFileUploadResponse {
        file_id: info.file_id.clone(),
        filename: info.filename.clone(),
        stored_name: info.stored_name.clone(),
        extension: info.extension.clone(),
        size: info.size,
        size_human: human_size(info.size),
        content_type: info.content_type.clone(),
        category: info.category.clone(),
        uploaded_at: info.uploaded_at,
        download_url: info.download_url.clone(),
        description: info.description.clone(),
    }
}

/// 上传单个文本文件。
async fn upload_text_file(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> AppResult<Json<ApiResponse<TextFileUploadResponse>>> {
    let max_bytes = state.settings.max_bytes();
    let mut filename = String::new();
    let mut content_type: Option<String> = None;
    let mut content: Vec<u8> = Vec::new();
    let mut description: Option<String> = None;
    let mut found_file = false;

    while let Some(mut field) = multipart
        .next_field()
        .await
        .map_err(|err| AppError::invalid_parameter(format!("表单解析失败：{err}")))?
    {
        match field.name() {
            Some("file") => {
                found_file = true;
                filename = field.file_name().unwrap_or("").to_string();
                content_type = field.content_type().map(|value| value.to_string());
                loop {
                    match field.chunk().await {
                        Ok(Some(chunk)) => {
                            if content.len().saturating_add(chunk.len()) > max_bytes {
                                return Err(AppError::file_too_large(format!(
                                    "文件大小超过限制，最大允许 {}",
                                    human_size(max_bytes)
                                )));
                            }
                            content.extend_from_slice(&chunk);
                        }
                        Ok(None) => break,
                        Err(err) => {
                            return Err(AppError::invalid_parameter(format!(
                                "文件读取失败：{err}"
                            )));
                        }
                    }
                }
            }
            Some("description") => {
                description = Some(field.text().await.map_err(|err| {
                    AppError::invalid_parameter(format!("表单字段读取失败：{err}"))
                })?);
            }
            _ => {}
        }
    }

    if !found_file || filename.trim().is_empty() {
        return Err(AppError::invalid_parameter("文件名不能为空"));
    }

    let mut store = state.store.lock().await;
    let info = service::save_file(
        &mut store,
        &state.settings,
        &filename,
        &content,
        content_type.as_deref(),
        description.as_deref(),
    )?;
    Ok(Json(ApiResponse::success_with_message(
        to_upload_response(&info),
        "文件上传成功",
    )))
}

/// 分页查询已上传的文本文件。
async fn list_text_files(
    State(state): State<AppState>,
    Query(pagination): Query<Pagination>,
) -> AppResult<Json<ApiResponse<TextFileListResponse>>> {
    let (page, page_size) = pagination.resolve()?;
    let store = state.store.lock().await;
    let (items, total) = service::list_files(&store, page, page_size);

    Ok(Json(ApiResponse::success(TextFileListResponse {
        total,
        page,
        page_size,
        pages: page_count(total, page_size),
        items: items
            .iter()
            .map(|item| TextFileListItem {
                file_id: item.file_id.clone(),
                filename: item.filename.clone(),
                extension: item.extension.clone(),
                size: item.size,
                size_human: human_size(item.size),
                category: item.category.clone(),
                uploaded_at: item.uploaded_at,
                download_url: item.download_url.clone(),
            })
            .collect(),
    })))
}

/// 查询文本文件详情。
async fn get_text_file(
    State(state): State<AppState>,
    Path(file_id): Path<String>,
) -> AppResult<Json<ApiResponse<TextFileUploadResponse>>> {
    let store = state.store.lock().await;
    let info = service::get_file(&store, &file_id)?;
    Ok(Json(ApiResponse::success(to_upload_response(&info))))
}

#[derive(Debug, Deserialize)]
struct ContentQuery {
    #[serde(default = "default_encoding")]
    encoding: String,
}

fn default_encoding() -> String {
    "utf-8".to_string()
}

/// 预览文本文件内容。
async fn get_text_file_content(
    State(state): State<AppState>,
    Path(file_id): Path<String>,
    Query(query): Query<ContentQuery>,
) -> AppResult<Json<ApiResponse<TextFileContentResponse>>> {
    let store = state.store.lock().await;
    let (content, info) = service::read_text(&store, &file_id, &query.encoding)?;
    Ok(Json(ApiResponse::success(TextFileContentResponse {
        file_id: info.file_id,
        filename: info.filename,
        content_type: info.content_type,
        encoding: query.encoding,
        size: info.size,
        content,
    })))
}

/// 下载文本文件。
async fn download_text_file(
    State(state): State<AppState>,
    Path(file_id): Path<String>,
) -> AppResult<Response<Body>> {
    let store = state.store.lock().await;
    let (path, info) = service::open_file(&store, &file_id)?;
    let bytes =
        std::fs::read(&path).map_err(|err| crate::error::storage_io("读取下载文件失败", err))?;

    let disposition = content_disposition(&info.filename);
    let response = Response::builder()
        .header(CONTENT_TYPE, info.content_type)
        .header(
            CONTENT_DISPOSITION,
            HeaderValue::from_str(&disposition)
                .map_err(|err| AppError::internal(format!("响应头构造失败：{err}")))?,
        )
        .body(Body::from(bytes))
        .map_err(|err| AppError::internal(format!("响应构造失败：{err}")))?;
    Ok(response)
}

fn content_disposition(filename: &str) -> String {
    let ascii_fallback: String = filename
        .chars()
        .map(|ch| if ch.is_ascii() && ch != '"' { ch } else { '_' })
        .collect();
    format!(
        "attachment; filename=\"{ascii_fallback}\"; filename*=UTF-8''{}",
        encode_filename(filename)
    )
}

fn encode_filename(filename: &str) -> String {
    let mut out = String::new();
    for byte in filename.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// 删除文本文件。
async fn delete_text_file(
    State(state): State<AppState>,
    Path(file_id): Path<String>,
) -> AppResult<Json<ApiResponse<TextFileDeleteResponse>>> {
    let mut store = state.store.lock().await;
    let info = service::delete_file(&mut store, &file_id)?;
    Ok(Json(ApiResponse::success_with_message(
        TextFileDeleteResponse {
            file_id: info.file_id,
            filename: info.filename,
            deleted: true,
        },
        "文件删除成功",
    )))
}

#[derive(Debug, Deserialize)]
struct SentenceQuery {
    chapter_id: Option<String>,
}

/// 查询小说章节切句列表。
async fn list_novel_sentences(
    State(state): State<AppState>,
    Path(file_id): Path<String>,
    Query(query): Query<SentenceQuery>,
) -> AppResult<Json<ApiResponse<TextFileSentenceListResponse>>> {
    let store = state.store.lock().await;
    service::get_file(&store, &file_id)?;

    let rows = if let Some(chapter_id) = query.chapter_id.as_deref() {
        service::get_chapter_sentences(&store, chapter_id)
    } else {
        service::get_novel_sentences(&store, &file_id)
    };

    let items: Vec<TextFileSentenceItem> = rows
        .iter()
        .map(|row| TextFileSentenceItem {
            sentence_id: str_field(row, "sentence_id").to_string(),
            novel_id: str_field(row, "novel_id").to_string(),
            chapter_id: str_field(row, "chapter_id").to_string(),
            sentence_index: int_field(row, "sentence_index"),
            content: str_field(row, "content").to_string(),
            word_count: int_field(row, "word_count"),
        })
        .collect();

    let total = items.len();
    Ok(Json(ApiResponse::success(TextFileSentenceListResponse {
        novel_id: file_id,
        total,
        items,
    })))
}
