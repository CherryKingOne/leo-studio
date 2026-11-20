use std::collections::HashMap;

use crate::storage::csv_table::{Record, str_field};
use crate::text::dialogue::{SegmentType, split_dialogue_line};
use crate::text::kaomoji::clean_line_text;
use crate::text::parsers::count_words;

/// 将章节正文按行切分为句子列表。
pub fn split_sentences_by_lines(content: &str) -> Vec<String> {
    if content.trim().is_empty() {
        return Vec::new();
    }

    let mut sentences = Vec::new();
    for line in content.lines() {
        let cleaned = clean_line_text(line);
        if !cleaned.is_empty() {
            sentences.push(cleaned);
        }
    }
    sentences
}

/// 为句子列表构建切句记录。
pub fn build_sentence_records(
    novel_id: &str,
    chapter_id: &str,
    sentences: &[String],
) -> Vec<Record> {
    sentences
        .iter()
        .enumerate()
        .map(|(index, sentence)| {
            let mut record = Record::new();
            record.insert(
                "sentence_id".into(),
                uuid::Uuid::new_v4().simple().to_string(),
            );
            record.insert("novel_id".into(), novel_id.to_string());
            record.insert("chapter_id".into(), chapter_id.to_string());
            record.insert("sentence_index".into(), index.to_string());
            record.insert("content".into(), sentence.clone());
            record.insert("word_count".into(), count_words(sentence).to_string());
            record
        })
        .collect()
}

/// 对单个章节的内容进行切句并构建对应的记录。
pub fn create_sentence_records(novel_id: &str, chapter_id: &str, content: &str) -> Vec<Record> {
    let sentences = split_sentences_by_lines(content);
    build_sentence_records(novel_id, chapter_id, &sentences)
}

/// 将一条切句记录按成对中文双引号拆分为台词片段记录。
pub fn create_dialogue_records_from_sentence(sentence: &Record) -> Vec<Record> {
    let novel_id = str_field(sentence, "novel_id");
    let chapter_id = str_field(sentence, "chapter_id");
    let sentence_id = str_field(sentence, "sentence_id");
    let source_sentence_index = str_field(sentence, "sentence_index");
    let content = str_field(sentence, "content");
    let split = split_dialogue_line(content);

    if split.segments.is_empty() {
        return vec![dialogue_record(
            novel_id,
            chapter_id,
            sentence_id,
            source_sentence_index,
            0,
            SegmentType::Narration,
            content,
            split.status,
        )];
    }

    split
        .segments
        .iter()
        .enumerate()
        .map(|(index, segment)| {
            dialogue_record(
                novel_id,
                chapter_id,
                sentence_id,
                source_sentence_index,
                index,
                segment.segment_type,
                &segment.content,
                split.status,
            )
        })
        .collect()
}

/// 台词片段写入 CSV 前统一执行文本清洗，清洗后为空的片段会被丢弃。
pub fn clean_dialogue_records(records: Vec<Record>) -> Vec<Record> {
    let mut cleaned = Vec::new();
    let mut counters: HashMap<String, usize> = HashMap::new();

    for mut record in records {
        let content = clean_line_text(str_field(&record, "content"));
        if content.is_empty() {
            continue;
        }

        let parent_sentence_id = str_field(&record, "parent_sentence_id").to_string();
        let source_sentence_index = str_field(&record, "source_sentence_index").to_string();
        let split_index = counters.entry(parent_sentence_id.clone()).or_insert(0);
        let current_index = *split_index;
        *split_index += 1;

        let dialogue_id = if parent_sentence_id.is_empty() {
            format!("sentence_{source_sentence_index}_{current_index}")
        } else {
            format!("{parent_sentence_id}_{current_index}")
        };

        record.insert("dialogue_id".into(), dialogue_id);
        record.insert("split_index".into(), current_index.to_string());
        record.insert("content".into(), content.clone());
        record.insert("word_count".into(), count_words(&content).to_string());
        cleaned.push(record);
    }

    cleaned
}

#[allow(clippy::too_many_arguments)]
fn dialogue_record(
    novel_id: &str,
    chapter_id: &str,
    sentence_id: &str,
    source_sentence_index: &str,
    split_index: usize,
    segment_type: SegmentType,
    content: &str,
    status: crate::text::dialogue::ParseStatus,
) -> Record {
    let dialogue_id = if sentence_id.is_empty() {
        format!("sentence_{source_sentence_index}_{split_index}")
    } else {
        format!("{sentence_id}_{split_index}")
    };

    let mut record = Record::new();
    record.insert("dialogue_id".into(), dialogue_id);
    record.insert("novel_id".into(), novel_id.to_string());
    record.insert("chapter_id".into(), chapter_id.to_string());
    record.insert("parent_sentence_id".into(), sentence_id.to_string());
    record.insert(
        "source_sentence_index".into(),
        source_sentence_index.to_string(),
    );
    record.insert("split_index".into(), split_index.to_string());
    record.insert("segment_type".into(), segment_type.as_str().to_string());
    record.insert("content".into(), content.to_string());
    record.insert("word_count".into(), count_words(content).to_string());
    record.insert("parse_status".into(), status.as_str().to_string());
    record
}
