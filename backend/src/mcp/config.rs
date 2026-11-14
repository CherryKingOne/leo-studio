/// MCP 运行期配置，全部来自 `LEO_MCP_*` 环境变量。
///
/// MCP 固定走 Streamable HTTP，没有传输方式开关。
#[derive(Debug, Clone)]
pub struct McpSettings {
    pub enabled: bool,
    pub http_path: String,
    pub token: String,
    pub write_tools: bool,
    pub destructive_tools: bool,
    pub max_chars: usize,
    pub max_page_size: usize,
}

impl Default for McpSettings {
    fn default() -> Self {
        Self::from_env()
    }
}

impl McpSettings {
    pub fn from_env() -> Self {
        let http_path = env_string("LEO_MCP_HTTP_PATH", "/mcp");
        let http_path = if http_path.starts_with('/') {
            http_path
        } else {
            format!("/{http_path}")
        };

        Self {
            enabled: env_bool("LEO_MCP_HTTP_ENABLED", false),
            http_path,
            token: env_string("LEO_MCP_TOKEN", ""),
            write_tools: env_bool("LEO_MCP_WRITE_TOOLS", false),
            destructive_tools: env_bool("LEO_MCP_DESTRUCTIVE_TOOLS", false),
            max_chars: 20_000,
            max_page_size: 100,
        }
    }
}

fn env_string(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

fn env_bool(key: &str, default: bool) -> bool {
    match std::env::var(key) {
        Ok(value) => matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        ),
        Err(_) => default,
    }
}
