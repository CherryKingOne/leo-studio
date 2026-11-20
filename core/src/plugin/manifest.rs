use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginCategory {
    Importer,
    Exporter,
    Cleaner,
    Parser,
    Dialogue,
    Tts,
    Storage,
    Api,
    Transport,
    Mcp,
    Cli,
    Other,
}

/// 插件元信息清单
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub category: PluginCategory,
    pub capabilities: Vec<String>,
}

impl PluginManifest {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        version: impl Into<String>,
        category: PluginCategory,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            version: version.into(),
            description: String::new(),
            category,
            capabilities: Vec::new(),
        }
    }
}
