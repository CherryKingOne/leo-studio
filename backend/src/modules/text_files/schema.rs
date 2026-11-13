use chrono::{DateTime, Utc};
use serde::Serialize;

/// 单文件上传结果。
#[derive(Debug, Clone, Serialize)]
pub struct TextFileUploadResponse {
    pub file_id: String,
    pub filename: String,
    pub stored_name: String,
    pub extension: String,
    pub size: usize,
    pub size_human: String,
    pub content_type: String,
    pub category: String,
    pub uploaded_at: DateTime<Utc>,
    pub download_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// 文件列表项。
#[derive(Debug, Clone, Serialize)]
pub struct TextFileListItem {
    pub file_id: String,
    pub filename: String,
    pub extension: String,
    pub size: usize,
    pub size_human: String,
    pub category: String,
    pub uploaded_at: DateTime<Utc>,
    pub download_url: String,
}

/// 文件列表分页结果。
#[derive(Debug, Clone, Serialize)]
pub struct TextFileListResponse {
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
    pub pages: usize,
    pub items: Vec<TextFileListItem>,
}

/// 删除文件结果。
#[derive(Debug, Clone, Serialize)]
pub struct TextFileDeleteResponse {
    pub file_id: String,
    pub filename: String,
    pub deleted: bool,
}

/// 文本文件内容响应。
#[derive(Debug, Clone, Serialize)]
pub struct TextFileContentResponse {
    pub file_id: String,
    pub filename: String,
    pub content_type: String,
    pub encoding: String,
    pub size: usize,
    pub content: String,
}

/// 文本文件章节切句项。
#[derive(Debug, Clone, Serialize)]
pub struct TextFileSentenceItem {
    pub sentence_id: String,
    pub novel_id: String,
    pub chapter_id: String,
    pub sentence_index: i64,
    pub content: String,
    pub word_count: i64,
}

/// 小说切句列表响应。
#[derive(Debug, Clone, Serialize)]
pub struct TextFileSentenceListResponse {
    pub novel_id: String,
    pub total: usize,
    pub items: Vec<TextFileSentenceItem>,
}
