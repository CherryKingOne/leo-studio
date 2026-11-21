use clap::Parser;
use leo_plugin_cli::{Cli, execute_cli};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .init();

    let cli = Cli::parse();
    if let Err(err) = execute_cli(cli).await {
        eprintln!("\n❌ 执行失败: {err}");
        std::process::exit(1);
    }
}
