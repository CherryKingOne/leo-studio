use std::sync::Arc;

use async_trait::async_trait;
use leo_core::ast::{ChapterNode, ParagraphNode, SegmentKind, SegmentNode, SentenceNode};
use leo_core::context::Context;
use leo_core::error::CoreResult;
use leo_core::pipeline::DirtyTracker;
use leo_core::plugin::{Plugin, PluginCategory, PluginManifest, PluginManager};

// 模拟一个简单的测试插件
struct DummyCleanerPlugin;

#[async_trait]
impl Plugin for DummyCleanerPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest::new("dummy-cleaner", "测试清洗插件", "0.1.0", PluginCategory::Cleaner)
    }

    async fn apply(&self, ctx: &Context) -> CoreResult<()> {
        ctx.provide(String::from("cleaner_service_ready")).await;
        Ok(())
    }
}

#[tokio::test]
async fn test_ast_and_dirty_tracking() {
    let mut chapter = ChapterNode::new(
        "ch_1".into(),
        1,
        "v_1_ch_1".into(),
        "第一章 异界苏醒".into(),
    );

    let mut paragraph = ParagraphNode {
        id: "p_1".into(),
        index: 0,
        sentences: Vec::new(),
    };

    let mut sentence = SentenceNode::new("s_1".into(), 0, "“师姐，请自重！”林冲退后半步。".into());

    let seg1 = SegmentNode::new("seg_1".into(), 0, SegmentKind::Dialogue, "师姐，请自重！".into());
    let seg2 = SegmentNode::new("seg_2".into(), 1, SegmentKind::Narration, "林冲退后半步。".into());

    sentence.segments.push(seg1);
    sentence.segments.push(seg2);
    paragraph.sentences.push(sentence);
    chapter.paragraphs.push(paragraph);
    chapter.recalculate_word_count();

    assert_eq!(chapter.word_count, 16);

    // 刚创建时，两个 segment 均无音频，应为 dirty
    let summary = DirtyTracker::inspect_chapter(&chapter);
    assert_eq!(summary.total_segments, 2);
    assert_eq!(summary.dirty_segments, 2);
    assert!(!summary.is_fully_clean);

    // 收集待合成的片段
    let dirty_items = DirtyTracker::collect_dirty_segments(&mut chapter);
    assert_eq!(dirty_items.len(), 2);
}

#[tokio::test]
async fn test_cordis_context_and_plugin_manager() -> CoreResult<()> {
    let ctx = Context::new("test_project_1");
    let manager = PluginManager::new();

    manager.register(Arc::new(DummyCleanerPlugin)).await?;
    assert!(manager.register(Arc::new(DummyCleanerPlugin)).await.is_err(), "同 id 重复登记应报错");

    let manifests = manager.list_manifests().await;
    assert_eq!(manifests.len(), 1);
    assert_eq!(manifests[0].id, "dummy-cleaner");

    // 激活挂载所有插件
    manager.apply_all(&ctx).await?;

    // 验证依赖注入获取服务
    let service = ctx.inject::<String>().await?;
    assert_eq!(service.as_str(), "cleaner_service_ready");

    Ok(())
}

struct FailingPlugin;

#[async_trait]
impl Plugin for FailingPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest::new("failing", "必定失败", "0.1.0", PluginCategory::Other).depends_on(["dummy-cleaner"])
    }

    async fn apply(&self, _ctx: &Context) -> CoreResult<()> {
        Err(leo_core::CoreError::Internal("boom".into()))
    }
}

#[tokio::test]
async fn test_apply_all_rolls_back_on_failure() {
    let ctx = Context::new("rollback");
    let manager = PluginManager::new();
    manager.register(Arc::new(FailingPlugin)).await.unwrap();
    manager.register(Arc::new(DummyCleanerPlugin)).await.unwrap();

    // 依赖先装配：dummy-cleaner 先装上，failing 失败后 dummy-cleaner 被回滚
    assert!(manager.apply_all(&ctx).await.is_err());
    assert!(ctx.inject::<String>().await.is_err(), "失败时已装配的插件应被回滚");

    manager.unload("failing").await.unwrap();
    manager.apply_all(&ctx).await.unwrap();
    assert_eq!(ctx.inject::<String>().await.unwrap().as_str(), "cleaner_service_ready");

    manager.unload("dummy-cleaner").await.unwrap();
    assert!(ctx.inject::<String>().await.is_err(), "卸载后服务应被撤销");
}
