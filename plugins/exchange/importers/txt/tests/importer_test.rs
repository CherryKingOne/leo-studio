
use leo_core::ast::SegmentKind;
use leo_core::context::Context;
use leo_core::plugin::{Importer, Plugin};
use leo_plugin_cleaner_kaomoji::KaomojiCleanerPlugin;
use leo_plugin_dialogue_quotes::DialogueQuotesPlugin;
use leo_plugin_importer_txt::TxtImporterPlugin;
use leo_plugin_parser_novel_regex::NovelRegexParserPlugin;

#[tokio::test]
async fn test_txt_importer_plugin() {
    let ctx = Context::new("test_txt_import");
    // 导入器只认文本注册层：清洗 / 分章 / 台词切分由各自子插件注册
    KaomojiCleanerPlugin::new().apply(&ctx).await.unwrap();
    NovelRegexParserPlugin::new().apply(&ctx).await.unwrap();
    DialogueQuotesPlugin::new().apply(&ctx).await.unwrap();
    let plugin = TxtImporterPlugin::new();
    plugin.apply(&ctx).await.unwrap();

    let importer = ctx.registry::<dyn Importer>().get("txt").expect("驱动未注册");
    assert_eq!(importer.supported_extensions(), &["txt"]);

    let sample_novel = r#"
第一章 绝境反击
林冲冷笑一声：“梁山虽大，却容不下尔等宵小！”
一枪刺出，风雪骤停。
"#;

    let filename = "水浒后传——施耐庵——古典名著.txt";
    let script = importer.import(filename, sample_novel.as_bytes()).await.unwrap();

    assert_eq!(script.title, "水浒后传");
    assert_eq!(script.author, "施耐庵");
    assert_eq!(script.genre, "古典名著");
    assert_eq!(script.chapters.len(), 1);

    let ch = &script.chapters[0];
    assert_eq!(ch.title, "第一章 绝境反击");
    assert_eq!(ch.fingerprint, "ch_1");
    assert_eq!(ch.paragraphs[0].sentences.len(), 2);

    let s1 = &ch.paragraphs[0].sentences[0];
    assert_eq!(s1.segments.len(), 2);
    assert_eq!(s1.segments[0].kind, SegmentKind::Narration);
    assert_eq!(s1.segments[1].kind, SegmentKind::Dialogue);
}

#[tokio::test]
async fn test_txt_importer_reports_missing_text_plugins() {
    let ctx = Context::new("no_text_plugins");
    TxtImporterPlugin::new().apply(&ctx).await.unwrap();
    let importer = ctx.registry::<dyn Importer>().get("txt").unwrap();
    let err = importer.import("a.txt", "第一章 x\n正文".as_bytes()).await.unwrap_err();
    assert!(err.to_string().contains("分章插件"), "{err}");
}
