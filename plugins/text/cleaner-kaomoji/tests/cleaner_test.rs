
use leo_core::context::Context;
use leo_core::plugin::{Plugin, TextCleaner};
use leo_plugin_cleaner_kaomoji::KaomojiCleanerPlugin;

#[tokio::test]
async fn test_kaomoji_cleaner_plugin() {
    let ctx = Context::new("test_cleaner");
    let plugin = KaomojiCleanerPlugin::new();

    plugin.apply(&ctx).await.unwrap();

    let cleaner = ctx.registry::<dyn TextCleaner>().get("kaomoji").expect("驱动未注册");

    // 含有颜文字和破折号的典型网络小说文本
    let dirty_line = "林冲心里一惊(╯°□°)╯，师姐怎么会在这里？！";
    let cleaned = cleaner.clean_line(dirty_line);

    // 颜文字应被洗掉，正文保留
    assert!(!cleaned.contains("(╯°□°)╯"));
    assert!(cleaned.contains("林冲心里一惊"));
    assert!(cleaned.contains("师姐怎么会在这里"));
}
