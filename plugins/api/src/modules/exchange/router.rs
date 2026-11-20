use std::sync::Arc;

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
use leo_core::plugin::{Exporter, Importer, StorageProvider};
use leo_plugin_exporter_csv::CsvExporter;
use leo_plugin_exporter_json::JsonExporter;
use leo_plugin_importer_txt::TxtImporter;

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

    let doc = if let Ok(importer) = state.context.inject::<Arc<dyn Importer>>().await {
        importer.import(&filename, &file_bytes).await
    } else {
        let importer = TxtImporter::new();
        importer.import(&filename, &file_bytes).await
    }
    .map_err(|e| AppError::internal(format!("导入解析失败: {e}")))?;

    // 若存在 StorageProvider，则自动持久化
    if let Ok(storage) = state.context.inject::<Arc<dyn StorageProvider>>().await {
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
    let format = query
        .format
        .unwrap_or_else(|| "json".to_string())
        .to_lowercase();

    // 加载目标剧本 AST
    let doc = if let Ok(storage) = state.context.inject::<Arc<dyn StorageProvider>>().await {
        storage
            .load_script(&project_id)
            .await
            .map_err(|e| AppError::internal(format!("加载剧本失败: {e}")))?
    } else {
        None
    };

    let doc = doc.ok_or_else(|| AppError::file_not_found(format!("未找到项目 {project_id} 的剧本文档")))?;

    // 获取导出器并执行导出
    let (data, mime_type, ext) = if format == "csv" {
        let exporter = CsvExporter;
        let bytes = exporter
            .export(&doc)
            .await
            .map_err(|e| AppError::internal(format!("CSV 导出失败: {e}")))?;
        (bytes, "text/csv; charset=utf-8", "csv")
    } else {
        let exporter = JsonExporter;
        let bytes = exporter
            .export(&doc)
            .await
            .map_err(|e| AppError::internal(format!("JSON 导出失败: {e}")))?;
        (bytes, "application/json", "json")
    };

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
