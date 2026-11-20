use std::sync::Arc;

use leo_core::ast::{
    AudioFormat, ChapterNode, ParagraphNode, ScriptDocument, SegmentKind, SegmentNode, SentenceNode,
};
use leo_core::context::Context;
use leo_core::plugin::{Plugin, StorageProvider};
use leo_plugin_storage_csv::CsvStoragePlugin;
use tempfile::tempdir;

#[tokio::test]
async fn test_csv_storage_plugin_full_cycle() {
    let tmp = tempdir().unwrap();
    let data_dir = tmp.path().to_path_buf();

    let ctx = Context::new("test_storage");
    let plugin = CsvStoragePlugin::new(&data_dir);

    // 挂载插件
    plugin.apply(&ctx).await.unwrap();

    let storage = ctx.inject::<Arc<dyn StorageProvider>>().await.unwrap();

    // 构造测试用的 AST 剧本
    let mut script = ScriptDocument::new(
        "p_001".into(),
        "凡人修仙之开局合欢宗".into(),
        "忘语后生".into(),
        "仙侠修真".into(),
    );

    let mut chapter = ChapterNode::new(
        "ch_001".into(),
        1,
        "v_1_ch_1".into(),
        "第一章 师姐的阴谋".into(),
    );

    let mut paragraph = ParagraphNode {
        id: "p_001".into(),
        index: 0,
        sentences: Vec::new(),
    };

    let mut sentence = SentenceNode::new("s_001".into(), 0, "“韩立，你逃不掉的！”师姐冷笑道。".into());

    let mut seg1 = SegmentNode::new("seg_001".into(), 0, SegmentKind::Dialogue, "“韩立，你逃不掉的！”".into());
    seg1.speaker_id = Some("senior_sister".into());

    // 模拟一段合成好的音频
    let fake_mp3_data = b"ID3FAKEAUDIOSTREAMDATA";
    let clip_ref = storage.save_audio_clip("clip_seg_001", fake_mp3_data, AudioFormat::Mp3).await.unwrap();
    seg1.audio = Some(clip_ref);
    seg1.is_dirty = false;

    let seg2 = SegmentNode::new("seg_002".into(), 1, SegmentKind::Narration, "师姐冷笑道。".into());

    sentence.segments.push(seg1);
    sentence.segments.push(seg2);
    paragraph.sentences.push(sentence);
    chapter.paragraphs.push(paragraph);
    chapter.recalculate_word_count();
    script.chapters.push(chapter);
    script.recalculate_totals();

    // 1. 保存到 CSV
    storage.save_script(&script).await.unwrap();

    // 验证生成的 CSV 文件存在且不为空
    let table_dir = data_dir.join("scripts");
    let novels_csv = table_dir.join("novels.csv");
    let contents_csv = table_dir.join("novel_contents.csv");
    let sentences_csv = table_dir.join("novel_sentences.csv");
    let dialogues_csv = table_dir.join("novel_dialogues.csv");
    let audio_clips_csv = table_dir.join("novel_audio_clips.csv");

    assert!(novels_csv.exists());
    assert!(contents_csv.exists());
    assert!(sentences_csv.exists());
    assert!(dialogues_csv.exists());
    assert!(audio_clips_csv.exists());

    // 2. 从 CSV 读取还原 AST 剧本
    let loaded = storage.load_script("p_001").await.unwrap().expect("未能读到剧本");
    assert_eq!(loaded.id, "p_001");
    assert_eq!(loaded.title, "凡人修仙之开局合欢宗");
    assert_eq!(loaded.chapters.len(), 1);

    let loaded_chapter = &loaded.chapters[0];
    assert_eq!(loaded_chapter.title, "第一章 师姐的阴谋");
    assert_eq!(loaded_chapter.paragraphs[0].sentences.len(), 1);

    let loaded_sentence = &loaded_chapter.paragraphs[0].sentences[0];
    assert_eq!(loaded_sentence.segments.len(), 2);

    // 验证台词片段与绑定的音频
    let loaded_seg1 = &loaded_sentence.segments[0];
    assert_eq!(loaded_seg1.kind, SegmentKind::Dialogue);
    assert_eq!(loaded_seg1.speaker_id.as_deref(), Some("senior_sister"));
    assert!(!loaded_seg1.is_dirty);

    let loaded_audio = loaded_seg1.audio.as_ref().expect("必须挂载了音频引用");
    assert_eq!(loaded_audio.clip_id, "clip_seg_001");

    // 3. 验证音频二进制回读
    let audio_bytes = storage.load_audio_clip(&loaded_audio.storage_key).await.unwrap().expect("音频字节必须存在");
    assert_eq!(audio_bytes, fake_mp3_data);
}

