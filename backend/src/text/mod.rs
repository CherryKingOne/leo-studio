pub mod dialogue;
pub mod kaomoji;
pub mod parsers;
pub mod processor;

pub use dialogue::{DialogueLine, DialogueSegment, ParseStatus, SegmentType, split_dialogue_line};
pub use parsers::{clean_text, count_words, parse_chapters, parse_novel_filename};
pub use processor::{
    build_sentence_records, clean_dialogue_records, create_dialogue_records_from_sentence,
    create_sentence_records, split_sentences_by_lines,
};
