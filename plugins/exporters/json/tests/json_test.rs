use std::sync::Arc;

use leo_core::ast::ScriptDocument;
use leo_core::context::Context;
use leo_core::plugin::{Exporter, Plugin};
use leo_plugin_exporter_json::JsonExporterPlugin;

#[tokio::test]
async fn test_json_exporter_plugin() {
    let ctx = Context::new("test_json_export");
    let plugin = JsonExporterPlugin::new();
    plugin.apply(&ctx).await.unwrap();

    let exporter = ctx.inject::<Arc<dyn Exporter>>().await.unwrap();
    assert_eq!(exporter.format_name(), "json");

    let script = ScriptDocument::new("p_01".into(), "测试小说".into(), "作者".into(), "玄幻".into());
    let bytes = exporter.export(&script).await.unwrap();
    let json_str = String::from_utf8(bytes).unwrap();

    assert!(json_str.contains("\"title\": \"测试小说\""));
    assert!(json_str.contains("\"author\": \"作者\""));
}