#[tokio::test]
async fn test_csv_storage_does_not_touch_rest_store_tables() {
    let tmp = tempdir().unwrap();
    let data_dir = tmp.path().to_path_buf();

    // 旧版本两套存储共用同名文件：REST 行没有 updated_at，插件行有
    std::fs::write(
        data_dir.join("novels.csv"),
        "novel_id,title,format,chapter_count,total_words,line_count,created_at,author,genre,updated_at\n\
         rest_1,REST 小说,txt,1,10,3,2026-01-01T00:00:00+00:00,,,\n\
         old_plugin,旧导入,,1,5,,2026-01-01T00:00:00+00:00,佚名,,2026-01-02T00:00:00+00:00\n",
    )
    .unwrap();
    std::fs::write(
        data_dir.join("novel_contents.csv"),
        "chapter_id,novel_id,chapter_index,chapter_title,content,word_count,fingerprint\n\
         rest_ch,rest_1,0,第一章,正文,10,\n\
         old_ch,old_plugin,1,旧章节,,5,v_1_ch_1\n",
    )
    .unwrap();
    std::fs::write(
        data_dir.join("novel_sentences.csv"),
        "sentence_id,novel_id,chapter_id,sentence_index,content,word_count\n\
         rest_s,rest_1,rest_ch,0,正文,2\n\
         old_s,old_plugin,old_ch,0,旧句子。,4\n",
    )
    .unwrap();
    std::fs::write(
        data_dir.join("novel_dialogues.csv"),
        "dialogue_id,novel_id,chapter_id,content,segment_id,sentence_id,segment_index,segment_kind\n\
         rest_d,rest_1,rest_ch,正文,,,,\n\
         ,old_plugin,old_ch,旧句子。,old_seg,old_s,0,narration\n",
    )
    .unwrap();
    let rest_novels_before = std::fs::read(data_dir.join("novels.csv")).unwrap();

    let ctx = Context::new("test_storage_isolation");
    CsvStoragePlugin::new(&data_dir).apply(&ctx).await.unwrap();
    let storage = ctx.inject::<Arc<dyn StorageProvider>>().await.unwrap();

    // 旧数据中插件写入的剧本仍可读取，REST 行不会被迁入
    let migrated = storage.load_script("old_plugin").await.unwrap().expect("旧剧本应可读取");
    assert_eq!(migrated.title, "旧导入");
    assert_eq!(migrated.chapters.len(), 1);
    assert_eq!(migrated.chapters[0].paragraphs[0].sentences[0].segments.len(), 1);
    assert!(storage.load_script("rest_1").await.unwrap().is_none());

    let mut script = ScriptDocument::new("p_new".into(), "新导入".into(), "".into(), "".into());
    let mut chapter = ChapterNode::new("ch_new".into(), 1, "v_1_ch_1".into(), "第一章".into());
    let mut paragraph = ParagraphNode {
        id: "p_ch_new".into(),
        index: 0,
        sentences: Vec::new(),
    };
    let mut sentence = SentenceNode::new("s_new".into(), 0, "你好。".into());
    sentence.segments.push(SegmentNode::new(
        "seg_new".into(),
        0,
        SegmentKind::Narration,
        "你好。".into(),
    ));
    paragraph.sentences.push(sentence);
    chapter.paragraphs.push(paragraph);
    script.chapters.push(chapter);
    storage.save_script(&script).await.unwrap();

    // 插件保存不再改写 REST Store 的表
    assert_eq!(std::fs::read(data_dir.join("novels.csv")).unwrap(), rest_novels_before);

    // 重新打开后两个剧本都在
    let ctx = Context::new("test_storage_reopen");
    CsvStoragePlugin::new(&data_dir).apply(&ctx).await.unwrap();
    let storage = ctx.inject::<Arc<dyn StorageProvider>>().await.unwrap();
    assert!(storage.load_script("p_new").await.unwrap().is_some());
    assert!(storage.load_script("old_plugin").await.unwrap().is_some());
}
