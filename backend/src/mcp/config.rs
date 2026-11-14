use std::path::PathBuf;

use crate::config::resolve_path;

/// MCP 传输方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpTransport {
    Stdio,
    Http,
    Both,
}

impl McpTransport {
    pub fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "http" => Self::Http,
            "both" => Self::Both,
            _ => Self::Stdio,
        }
    }

    /// 是否需要在 Axum 进程内挂载 `/mcp` 端点。
    pub fn serves_http(self) -> bool {
        matches!(self, Self::Http | Self::Both)
    }
}

/// MCP 运行期配置，全部来自 `LEO_MCP_*` 环境变量。
#[derive(Debug, Clone)]
pub struct McpSettings {
    pub enabled: bool,
    pub transport: McpTransport,
    pub http_path: String,
    pub token: String,
    pub write_tools: bool,
    pub destructive_tools: bool,
    pub allow_file_path: bool,
    pub allowed_roots: Vec<PathBuf>,
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
        let raw_roots = env_string("LEO_MCP_ALLOWED_ROOTS", "");
        let allowed_roots = raw_roots
            .split(',')
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(|item| resolve_path(std::path::Path::new(item)))
            .collect();

        let http_path = env_string("LEO_MCP_HTTP_PATH", "/mcp");
        let http_path = if http_path.starts_with('/') {
            http_path
        } else {
            format!("/{http_path}")
        };

        Self {
            enabled: env_bool("LEO_MCP_ENABLED", false),
            transport: McpTransport::parse(&env_string("LEO_MCP_TRANSPORT", "stdio")),
            http_path,
            token: env_string("LEO_MCP_TOKEN", ""),
            write_tools: env_bool("LEO_MCP_WRITE_TOOLS", false),
            destructive_tools: env_bool("LEO_MCP_DESTRUCTIVE_TOOLS", false),
            allow_file_path: env_bool("LEO_MCP_ALLOW_FILE_PATH", false),
            allowed_roots,
            max_chars: env_u64("LEO_MCP_MAX_CHARS", 20_000) as usize,
            max_page_size: env_u64("LEO_MCP_MAX_PAGE_SIZE", 100) as usize,
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

fn env_u64(key: &str, default: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|value| value.trim().parse().ok())
        .unwrap_or(default)
}
