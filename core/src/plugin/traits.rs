use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::ast::audio::{AudioClipRef, AudioFormat};
use crate::ast::node::{ScriptDocument, SegmentKind};
use crate::context::Context;
use crate::error::CoreResult;
use crate::plugin::manifest::PluginManifest;

/// 插件根接口：参考 Cordis 的 apply(ctx) 哲学。
#[async_trait]
pub trait Plugin: Send + Sync {
    /// 插件的元数据清单
    fn manifest(&self) -> PluginManifest;

    /// 挂载插件：在此方法内向 Context 注册服务、监听事件或注入中间件
    async fn apply(&self, ctx: &Context) -> CoreResult<()>;

    /// 卸载插件：释放 apply 阶段持有的外部资源；其注册的服务由 PluginManager 自动撤销
    async fn dispose(&self, _ctx: &Context) -> CoreResult<()> {
        Ok(())
    }
}

/// 文本清洗扩展点
pub trait TextCleaner: Send + Sync {
    fn clean_line(&self, text: &str) -> String;
}

/// 章节拆分结果
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawChapter {
    pub title: String,
    pub content: String,
}

/// 台词拆分片段结果
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawDialogueSegment {
    pub text: String,
    pub kind: SegmentKind,
}

/// 台词切分的解析状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogueParseStatus {
    Ok,
    UnpairedQuote,
    Empty,
}

impl DialogueParseStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::UnpairedQuote => "unpaired_quote",
            Self::Empty => "empty",
        }
    }
}

/// 单句台词切分结果（含解析状态）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawDialogueLine {
    pub segments: Vec<RawDialogueSegment>,
    pub status: DialogueParseStatus,
}

/// 章节分章扩展点
pub trait ChapterSplitter: Send + Sync {
    fn split_chapters(&self, full_text: &str) -> Vec<RawChapter>;
    fn resolve_fingerprint(&self, title: &str) -> String;

    /// 从章节标题解析章节序号，用于追加章节时排序；无法识别时返回 None
    fn chapter_number(&self, _title: &str) -> Option<i64> {
        None
    }
}

/// 单句内台词/旁白切分扩展点
pub trait DialogueSplitter: Send + Sync {
    fn split_sentence(&self, sentence: &str) -> Vec<RawDialogueSegment>;

    /// 切分并给出解析状态；默认实现只区分空句与正常句
    fn analyze_sentence(&self, sentence: &str) -> RawDialogueLine {
        let status = if sentence.trim().is_empty() {
            DialogueParseStatus::Empty
        } else {
            DialogueParseStatus::Ok
        };
        RawDialogueLine {
            segments: self.split_sentence(sentence),
            status,
        }
    }
}

/// 语音合成输入载荷
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesisPayload {
    pub text: String,
    /// 目标声线（如 zh-CN-XiaoxiaoNeural / cosyvoice:custom_voice_1）
    pub voice: String,
    /// 语速倍率 0.5 ~ 2.0
    pub speed: f32,
    /// 音调调整 -100% ~ +100%
    pub pitch: f32,
    /// 情绪标识
    pub emotion: Option<String>,
}

/// 语音合成结果
#[derive(Debug, Clone)]
pub struct SynthesisOutput {
    pub audio_bytes: Vec<u8>,
    pub format: AudioFormat,
    pub sample_rate: u32,
    pub duration_ms: u64,
}

/// 声线信息描述
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceInfo {
    pub id: String,
    pub name: String,
    pub gender: String,
    pub locale: String,
    pub description: String,
}

/// TTS 语音合成扩展点
#[async_trait]
pub trait TTSProvider: Send + Sync {
    fn provider_id(&self) -> &str;
    async fn list_voices(&self) -> CoreResult<Vec<VoiceInfo>>;
    async fn synthesize(&self, payload: &SynthesisPayload) -> CoreResult<SynthesisOutput>;
}

/// 持久化存储扩展点
#[async_trait]
pub trait StorageProvider: Send + Sync {
    async fn save_script(&self, script: &ScriptDocument) -> CoreResult<()>;
    async fn load_script(&self, project_id: &str) -> CoreResult<Option<ScriptDocument>>;
    async fn save_audio_clip(&self, clip_id: &str, data: &[u8], format: AudioFormat) -> CoreResult<AudioClipRef>;
    async fn load_audio_clip(&self, storage_key: &str) -> CoreResult<Option<Vec<u8>>>;
}

/// 导入器扩展点
#[async_trait]
pub trait Importer: Send + Sync {
    fn supported_extensions(&self) -> &[&str];
    async fn import(&self, filename: &str, content: &[u8]) -> CoreResult<ScriptDocument>;
}

/// 导出器扩展点
#[async_trait]
pub trait Exporter: Send + Sync {
    fn format_name(&self) -> &str;
    fn mime_type(&self) -> &str;
    async fn export(&self, script: &ScriptDocument) -> CoreResult<Vec<u8>>;
}
