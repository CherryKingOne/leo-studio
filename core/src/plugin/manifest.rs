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
    Llm,
    Storage,
    Api,
    Transport,
    Mcp,
    Cli,
    /// 注册层插件（某个扩展点的“大箱子”）
    Registry,
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
    /// 必须先装配的插件 id（缺失时装配失败）。只用于决定顺序；
    /// 往注册表里注册驱动不需要声明依赖，注册层与子插件谁先装配都可以。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends: Vec<String>,
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
            depends: Vec::new(),
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn with_capabilities<I: IntoIterator<Item = S>, S: Into<String>>(mut self, caps: I) -> Self {
        self.capabilities = caps.into_iter().map(Into::into).collect();
        self
    }

    pub fn depends_on<I: IntoIterator<Item = S>, S: Into<String>>(mut self, ids: I) -> Self {
        self.depends = ids.into_iter().map(Into::into).collect();
        self
    }
}
