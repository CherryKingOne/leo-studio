use axum::Json;
use axum::Router;
use axum::extract::{Multipart, Path, Query, State};
use axum::http::{StatusCode, header};
use axum::response::Response;
use axum::routing::{get, post};
use serde::Deserialize;
use serde_json::json;

use crate::error::{AppError, AppResult};
use crate::response::ApiResponse;
use crate::state::AppState;
use leo_plugin_exchange::ExchangeHub;
use leo_plugin_storage::StorageHub;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/import", post(import_script))
        .route("/export/{project_id}", get(export_script))
}

#[derive(Debug, Deserialize)]
pub struct ExportQuery {
    pub format: Option<String>,
}

/// 导入剧本文档端点：根据上传文件后缀自动匹配已激活的 Importer 插件
async fn import_script(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    let mut filename = String::new();
    let mut file_bytes = Vec::new();

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" {
            filename = field.file_name().unwrap_or("novel.txt").to_string();
            file_bytes = field
                .bytes()
                .await
                .map_err(|e| AppError::invalid_parameter(format!("读取上传文件失败: {e}")))?
                .to_vec();
            break;
        }
    }

    if file_bytes.is_empty() {
        return Err(AppError::invalid_parameter("上传文件为空"));
    }

    // 由导入导出注册层按扩展名挑选导入器
    let (importer_id, importer) = ExchangeHub::of(&state.context)
        .await
        .importer_for(&filename)
        .map_err(|e| AppError::invalid_parameter(e.to_string()))?;
    let doc = importer
        .import(&filename, &file_bytes)
        .await
        .map_err(|e| AppError::internal(format!("导入解析失败（{importer_id}）: {e}")))?;

    // 若注册了存储驱动，则自动持久化
    if let Ok(storage) = StorageHub::of(&state.context).await.default_storage() {
        if let Err(err) = storage.save_script(&doc).await {
            tracing::warn!("自动保存导入剧本失败: {err}");
        }
    }

    Ok(Json(ApiResponse::success(json!({
        "id": doc.id,
        "title": doc.title,
        "chapter_count": doc.chapters.len(),
    }))))
}

/// 导出剧本文档端点：根据 format 参数查找对应 Exporter 插件并返回文件流
async fn export_script(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Query(query): Query<ExportQuery>,
) -> Result<Response, AppError> {
    // 由导入导出注册层按格式挑选导出器（不传 format 时用默认导出器）
    let (_, exporter) = ExchangeHub::of(&state.context)
        .await
        .exporter(query.format.as_deref())
        .map_err(|e| AppError::invalid_parameter(e.to_string()))?;

    // 加载目标剧本 AST
    let doc = if let Ok(storage) = StorageHub::of(&state.context).await.default_storage() {
        storage
            .load_script(&project_id)
            .await
            .map_err(|e| AppError::internal(format!("加载剧本失败: {e}")))?
    } else {
        None
    };

    let doc = doc.ok_or_else(|| AppError::file_not_found(format!("未找到项目 {project_id} 的剧本文档")))?;

    let data = exporter
        .export(&doc)
        .await
        .map_err(|e| AppError::internal(format!("{} 导出失败: {e}", exporter.format_name())))?;
    let mime_type = exporter.mime_type().to_string();
    let ext = exporter.file_extension().to_string();

    let filename = format!("{}_{}.{}", doc.title, project_id, ext);
    let disposition = format!("attachment; filename=\"{}\"", filename);

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime_type)
        .header(header::CONTENT_DISPOSITION, disposition)
        .body(axum::body::Body::from(data))
        .map_err(|e| AppError::internal(format!("构建响应流失败: {e}")))?;

    Ok(response)
}
