
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

    // 仅初始化存储时不应创建音频目录
    assert!(!data_dir.join("audio").exists());

    let storage = ctx.registry::<dyn StorageProvider>().get("csv").expect("驱动未注册");

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
    let novels_csv = data_dir.join("novels.csv");
    let contents_csv = data_dir.join("novel_contents.csv");
    let sentences_csv = data_dir.join("novel_sentences.csv");
    let dialogues_csv = data_dir.join("novel_dialogues.csv");
    let audio_clips_csv = data_dir.join("novel_audio_clips.csv");

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
