use chrono::{DateTime, Utc};
use serde::Serialize;

/// 项目章节台词片段。
#[derive(Debug, Clone, Serialize)]
pub struct DialogueInfo {
    pub project_id: String,
    pub chapter_id: String,
    pub dialogue_id: String,
    pub parent_sentence_id: String,
    pub sentence_index: i64,
    pub segment_index: i64,
    pub segment_type: String,
    pub content: String,
    pub word_count: i64,
    pub parse_status: String,
    pub speaker_id: Option<String>,
    pub speaker_name: Option<String>,
    pub binding_source: Option<String>,
    pub binding_confidence: Option<f64>,
    pub speaker_updated_at: Option<DateTime<Utc>>,
}
