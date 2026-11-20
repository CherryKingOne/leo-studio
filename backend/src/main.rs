use std::sync::Arc;

use leo_studio_backend::app::build_router;
use leo_studio_backend::config::{Settings, load_dotenv};
use leo_studio_backend::modules::text_files::service::backfill_dialogues;
use leo_studio_backend::storage::store::Store;

#[tokio::main]
async fn main() {
    load_dotenv();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let settings = Settings::from_env();
    let mut store = match Store::new(&settings) {
        Ok(store) => store,
        Err(err) => {
            tracing::error!("初始化数据存储失败：{err}");
            std::process::exit(1);
        }
    };

    match backfill_dialogues(&mut store) {
        Ok(0) => {}
        Ok(count) => tracing::info!("补齐台词片段 {count} 行"),
        Err(err) => tracing::warn!("补齐台词片段失败：{err}"),
    }

    let address = format!("{}:{}", settings.host, settings.port);
    let listener = match tokio::net::TcpListener::bind(&address).await {
        Ok(listener) => listener,
        Err(err) => {
            tracing::error!("监听 {address} 失败：{err}");
            std::process::exit(1);
        }
    };

    let app = build_router(Arc::new(settings.clone()), store).await;
    tracing::info!("Leo Studio API 已启动：http://{address}");

    if let Err(err) = axum::serve(listener, app).await {
        tracing::error!("服务异常退出：{err}");
        std::process::exit(1);
    }
}
