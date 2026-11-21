use async_trait::async_trait;

use crate::ast::node::ScriptDocument;
use crate::error::CoreResult;

/// 导入器扩展点（注册层按文件扩展名挑选）
#[async_trait]
pub trait Importer: Send + Sync {
    fn supported_extensions(&self) -> &[&str];
    async fn import(&self, filename: &str, content: &[u8]) -> CoreResult<ScriptDocument>;
}

/// 导出器扩展点（注册层按格式名挑选）
#[async_trait]
pub trait Exporter: Send + Sync {
    fn format_name(&self) -> &str;
    fn mime_type(&self) -> &str;
    /// 文件扩展名，默认等于格式名
    fn file_extension(&self) -> &str {
        self.format_name()
    }
    async fn export(&self, script: &ScriptDocument) -> CoreResult<Vec<u8>>;
}
