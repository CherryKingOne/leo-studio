use serde::{Deserialize, Serialize};

use super::model::DialogueInfo;

#[derive(Debug, Clone, Deserialize)]
pub struct DialogueSpeakerBindRequest {
    pub speaker_id: String,
    #[serde(default)]
    pub binding_source: Option<String>,
    #[serde(default)]
    pub binding_confidence: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DialogueListResponse {
    pub project_id: String,
    pub chapter_id: String,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
    pub pages: usize,
    pub items: Vec<DialogueInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DialogueSpeakerUnbindResponse {
    pub dialogue_id: String,
    pub unbound: bool,
}
