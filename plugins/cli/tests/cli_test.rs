use clap::Parser;
use leo_core::plugin::{Plugin, PluginCategory};
use leo_plugin_cli::{Cli, CliPlugin, Commands, setup_cli_runtime};

#[test]
fn test_cli_plugin_manifest() {
    let plugin = CliPlugin::new();
    let manifest = plugin.manifest();
    assert_eq!(manifest.id, "leo-plugin-cli");
    assert_eq!(manifest.category, PluginCategory::Cli);
    assert!(manifest.capabilities.contains(&"cli:import".to_string()));
    assert!(manifest.capabilities.contains(&"cli:synthesize".to_string()));
}

#[tokio::test]
async fn test_cli_runtime_setup() {
    let temp_dir = tempfile::tempdir().unwrap();
    let data_dir = temp_dir.path().to_str().unwrap();

    let (_ctx, manager) = setup_cli_runtime(data_dir).await.expect("装配 CLI 环境失败");
    let manifests = manager.list_manifests().await;
    assert!(manifests.len() >= 8);
    assert!(manifests.iter().any(|m| m.id == "leo-plugin-cli"));
    assert!(manifests.iter().any(|m| m.id == "leo-plugin-tts-edge"));
    assert!(manifests.iter().any(|m| m.id == "leo-plugin-importer-txt"));
}

#[test]
fn test_cli_command_parsing() {
    // 1. import
    let cli = Cli::try_parse_from(["leo-cli", "import", "sample.txt", "--title", "自定义仙侠小说"]).unwrap();
    match cli.command {
        Commands::Import { file, title } => {
            assert_eq!(file, "sample.txt");
            assert_eq!(title, Some("自定义仙侠小说".to_string()));
        }
        _ => panic!("解析命令类型错误"),
    }

    // 2. export
    let cli = Cli::try_parse_from(["leo-cli", "export", "proj_123", "--format", "json", "-o", "out.json"]).unwrap();
    match cli.command {
        Commands::Export { project_id, format, output } => {
            assert_eq!(project_id, "proj_123");
            assert_eq!(format, "json");
            assert_eq!(output, Some("out.json".to_string()));
        }
        _ => panic!("解析命令类型错误"),
    }

    // 3. synthesize
    let cli = Cli::try_parse_from([
        "leo-cli",
        "synthesize",
        "proj_123",
        "--chapter",
        "1",
        "--voice",
        "zh-CN-XiaoxiaoNeural",
        "--speed",
        "1.2",
    ])
    .unwrap();
    match cli.command {
        Commands::Synthesize { project_id, chapter, voice, speed, .. } => {
            assert_eq!(project_id, "proj_123");
            assert_eq!(chapter, 1);
            assert_eq!(voice, "zh-CN-XiaoxiaoNeural");
            assert_eq!(speed, 1.2);
        }
        _ => panic!("解析命令类型错误"),
    }
}

#[tokio::test]
async fn test_cli_runtime_keeps_both_exporters() {
    use std::sync::Arc;

    use leo_core::plugin::Exporter;

    let temp_dir = tempfile::tempdir().unwrap();
    let (ctx, _manager) = setup_cli_runtime(temp_dir.path().to_str().unwrap())
        .await
        .expect("装配 CLI 环境失败");

    let names: Vec<String> = ctx
        .inject_all::<Arc<dyn Exporter>>()
        .await
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    assert_eq!(names, vec!["csv", "json"]);
    for format in ["csv", "json"] {
        let exporter = ctx.inject_named::<Arc<dyn Exporter>>(format).await.unwrap();
        assert_eq!(exporter.format_name(), format);
    }
}

#[tokio::test]
async fn test_cli_export_routes_through_exporter_plugins() {
    use std::sync::Arc;

    use leo_core::plugin::{Importer, StorageProvider};
    use leo_plugin_cli::execute_cli;

    let temp_dir = tempfile::tempdir().unwrap();
    let data_dir = temp_dir.path().join("data").to_str().unwrap().to_string();

    let doc = {
        let (ctx, _manager) = setup_cli_runtime(&data_dir).await.expect("装配 CLI 环境失败");
        let importer = ctx.inject::<Arc<dyn Importer>>().await.unwrap();
        let doc = importer
            .import("t.txt", "第一章 开端\n他说：“你好。”".as_bytes())
            .await
            .unwrap();
        let storage = ctx.inject::<Arc<dyn StorageProvider>>().await.unwrap();
        storage.save_script(&doc).await.unwrap();
        doc
    };

    for (format, needle) in [("csv", "章节序号"), ("JSON", "\"chapters\"")] {
        let output = temp_dir.path().join(format!("out.{format}"));
        let cli = Cli {
            command: Commands::Export {
                project_id: doc.id.clone(),
                format: format.to_string(),
                output: Some(output.to_str().unwrap().to_string()),
            },
            data_dir: data_dir.clone(),
        };
        execute_cli(cli).await.expect("导出失败");
        let text = std::fs::read_to_string(&output).unwrap();
        assert!(text.contains(needle), "{format}: {text}");
    }

    let cli = Cli {
        command: Commands::Export {
            project_id: doc.id.clone(),
            format: "xml".to_string(),
            output: Some(temp_dir.path().join("out.xml").to_str().unwrap().to_string()),
        },
        data_dir,
    };
    let err = execute_cli(cli).await.expect_err("未注册的导出格式应报错");
    assert!(err.to_string().contains("csv, json"), "{err}");
}
