use chrono::{DateTime, Utc};
use serde::Serialize;

/// 项目角色档案。
#[derive(Debug, Clone, Serialize)]
pub struct CharacterInfo {
    pub character_id: String,
    pub project_id: String,
    pub name: String,
    pub aliases: Vec<String>,
    pub gender: String,
    pub age: Option<String>,
    pub role_type: String,
    pub status: String,
    pub description: String,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
