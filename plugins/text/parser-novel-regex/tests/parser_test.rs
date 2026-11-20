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
fn test_chinese_number_overflow_is_rejected() {
    // 仍在 i64 范围内的大数照常换算
    assert_eq!(cn_to_int("一万万"), Some(100_000_000));
    assert_eq!(cn_to_int("九千九百九十九万九千九百九十九"), Some(99_999_999));
    assert_eq!(cn_to_int("9223372036854775807"), Some(i64::MAX));
    // 溢出：debug 下不得 panic，release 下不得回绕成错误数字
    assert_eq!(cn_to_int("一万万万万万"), None);
    assert_eq!(cn_to_int("九万万万万万万"), None);
    assert_eq!(cn_to_int("99999999999999999999"), None);
    assert_eq!(cn_to_int("9223372036854775808"), None);
    // 无法识别的字符不再被静默忽略
    assert_eq!(cn_to_int("十x"), None);

    // 溢出的章节号不会被当成“第 0 章”或回绕后的序号，指纹退回完整标题，避免误判重复章节
    assert_eq!(resolve_chapter_key("第一万万万万万章 溢出"), "第一万万万万万章 溢出");
    assert_eq!(
        resolve_chapter_key("第99999999999999999999章 溢出"),
        "第99999999999999999999章 溢出"
    );
    assert_ne!(
        resolve_chapter_key("第99999999999999999999章 甲"),
        resolve_chapter_key("第0章 乙")
    );
    assert_eq!(leo_plugin_parser_novel_regex::chapter_number("第一万万万万万章"), None);
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
