//! 扩展点契约（SPI）。
//!
//! 内核只定义“插件之间说什么话”，不包含任何实现。每个扩展点都有一张同名注册表
//! （`Registry<dyn TTSProvider>` 等），子插件往表里注册，注册层插件负责挑选与对外服务。

pub mod exchange;
pub mod storage;
pub mod text;
pub mod tts;

pub use exchange::{Exporter, Importer};
pub use storage::StorageProvider;
pub use text::{ChapterSplitter, DialogueSplitter, RawChapter, RawDialogueSegment, TextCleaner};
pub use tts::{SynthesisOutput, SynthesisPayload, TTSProvider, VoiceInfo};
