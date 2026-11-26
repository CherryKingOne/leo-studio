//! 手动联调 LLM 插件：装配 catalog → 取 LlmHub → 发一轮对话。
//!
//! 用法：
//! ```bash
//! # 先在 .env 或 shell 里配置对应供应商的 API Key
//! cargo run -p leo-plugin-catalog --example llm_chat -- "用一句话介绍有声书制作"
//!
//! # 指定驱动/模型（驱动id:模型）
//! cargo run -p leo-plugin-catalog --example llm_chat -- "你好" deepseek:deepseek-chat
//! cargo run -p leo-plugin-catalog --example llm_chat -- "你好" volcengine:ep-2026xxxxxxxx
//! ```
//!
//! 也可以先用 `LEO_PLUGINS_CONFIG=./leo-plugins.json` 指定插件配置。

use leo_core::spi::{ChatMessage, ChatRequest};
use leo_plugin_catalog::{HostEnv, LlmHub, PluginsConfig, bootstrap};

#[tokio::main(flavor = "current_thread")]
async fn main() -> leo_core::error::CoreResult<()> {
    let _ = dotenvy::dotenv();

    let prompt = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "用一句话介绍有声书制作".to_string());
    let model = std::env::args().nth(2);

    let config = PluginsConfig::load(None)?;
    let env = HostEnv {
        csv_dir: std::env::temp_dir().join("leo-llm-chat-example"),
    };
    let (ctx, _manager) = bootstrap("llm_chat_example", &env, &config).await?;
    let hub = LlmHub::of(&ctx).await;

    let drivers = hub.drivers();
    if drivers.is_empty() {
        eprintln!(
            "没有注册任何 LLM 驱动。请检查 .env / 环境变量里的 DEEPSEEK_API_KEY、ARK_API_KEY、DASHSCOPE_API_KEY。"
        );
        return Ok(());
    }
    eprintln!("已注册 LLM 驱动：");
    for driver in &drivers {
        eprintln!(
            "  - {}{}",
            driver.id,
            if driver.is_default {
                "（默认）"
            } else {
                ""
            }
        );
    }

    let response = hub
        .chat(&ChatRequest {
            model,
            messages: vec![ChatMessage::user(prompt)],
            max_tokens: Some(256),
            ..Default::default()
        })
        .await?;

    println!("model: {}", response.model);
    println!("content: {}", response.content);
    if let Some(reasoning) = &response.reasoning_content {
        println!("reasoning: {reasoning}");
    }
    println!("finish_reason: {:?}", response.finish_reason);
    if let Some(usage) = response.usage {
        println!(
            "usage: input={} output={} total={}",
            usage.input_tokens, usage.output_tokens, usage.total_tokens
        );
    }
    Ok(())
}
