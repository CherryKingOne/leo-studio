use serde::{Deserialize, Serialize};

use super::model::{ProjectChapterAppendInfo, ProjectInfo};

/// 更新项目请求体（重命名）。
#[derive(Debug, Clone, Deserialize)]
pub struct ProjectUpdateRequest {
    #[serde(default)]
    pub name: Option<String>,
}

/// 单个多音字标注项（用于保存请求）。
#[derive(Debug, Clone, Deserialize)]
pub struct PolyphoneAnnotation {
    #[serde(default)]
    pub char_index: i64,
    #[serde(default)]
    pub character: String,
    #[serde(default)]
    pub pinyin: String,
    #[serde(default)]
    pub pinyin_base: Option<String>,
    #[serde(default)]
    pub tone: Option<i64>,
    #[serde(default)]
    pub tts_pinyin: Option<String>,
}

/// 保存句子多音字标注请求体（全量覆盖式）。
#[derive(Debug, Clone, Deserialize)]
pub struct SentencePolyphoneSaveRequest {
    #[serde(default)]
    pub annotations: Vec<PolyphoneAnnotation>,
}

/// 项目列表分页响应。
#[derive(Debug, Clone, Serialize)]
pub struct ProjectListResponse {
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
    pub pages: usize,
    pub items: Vec<ProjectInfo>,
}

/// 项目软删除响应。
#[derive(Debug, Clone, Serialize)]
pub struct ProjectDeleteResponse {
    pub project_id: String,
    pub name: String,
    pub deleted: bool,
    pub deleted_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// 项目从回收站恢复响应。
#[derive(Debug, Clone, Serialize)]
pub struct ProjectRestoreResponse {
    pub project_id: String,
    pub name: String,
    pub restored: bool,
}

/// 项目彻底删除响应。
#[derive(Debug, Clone, Serialize)]
pub struct ProjectHardDeleteResponse {
    pub project_id: String,
    pub name: String,
    pub hard_deleted: bool,
}

/// 清空回收站响应。
#[derive(Debug, Clone, Serialize)]
pub struct ProjectClearRecycleBinResponse {
    pub cleared_count: usize,
    pub cleared: bool,
}

/// 项目章节大纲列表响应。
#[derive(Debug, Clone, Serialize)]
pub struct ChapterOutlineListResponse {
    pub project_id: String,
    pub total: usize,
    pub items: Vec<super::model::ChapterOutlineInfo>,
}

/// 项目追加章节响应。
pub type ProjectChapterAppendResponse = ProjectChapterAppendInfo;

/// 句子多音字标注列表响应。
#[derive(Debug, Clone, Serialize)]
pub struct SentencePolyphoneListResponse {
    pub sentence_id: String,
    pub total: usize,
    pub items: Vec<super::model::SentencePolyphoneInfo>,
}

/// 删除句子多音字标注响应。
#[derive(Debug, Clone, Serialize)]
pub struct SentencePolyphoneDeleteResponse {
    pub sentence_id: String,
    pub deleted_count: usize,
    pub deleted: bool,
}
