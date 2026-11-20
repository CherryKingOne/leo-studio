pub mod manifest;
pub mod manager;
pub mod traits;

pub use manifest::{PluginCategory, PluginManifest};
pub use manager::{FailurePolicy, PluginFailure, PluginManager};
pub use traits::{
    ChapterSplitter, DialogueParseStatus, DialogueSplitter, Exporter, Importer, Plugin,
    RawChapter, RawDialogueLine, RawDialogueSegment, ScriptSummary, StorageProvider, SynthesisOutput,
    SynthesisPayload, TTSProvider, TextCleaner, VoiceInfo,
};
