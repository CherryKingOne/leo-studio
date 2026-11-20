use std::sync::Arc;

use leo_core::context::Context;
use leo_core::plugin::{ChapterSplitter, Plugin};
use leo_plugin_parser_novel_regex::{NovelRegexParserPlugin, cn_to_int, resolve_chapter_key};

#[test]
fn test_chinese_number_conversion() {
    assert_eq!(cn_to_int("一"), Some(1));
    assert_eq!(cn_to_int("十二"), Some(12));
    assert_eq!(cn_to_int("一百零八"), Some(108));
    assert_eq!(cn_to_int("一千三百五十二"), Some(1352));
    assert_eq!(cn_to_int("1352"), Some(1352));
}

#[test]
fn test_chapter_fingerprint() {
    assert_eq!(resolve_chapter_key("第一章 异界重生"), "ch_1");
    assert_eq!(resolve_chapter_key("第1章 异界重生"), "ch_1");
    assert_eq!(resolve_chapter_key("第二卷 第十五章 决战"), "v_2_ch_15");
}

#[tokio::test]
async fn test_novel_parser_plugin() {
    let ctx = Context::new("test_parser");
    let plugin = NovelRegexParserPlugin::new();
    plugin.apply(&ctx).await.unwrap();

    let splitter = ctx.inject::<Arc<dyn ChapterSplitter>>().await.unwrap();

    let novel_text = r#"
前言
这是作者的碎碎念。

第一章 初入江湖
林冲提着朴刀走在雪地里。

第二章 雪夜上梁山
大雪纷飞，草料场火起。
"#;

    let chapters = splitter.split_chapters(novel_text);
    assert_eq!(chapters.len(), 3);
    assert_eq!(chapters[0].title, "前言");
    assert_eq!(chapters[1].title, "第一章 初入江湖");
    assert_eq!(chapters[2].title, "第二章 雪夜上梁山");
}
