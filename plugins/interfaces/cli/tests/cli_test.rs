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
