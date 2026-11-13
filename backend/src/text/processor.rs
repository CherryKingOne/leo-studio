use crate::storage::csv_table::Record;
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
