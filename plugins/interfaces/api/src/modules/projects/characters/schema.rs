use serde::{Deserialize, Deserializer};
use serde_json::Value;

use super::model::CharacterInfo;

#[derive(Debug, Clone, Deserialize)]
pub struct CharacterCreateRequest {
    pub name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub gender: Option<String>,
    #[serde(default)]
    pub age: Option<String>,
    #[serde(default)]
    pub role_type: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CharacterUpdateRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub aliases: Option<Vec<String>>,
    #[serde(default)]
    pub gender: Option<String>,
    #[serde(default, deserialize_with = "deserialize_present_value")]
    pub age: Option<Value>,
    #[serde(default)]
    pub role_type: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
}

fn deserialize_present_value<'de, D>(deserializer: D) -> Result<Option<Value>, D::Error>
where
    D: Deserializer<'de>,
{
    Value::deserialize(deserializer).map(Some)
}

impl CharacterUpdateRequest {
    pub fn has_updates(&self) -> bool {
        self.name.is_some()
            || self.aliases.is_some()
            || self.gender.is_some()
            || self.age.is_some()
            || self.role_type.is_some()
            || self.status.is_some()
            || self.description.is_some()
            || self.tags.is_some()
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CharacterListResponse {
    pub project_id: String,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
    pub pages: usize,
    pub items: Vec<CharacterInfo>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CharacterDeleteResponse {
    pub project_id: String,
    pub character_id: String,
    pub name: String,
    pub deleted: bool,
}
