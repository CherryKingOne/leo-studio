use std::sync::Arc;

use leo_core::ast::SegmentKind;
use leo_core::context::Context;
use leo_core::plugin::{Importer, PluginManager, TextCleaner};
use leo_plugin_cleaner_kaomoji::KaomojiCleanerPlugin;
use leo_plugin_dialogue_quotes::DialogueQuotesPlugin;
use leo_plugin_importer_txt::TxtImporterPlugin;
use leo_plugin_parser_novel_regex::NovelRegexParserPlugin;

#[tokio::test]
async fn test_txt_importer_plugin() {
    let ctx = Context::new("test_txt_import");
    let manager = PluginManager::new();
    manager.register(Arc::new(TxtImporterPlugin::new())).await.unwrap();
    manager.register(Arc::new(KaomojiCleanerPlugin::new())).await.unwrap();
    manager.register(Arc::new(NovelRegexParserPlugin::new())).await.unwrap();
    manager.register(Arc::new(DialogueQuotesPlugin::new())).await.unwrap();
    manager.apply_all(&ctx).await.unwrap();

    let importer = ctx.inject::<Arc<dyn Importer>>().await.unwrap();
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

struct ShoutCleaner;

impl TextCleaner for ShoutCleaner {
    fn clean_line(&self, text: &str) -> String {
        text.trim().replace("林冲", "豹子头")
    }
}

#[tokio::test]
async fn test_txt_importer_uses_injected_cleaner() {
    let ctx = Context::new("test_txt_import_custom");
    let cleaner: Arc<dyn TextCleaner> = Arc::new(ShoutCleaner);
    ctx.provide_named("shout", cleaner).await.unwrap();

    let manager = PluginManager::new();
    manager.register(Arc::new(KaomojiCleanerPlugin::new())).await.unwrap();
    manager.register(Arc::new(NovelRegexParserPlugin::new())).await.unwrap();
    manager.register(Arc::new(DialogueQuotesPlugin::new())).await.unwrap();
    manager.register(Arc::new(TxtImporterPlugin::new())).await.unwrap();
    manager.apply_all(&ctx).await.unwrap();

    let importer = ctx.inject::<Arc<dyn Importer>>().await.unwrap();
    let script = importer
        .import("t.txt", "第一章 风雪\n林冲冷笑一声。(￣▽￣)".as_bytes())
        .await
        .unwrap();
    let sentence = &script.chapters[0].paragraphs[0].sentences[0];
    assert_eq!(sentence.raw_text, "豹子头冷笑一声。(￣▽￣)");

    let missing = PluginManager::new();
    missing.register(Arc::new(TxtImporterPlugin::new())).await.unwrap();
    let err = missing
        .apply_all(&Context::new("no_text_plugins"))
        .await
        .expect_err("缺少文本处理插件时应报错");
    assert!(err.to_string().contains("leo-plugin-cleaner-kaomoji"), "{err}");
}
