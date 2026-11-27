use std::fs;
use std::path::Path;

use leo_app::config::Settings;
use leo_app::modules::projects::storage_compat::{backfill_dialogues, save_file};
use leo_app::storage::csv_table::Record;
use leo_app::storage::store::Store;
use leo_app::text::dialogue::{
    DialogueSegment, ParseStatus, SegmentType, split_dialogue_line,
};
use leo_app::text::processor::{
    clean_dialogue_records, create_dialogue_records_from_sentence,
};

fn segment(content: &str, segment_type: SegmentType) -> DialogueSegment {
    DialogueSegment {
        content: content.to_string(),
        segment_type,
    }
}

#[test]
fn splits_dialogue_after_narration() {
    let result = split_dialogue_line("唐三对比比东说道：“三十年河东，三十年河西，莫欺少年穷”");
    assert_eq!(result.status, ParseStatus::Ok);
    assert_eq!(
        result.segments,
        vec![
            segment("唐三对比比东说道：", SegmentType::Narration),
            segment(
                "“三十年河东，三十年河西，莫欺少年穷”",
                SegmentType::Dialogue
            ),
        ]
    );
}

#[test]
fn splits_dialogue_with_question_mark() {
    let result = split_dialogue_line("他看着身后的中年男人道：“还有别的办法吗？”");
    assert_eq!(result.status, ParseStatus::Ok);
    assert_eq!(result.segments.len(), 2);
    assert_eq!(result.segments[0].content, "他看着身后的中年男人道：");
    assert_eq!(result.segments[0].segment_type, SegmentType::Narration);
    assert_eq!(result.segments[1].content, "“还有别的办法吗？”");
    assert_eq!(result.segments[1].segment_type, SegmentType::Dialogue);
}

#[test]
fn keeps_plain_sentence_unchanged() {
    let result = split_dialogue_line("林峰年转手开始快速跑动");
    assert_eq!(result.status, ParseStatus::Ok);
    assert_eq!(
        result.segments,
        vec![segment("林峰年转手开始快速跑动", SegmentType::Narration)]
    );
}

#[test]
fn splits_text_after_dialogue() {
    let result = split_dialogue_line("他说：“你好。”然后走了。");
    assert_eq!(result.status, ParseStatus::Ok);
    assert_eq!(
        result.segments,
        vec![
            segment("他说：", SegmentType::Narration),
            segment("“你好。”", SegmentType::Dialogue),
            segment("然后走了。", SegmentType::Narration),
        ]
    );
}

#[test]
fn splits_multiple_dialogues_in_order() {
    let result = split_dialogue_line("他说：“你好”，她说：“再见”");
    assert_eq!(result.status, ParseStatus::Ok);
    assert_eq!(
        result.segments,
        vec![
            segment("他说：", SegmentType::Narration),
            segment("“你好”", SegmentType::Dialogue),
            segment("，她说：", SegmentType::Narration),
            segment("“再见”", SegmentType::Dialogue),
        ]
    );
}

#[test]
fn splits_dialogue_at_line_start() {
    let result = split_dialogue_line("“直接以引号开头”他说道。");
    assert_eq!(result.status, ParseStatus::Ok);
    assert_eq!(
        result.segments,
        vec![
            segment("“直接以引号开头”", SegmentType::Dialogue),
            segment("他说道。", SegmentType::Narration),
        ]
    );
}

#[test]
fn does_not_add_missing_colon() {
    let result = split_dialogue_line("他说“没有冒号”");
    assert_eq!(result.status, ParseStatus::Ok);
    assert_eq!(
        result.segments,
        vec![
            segment("他说", SegmentType::Narration),
            segment("“没有冒号”", SegmentType::Dialogue),
        ]
    );
}

#[test]
fn splits_each_quoted_word() {
    let result = split_dialogue_line("他叫我“爸爸”和“妈妈”");
    assert_eq!(result.status, ParseStatus::Ok);
    assert_eq!(
        result.segments,
        vec![
            segment("他叫我", SegmentType::Narration),
            segment("“爸爸”", SegmentType::Dialogue),
            segment("和", SegmentType::Narration),
            segment("“妈妈”", SegmentType::Dialogue),
        ]
    );
}

#[test]
fn keeps_inner_single_quotes() {
    let result = split_dialogue_line("他说：“他叫我‘爸爸’”");
    assert_eq!(result.status, ParseStatus::Ok);
    assert_eq!(
        result.segments,
        vec![
            segment("他说：", SegmentType::Narration),
            segment("“他叫我‘爸爸’”", SegmentType::Dialogue),
        ]
    );
}

#[test]
fn marks_unpaired_quote_without_splitting() {
    let result = split_dialogue_line("他说：“没闭合");
    assert_eq!(result.status, ParseStatus::UnpairedQuote);
    assert_eq!(
        result.segments,
        vec![segment("他说：“没闭合", SegmentType::Narration)]
    );
}

#[test]
fn ignores_ascii_quotes() {
    let result = split_dialogue_line("He said \"hello\"");
    assert_eq!(result.status, ParseStatus::Ok);
    assert_eq!(
        result.segments,
        vec![segment("He said \"hello\"", SegmentType::Narration)]
    );
}

#[test]
fn marks_empty_content() {
    let result = split_dialogue_line("   ");
    assert_eq!(result.status, ParseStatus::Empty);
    assert!(result.segments.is_empty());
}

