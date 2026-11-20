pub mod manifest;
pub mod manager;
pub mod traits;

pub use manifest::{PluginCategory, PluginManifest};
pub use manager::PluginManager;
pub use traits::{
    ChapterSplitter, DialogueSplitter, Exporter, Importer, Plugin, RawChapter, RawDialogueSegment,
    StorageProvider, SynthesisOutput, SynthesisPayload, TTSProvider, TextCleaner, VoiceInfo,
};
