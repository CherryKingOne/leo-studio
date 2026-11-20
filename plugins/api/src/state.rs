use std::sync::Arc;

use tokio::sync::Mutex;

use crate::config::Settings;
use crate::error::{AppError, AppResult};
use crate::storage::store::Store;
use crate::text::TextPipeline;

/// 应用共享状态。
#[derive(Clone)]
pub struct AppState {
    pub settings: Arc<Settings>,
    pub store: Arc<Mutex<Store>>,
    pub context: leo_core::Context,
    pub plugin_manager: Arc<leo_core::PluginManager>,
}

impl AppState {
    /// 从插件上下文取出当前生效的文本清洗、分章与台词切分服务
    pub async fn text_pipeline(&self) -> AppResult<TextPipeline> {
        TextPipeline::from_context(&self.context)
            .await
            .map_err(|err| AppError::internal(format!("文本处理插件未就绪：{err}")))
    }
}
