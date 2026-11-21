use async_trait::async_trait;

use crate::ast::audio::{AudioClipRef, AudioFormat};
use crate::ast::node::ScriptDocument;
use crate::error::CoreResult;

/// 持久化存储扩展点
#[async_trait]
pub trait StorageProvider: Send + Sync {
    async fn save_script(&self, script: &ScriptDocument) -> CoreResult<()>;
    async fn load_script(&self, project_id: &str) -> CoreResult<Option<ScriptDocument>>;
    async fn save_audio_clip(&self, clip_id: &str, data: &[u8], format: AudioFormat) -> CoreResult<AudioClipRef>;
    async fn load_audio_clip(&self, storage_key: &str) -> CoreResult<Option<Vec<u8>>>;
}
