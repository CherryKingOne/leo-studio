use std::borrow::Cow;
use std::sync::Arc;

use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler,
    model::{
        CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock,
        GetPromptRequestParams, GetPromptResponse, Implementation, ListPromptsResult,
        ListResourceTemplatesResult, ListResourcesResult, ListToolsResult, PaginatedRequestParams,
        ProtocolVersion, ReadResourceRequestParams, ReadResourceResponse, ServerCapabilities,
        ServerConfig, Tool,
    },
    service::RequestContext,
};
use serde_json::json;
use tokio::sync::Mutex;

use leo_plugin_api::config::Settings;
use leo_plugin_api::state::AppState;
use leo_plugin_api::storage::store::Store;

use super::config::McpSettings;
use super::registry::definitions;
use super::{prompts, resources};

/// MCP 服务处理器；挂载在现有 Axum 进程内。
#[derive(Clone)]
pub struct McpHandler {
    pub(crate) settings: Arc<Settings>,
    pub(crate) store: Arc<Mutex<Store>>,
    pub(crate) mcp: Arc<McpSettings>,
    pub(crate) tools: Arc<Vec<Tool>>,
}

impl McpHandler {
    pub fn new(state: AppState, mcp: Arc<McpSettings>) -> Self {
        let tools = Arc::new(definitions(&mcp));
        Self {
            settings: state.settings,
            store: state.store,
            mcp,
            tools,
        }
    }
}

impl ServerHandler for McpHandler {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .enable_prompts()
                .build(),
        )
        .with_server_info(Implementation::new("leo-studio", env!("CARGO_PKG_VERSION")))
        .with_instructions(
            "Leo Studio 的 MCP 服务：查询与制作有声书剧目、章节、切句与多音字标注。\n\
             只读工具默认可用；写入与破坏性工具需服务端开启对应开关。\n\
             大数据优先使用 leo_get_chapter 的 max_chars 截断或 leo_search_chapters 定位。",
        )
    }

    fn supported_protocol_versions(&self) -> Cow<'static, [ProtocolVersion]> {
        // 现代无状态版本优先；保留旧版握手，兼容只支持 2025-06-18 的客户端。
        Cow::Owned(vec![
            ProtocolVersion::V_2026_07_28,
            ProtocolVersion::V_2025_11_25,
            ProtocolVersion::V_2025_06_18,
        ])
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        Ok(ListToolsResult::with_all_items(self.definitions()).with_ttl_ms(5_000))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, McpError> {
        if !self.has_tool(&request.name) {
            return Err(McpError::invalid_params(
                format!("未知工具：{}", request.name),
                None,
            ));
        }

        let empty = serde_json::Map::new();
        let args = request.arguments.as_ref().unwrap_or(&empty);

        match self.dispatch(&request.name, args).await {
            Ok(value) => Ok(CallToolResult::structured(value).into()),
            Err(err) => {
                let mut result =
                    CallToolResult::error(vec![ContentBlock::text(err.message.clone())]);
                result.structured_content = Some(json!({
                    "code": err.code,
                    "message": err.message
                }));
                Ok(result.into())
            }
        }
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        Ok(ListResourcesResult::with_all_items(resources::list()))
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, McpError> {
        Ok(ListResourceTemplatesResult::with_all_items(
            resources::templates(),
        ))
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResponse, McpError> {
        match resources::read(self, &request.uri).await {
            Ok(result) => Ok(result.into()),
            Err(err) => Err(McpError::invalid_params(err.message, None)),
        }
    }

    async fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, McpError> {
        Ok(ListPromptsResult::with_all_items(prompts::list()))
    }

    async fn get_prompt(
        &self,
        request: GetPromptRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<GetPromptResponse, McpError> {
        let empty = serde_json::Map::new();
        let args = request.arguments.as_ref().unwrap_or(&empty);
        match prompts::render(self, &request.name, args).await {
            Ok(result) => Ok(result.into()),
            Err(err) => Err(McpError::invalid_params(err.message, None)),
        }
    }
}
