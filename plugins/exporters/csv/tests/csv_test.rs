use std::sync::Arc;

use leo_core::ast::{ChapterNode, ParagraphNode, ScriptDocument, SegmentKind, SegmentNode, SentenceNode};
use leo_core::context::Context;
use leo_core::plugin::{Exporter, Plugin};
use leo_plugin_exporter_csv::CsvExporterPlugin;

#[tokio::test]
async fn test_csv_exporter_plugin() {
    let ctx = Context::new("test_csv_export");
    let plugin = CsvExporterPlugin::new();
    plugin.apply(&ctx).await.unwrap();

    let exporter = ctx.inject::<Arc<dyn Exporter>>().await.unwrap();
    assert_eq!(exporter.format_name(), "csv");

    let mut script = ScriptDocument::new("p_01".into(), "水浒传".into(), "施耐庵".into(), "名著".into());
    let mut ch = ChapterNode::new("ch_1".into(), 1, "ch_1".into(), "第一章 风雪山神庙".into());
    let mut p = ParagraphNode { id: "p1".into(), index: 0, sentences: Vec::new() };
    let mut s = SentenceNode::new("s1".into(), 0, "林冲大喝一声：“受死！”".into());
    s.segments.push(SegmentNode::new("seg1".into(), 0, SegmentKind::Narration, "林冲大喝一声：".into()));
    s.segments.push(SegmentNode::new("seg2".into(), 1, SegmentKind::Dialogue, "“受死！”".into()));
    p.sentences.push(s);
    ch.paragraphs.push(p);
    script.chapters.push(ch);

    let bytes = exporter.export(&script).await.unwrap();
    let csv_str = String::from_utf8_lossy(&bytes);

    assert!(csv_str.contains("章节序号,章节标题,句子序号"));
    assert!(csv_str.contains("第一章 风雪山神庙"));
    assert!(csv_str.contains("台词"));
    assert!(csv_str.contains("“受死！”"));
}