#[test]
fn builds_dialogue_records_from_sentence() {
    let mut sentence = Record::new();
    sentence.insert("sentence_id".into(), "s1".to_string());
    sentence.insert("novel_id".into(), "n1".to_string());
    sentence.insert("chapter_id".into(), "c1".to_string());
    sentence.insert("sentence_index".into(), "7".to_string());
    sentence.insert("content".into(), "他说：“你好。”然后走了。".to_string());

    let records = create_dialogue_records_from_sentence(&sentence);
    assert_eq!(records.len(), 3);
    assert_eq!(records[0]["dialogue_id"], "s1_0");
    assert_eq!(records[0]["parent_sentence_id"], "s1");
    assert_eq!(records[0]["source_sentence_index"], "7");
    assert_eq!(records[0]["segment_type"], "narration");
    assert_eq!(records[0]["content"], "他说：");
    assert_eq!(records[1]["dialogue_id"], "s1_1");
    assert_eq!(records[1]["segment_type"], "dialogue");
    assert_eq!(records[1]["content"], "“你好。”");
    assert_eq!(records[2]["dialogue_id"], "s1_2");
    assert_eq!(records[2]["segment_type"], "narration");
    assert_eq!(records[2]["content"], "然后走了。");
    for (index, record) in records.iter().enumerate() {
        assert_eq!(record["split_index"], index.to_string());
        assert_eq!(record["parse_status"], "ok");
    }
}

#[test]
fn cleans_dialogue_records_before_writing() {
    let mut record = Record::new();
    record.insert("dialogue_id".into(), "s1_0".to_string());
    record.insert("content".into(), "他说：“你好” (￣▽￣)".to_string());
    record.insert("word_count".into(), "99".to_string());

    let cleaned = clean_dialogue_records(vec![record]);
    assert_eq!(cleaned.len(), 1);
    assert_eq!(cleaned[0]["content"], "他说：“你好”");
    assert_eq!(cleaned[0]["word_count"], "7");

    let mut symbol = Record::new();
    symbol.insert("dialogue_id".into(), "s2_0".to_string());
    symbol.insert("content".into(), "“”".to_string());
    assert!(clean_dialogue_records(vec![symbol]).is_empty());
}

fn test_store(dir: &Path) -> (Settings, Store) {
    let settings = Settings {
        data_dir: dir.to_path_buf(),
        upload_dir: dir.join("uploads"),
        csv_dir: dir.join(".leo-studio"),
        ..Settings::default()
    };
    let store = Store::with_filenames(
        settings.upload_dir.clone(),
        settings.csv_dir.clone(),
        &settings.novels_csv_filename,
        &settings.novel_contents_csv_filename,
        &settings.novel_sentences_csv_filename,
        &settings.novel_dialogues_csv_filename,
        &settings.novel_polyphones_csv_filename,
        &settings.novel_characters_csv_filename,
    )
    .expect("初始化 Store 失败");
    (settings, store)
}

#[test]
fn upload_writes_dialogue_csv_for_every_sentence() {
    let dir = tempfile::tempdir().unwrap();
    let (settings, mut store) = test_store(dir.path());
    let content = "第1章 开端\n唐三对比比东说道：“三十年河东，三十年河西，莫欺少年穷”\n林峰年转手开始快速跑动\n他说：“你好。”然后走了。";

    save_file(
        &mut store,
        &settings,
        "test.txt",
        content.as_bytes(),
        Some("text/plain"),
        None,
    )
    .expect("保存文本文件失败");

    let path = settings.csv_dir.join("novel_dialogues.csv");
    let mut reader = csv::Reader::from_path(&path).expect("读取台词 CSV 失败");
    let headers: Vec<String> = reader
        .headers()
        .unwrap()
        .iter()
        .map(|header| header.to_string())
        .collect();
    assert_eq!(
        headers,
        vec![
            "dialogue_id",
            "novel_id",
            "chapter_id",
            "parent_sentence_id",
            "source_sentence_index",
            "split_index",
            "segment_type",
            "content",
            "word_count",
            "parse_status",
        ]
    );

    let rows: Vec<csv::StringRecord> = reader.records().map(|row| row.unwrap()).collect();
    assert_eq!(rows.len(), 6);
    assert_eq!(&rows[0][7], "唐三对比比东说道：");
    assert_eq!(&rows[1][7], "“三十年河东，三十年河西，莫欺少年穷”");
    assert_eq!(&rows[2][7], "林峰年转手开始快速跑动");
    assert_eq!(&rows[3][7], "他说：");
    assert_eq!(&rows[4][7], "“你好。”");
    assert_eq!(&rows[5][7], "然后走了。");
    assert!(rows.iter().all(|row| &row[9] == "ok"));
}

#[test]
fn backfill_restores_missing_dialogue_csv() {
    let dir = tempfile::tempdir().unwrap();
    let (settings, mut store) = test_store(dir.path());
    let content = "第1章 开端\n他说：“你好。”然后走了。";
    save_file(
        &mut store,
        &settings,
        "test.txt",
        content.as_bytes(),
        Some("text/plain"),
        None,
    )
    .expect("保存文本文件失败");

    let path = settings.csv_dir.join("novel_dialogues.csv");
    assert!(path.exists());
    fs::remove_file(&path).unwrap();
    store.dialogues.clear();

    let count = backfill_dialogues(&mut store).expect("补齐台词失败");
    assert_eq!(count, 3);
    assert_eq!(store.dialogues.find_all().len(), 3);
    assert!(path.exists());
}
