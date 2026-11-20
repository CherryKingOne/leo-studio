pub mod annotation;
pub mod audio;
pub mod node;

pub use annotation::{Annotation, PausePosition};
pub use audio::{AudioClipRef, AudioFormat, mp3_duration_ms, synthesis_clip_id};
pub use node::{
    ChapterNode, ParagraphNode, ScriptDocument, SegmentKind, SegmentNode, SentenceNode, VolumeNode,
};
