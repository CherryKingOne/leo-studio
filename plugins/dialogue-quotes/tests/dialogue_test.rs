use std::sync::Arc;

use leo_core::ast::SegmentKind;
use leo_core::context::Context;
use leo_core::plugin::{DialogueSplitter, Plugin};
use leo_plugin_dialogue_quotes::DialogueQuotesPlugin;

#[tokio::test]
async fn test_dialogue_quotes_plugin() {
    let ctx = Context::new("test_dialogue");
    let plugin = DialogueQuotesPlugin::new();
    plugin.apply(&ctx).await.unwrap();

    let splitter = ctx.inject::<Arc<dyn DialogueSplitter>>().await.unwrap();

    // 典型的对白混排句子
    let line = "林冲拱了拱手：“娘子受惊了，且随我回房歇息。”便扶着林娘子往后院走去。";
    let segments = splitter.split_sentence(line);

    assert_eq!(segments.len(), 3);
    assert_eq!(segments[0].kind, SegmentKind::Narration);
    assert_eq!(segments[0].text, "林冲拱了拱手：");

    assert_eq!(segments[1].kind, SegmentKind::Dialogue);
    assert_eq!(segments[1].text, "“娘子受惊了，且随我回房歇息。”");

    assert_eq!(segments[2].kind, SegmentKind::Narration);
    assert_eq!(segments[2].text, "便扶着林娘子往后院走去。");
}
