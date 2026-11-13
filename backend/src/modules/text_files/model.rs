use chrono::{DateTime, Utc};
use serde::Serialize;

/// 已存储文本文件的元数据。
#[derive(Debug, Clone, Serialize)]
pub struct TextFileInfo {
    pub file_id: String,
    pub filename: String,
    pub stored_name: String,
    pub extension: String,
    pub size: usize,
    pub content_type: String,
    pub category: String,
    pub uploaded_at: DateTime<Utc>,
    pub download_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// 扩展名到归类的映射。
pub fn categorize(extension: &str) -> String {
    match extension.to_lowercase().as_str() {
        ".txt" => "plain_text".to_string(),
        _ => "other".to_string(),
    }
}

/// 扩展名到默认 MIME 类型的映射。
pub fn default_mime(extension: &str) -> Option<&'static str> {
    match extension.to_lowercase().as_str() {
        ".txt" => Some("text/plain"),
        _ => None,
    }
}
