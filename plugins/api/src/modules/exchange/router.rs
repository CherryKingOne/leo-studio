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
use leo_core::error::CoreError;
use leo_core::plugin::{Exporter, Importer, StorageProvider};

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

    let importer = find_importer(&state, &filename).await?;
    let doc = importer
        .import(&filename, &file_bytes)
        .await
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

    // 按 format 名称查找已注册的 Exporter 插件并执行导出
    let exporter = find_exporter(&state, &format).await?;
    let data = exporter
        .export(&doc)
        .await
        .map_err(|e| AppError::internal(format!("{format} 导出失败: {e}")))?;
    let mime_type = exporter.mime_type().to_string();

    let filename = format!("{}_{}.{}", doc.title, project_id, exporter.format_name());
    let disposition = format!("attachment; filename=\"{}\"", filename);

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime_type)
        .header(header::CONTENT_DISPOSITION, disposition)
        .body(axum::body::Body::from(data))
        .map_err(|e| AppError::internal(format!("构建响应流失败: {e}")))?;

    Ok(response)
}

/// 按上传文件扩展名挑选第一个声明支持该扩展名的 Importer 插件（按注册顺序）；
/// 文件名不带扩展名时使用默认 Importer
async fn find_importer(state: &AppState, filename: &str) -> AppResult<Arc<dyn Importer>> {
    let extension = std::path::Path::new(filename)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(str::to_lowercase)
        .unwrap_or_default();
    if extension.is_empty() {
        return state
            .context
            .inject::<Arc<dyn Importer>>()
            .await
            .map(|importer| importer.as_ref().clone())
            .map_err(|e| AppError::internal(format!("未挂载任何导入插件: {e}")));
    }
    let importers = state.context.inject_all::<Arc<dyn Importer>>().await;
    importers
        .iter()
        .find(|(_, importer)| {
            importer
                .supported_extensions()
                .iter()
                .any(|ext| ext.eq_ignore_ascii_case(&extension))
        })
        .map(|(_, importer)| importer.as_ref().clone())
        .ok_or_else(|| {
            let supported: Vec<String> = importers
                .iter()
                .flat_map(|(_, importer)| importer.supported_extensions().iter().map(|e| format!(".{e}")))
                .collect();
            AppError::unsupported_file_type(format!(
                "没有可处理 .{extension} 文件的导入插件，已挂载的导入格式：{}",
                supported.join(", ")
            ))
        })
}

/// 按名称查找 Exporter 插件；未注册该格式时返回参数错误并列出可用格式
async fn find_exporter(state: &AppState, format: &str) -> AppResult<Arc<dyn Exporter>> {
    match state.context.inject_named::<Arc<dyn Exporter>>(format).await {
        Ok(exporter) => Ok(exporter.as_ref().clone()),
        Err(CoreError::ServiceNotFound(_)) => {
            let available: Vec<String> = state
                .context
                .inject_all::<Arc<dyn Exporter>>()
                .await
                .into_iter()
                .map(|(name, _)| name)
                .collect();
            Err(AppError::invalid_parameter(format!(
                "不支持的导出格式 {format}，可用格式：{}",
                available.join(", ")
            )))
        }
        Err(err) => Err(AppError::internal(format!("获取导出插件失败: {err}"))),
    }
}
