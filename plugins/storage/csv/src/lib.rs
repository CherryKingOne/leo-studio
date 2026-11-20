pub mod schema;
pub mod store;
pub mod table;

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use leo_core::context::Context;
use leo_core::error::CoreResult;
use leo_core::plugin::{Plugin, PluginCategory, PluginManifest, StorageProvider};

pub use store::{CsvStorageService, is_safe_clip_id};
pub use table::{CsvTable, Record, bool_field, int_field, str_field};

pub struct CsvStoragePlugin {
    data_dir: PathBuf,
}

impl Default for CsvStoragePlugin {
    fn default() -> Self {
        Self::new(".leo-studio")
    }
}

impl CsvStoragePlugin {
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            data_dir: data_dir.into(),
        }
    }
}

#[async_trait]
impl Plugin for CsvStoragePlugin {
    fn manifest(&self) -> PluginManifest {
        let mut manifest = PluginManifest::new(
            "leo-plugin-storage-csv",
            "CSV 纯文本持久化存储插件",
            "0.1.0",
            PluginCategory::Storage,
        );
        manifest.description =
            "采用透明、易审阅的纯文本 CSV 表存储剧本 AST 结构、多音字注音与音频切片索引。".into();
        manifest.capabilities = vec![
            "storage:script".into(),
            "storage:audio".into(),
            "storage:atomic".into(),
        ];
        manifest
    }

    async fn apply(&self, ctx: &Context) -> CoreResult<()> {
        let service = CsvStorageService::new(self.data_dir.clone())?;
        let provider: Arc<dyn StorageProvider> = Arc::new(service);
        ctx.provide_named("csv", provider).await?;
        tracing::info!(
            "已挂载 CsvStoragePlugin，数据存储目录: {:?}",
            self.data_dir
        );
        Ok(())
    }
}
