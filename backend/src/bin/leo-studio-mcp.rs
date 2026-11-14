use std::sync::Arc;

use leo_studio_backend::{
    config::Settings,
    mcp::{McpHandler, McpSettings},
    storage::store::Store,
};
use rmcp::ServiceExt;
use tokio::sync::Mutex;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    // stdio 传输的 stdout 是协议通道，日志必须写 stderr。
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_env("LEO_MCP_LOG_LEVEL").unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    let settings = Arc::new(Settings::from_env());
    let mcp = Arc::new(McpSettings::from_env());
    let store = Arc::new(Mutex::new(Store::new(&settings)?));

    let handler = McpHandler::new(settings, store, mcp);
    let running = handler.serve(rmcp::transport::stdio()).await?;
    running.waiting().await?;

    Ok(())
}
