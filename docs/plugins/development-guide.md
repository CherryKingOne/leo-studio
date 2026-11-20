# 🛠️ Leo Studio 插件实战开发教程 (Step-by-Step Development Guide)

本教程手把手带领你从零开始编写一个符合 Leo Studio 契约规范的扩展插件。我们将以最常见的两个场景为例：
1. **实战一：开发一个自定义 TTS 语音合成插件（以本地/云端 CosyVoice 为例）**
2. **实战二：开发一个自定义文档导入器插件（以 EPUB 电子书导入为例）**

---

## 准备工作

确保你的开发环境满足：
- Rust 1.85+ (Stable)
- 已克隆 `leo-studio` 仓库并在根目录下

---

## 实战一：开发自定义 TTS 语音合成插件

假设我们要接入一个兼容 OpenAI 格式或本地部署的 **CosyVoice** 语音服务。

### 第一步：在 `plugins/tts/` 目录下创建 Crate

在终端执行：
```bash
cargo new --lib plugins/tts/cosyvoice
```

### 第二步：配置 `Cargo.toml`

编辑 `plugins/tts/cosyvoice/Cargo.toml`：

```toml
[package]
name = "leo-plugin-tts-cosyvoice"
version = "0.1.0"
edition = "2024"
description = "CosyVoice 高保真拟真语音合成插件"

[dependencies]
leo-core = { path = "../../../core" }
async-trait = "0.1"
reqwest = { version = "0.12", features = ["json"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tracing = "0.1"

[dev-dependencies]
tokio = { version = "1.43", features = ["full"] }
```

并在根目录 `Cargo.toml` 的 `[workspace]` 中添加成员 `"plugins/tts/cosyvoice"`。

### 第三步：编写插件实现代码

编辑 `plugins/tts/cosyvoice/src/lib.rs`：

```rust
use std::sync::Arc;
use async_trait::async_trait;
use leo_core::ast::AudioFormat;
use leo_core::context::Context;
use leo_core::error::{CoreError, CoreResult};
use leo_core::plugin::{
    Plugin, PluginCategory, PluginManifest, SynthesisOutput, SynthesisPayload, TTSProvider, VoiceInfo,
};

/// 1. 定义插件主体结构
pub struct CosyVoicePlugin {
    pub api_endpoint: String,
}

impl CosyVoicePlugin {
    pub fn new(api_endpoint: impl Into<String>) -> Self {
        Self {
            api_endpoint: api_endpoint.into(),
        }
    }
}

/// 2. 实现 Plugin 根接口
#[async_trait]
impl Plugin for CosyVoicePlugin {
    fn manifest(&self) -> PluginManifest {
        let mut manifest = PluginManifest::new(
            "leo-plugin-tts-cosyvoice",
            "CosyVoice 拟真语音合成",
            "0.1.0",
            PluginCategory::Tts,
        );
        manifest.description = "基于 CosyVoice 开源大模型的拟真情感语音合成插件".into();
        manifest.capabilities = vec!["tts:synthesize".into(), "tts:list_voices".into()];
        manifest
    }

    async fn apply(&self, ctx: &Context) -> CoreResult<()> {
        let provider: Arc<dyn TTSProvider> = Arc::new(CosyVoiceProvider {
            api_endpoint: self.api_endpoint.clone(),
        });
        // 注册到微内核 IoC 容器
        ctx.provide(provider).await;
        tracing::info!("CosyVoice 插件挂载成功，已注入 TTSProvider");
        Ok(())
    }
}

/// 3. 实现 TTSProvider 业务契约
pub struct CosyVoiceProvider {
    api_endpoint: String,
}

#[async_trait]
impl TTSProvider for CosyVoiceProvider {
    fn provider_id(&self) -> &str {
        "cosyvoice"
    }

    async fn list_voices(&self) -> CoreResult<Vec<VoiceInfo>> {
        // 返回该模型支持的预设或克隆角色声线
        Ok(vec![
            VoiceInfo {
                id: "cosyvoice:chinese_female_1".into(),
                name: "温柔御姐 (CosyVoice)".into(),
                gender: "female".into(),
                locale: "zh-CN".into(),
                description: "适合玄幻/言情小说女主播音".into(),
            },
            VoiceInfo {
                id: "cosyvoice:chinese_male_narrator".into(),
                name: "沉稳旁白 (CosyVoice)".into(),
                gender: "male".into(),
                locale: "zh-CN".into(),
                description: "适合长篇宏大世界观故事讲述".into(),
            },
        ])
    }

    async fn synthesize(&self, payload: &SynthesisPayload) -> CoreResult<SynthesisOutput> {
        // 模拟/调用远程 CosyVoice HTTP 服务
        tracing::info!("CosyVoice 正在合成台词: 「{}」, 声线: {}", payload.text, payload.voice);

        // 如果配置了实际服务，使用 reqwest 发送网络请求
        // let client = reqwest::Client::new();
        // let res = client.post(&format!("{}/v1/audio/speech", self.api_endpoint))
        //     .json(&payload).send().await.map_err(|e| CoreError::TTS(e.to_string()))?;
        // let audio_bytes = res.bytes().await.map_err(|e| CoreError::TTS(e.to_string()))?.to_vec();

        // 构造返回对象
        Ok(SynthesisOutput {
            audio_bytes: vec![0xFF, 0xFB, 0x90, 0x44], // 示例 MP3 标头数据
            format: AudioFormat::Mp3,
            sample_rate: 24000,
            duration_ms: 2500,
        })
    }
}
```

### 第四步：在宿主 `app/src/main.rs` 中注册挂载

只需在启动器的初始化列表增加一行：

```rust
// app/src/main.rs
use leo_plugin_tts_cosyvoice::CosyVoicePlugin;

// 在 plugin_manager 中注册：
plugin_manager.register(Arc::new(CosyVoicePlugin::new("http://localhost:50000"))).await;
```

---

## 实战二：开发自定义文档导入器（EPUB 导入插件）

假设我们要支持从 `.epub` 电子书文件直接提取章节目录并转换为剧本 AST。

### 第一步：创建 Crate

```bash
cargo new --lib plugins/importers/epub
```

### 第二步：配置 `Cargo.toml`

```toml
[package]
name = "leo-plugin-importer-epub"
version = "0.1.0"
edition = "2024"
description = "EPUB 格式电子书解析与剧本 AST 转换插件"

[dependencies]
leo-core = { path = "../../../core" }
async-trait = "0.1"
epub = "2.1"     # 开源 epub 解析 crate
tracing = "0.1"
uuid = { version = "1.15", features = ["v4"] }
```

并在根目录 `Cargo.toml` 引入该 Crate。

### 第三步：编写插件实现代码

编辑 `plugins/importers/epub/src/lib.rs`：

```rust
use std::sync::Arc;
use async_trait::async_trait;
use leo_core::ast::{ChapterNode, ParagraphNode, ScriptDocument, SegmentKind, SegmentNode, SentenceNode};
use leo_core::context::Context;
use leo_core::error::{CoreError, CoreResult};
use leo_core::plugin::{Importer, Plugin, PluginCategory, PluginManifest};
use uuid::Uuid;

pub struct EpubImporterPlugin;

#[async_trait]
impl Plugin for EpubImporterPlugin {
    fn manifest(&self) -> PluginManifest {
        let mut manifest = PluginManifest::new(
            "leo-plugin-importer-epub",
            "EPUB 电子书导入器",
            "0.1.0",
            PluginCategory::Importer,
        );
        manifest.description = "解析 EPUB 电子书，提取目录章节并生成剧本 AST".into();
        manifest.capabilities = vec!["importer:epub".into()];
        manifest
    }

    async fn apply(&self, ctx: &Context) -> CoreResult<()> {
        let importer: Arc<dyn Importer> = Arc::new(EpubImporter);
        ctx.provide(importer).await;
        tracing::info!("EPUB 导入器插件已挂载");
        Ok(())
    }
}

pub struct EpubImporter;

#[async_trait]
impl Importer for EpubImporter {
    fn supported_extensions(&self) -> &[&str] {
        &["epub"]
    }

    async fn import(&self, filename: &str, content: &[u8]) -> CoreResult<ScriptDocument> {
        // 读取字节流并构建 ScriptDocument AST
        let doc_id = Uuid::new_v4().to_string();
        let title = filename.trim_end_matches(".epub").to_string();

        let mut doc = ScriptDocument::new(doc_id, title);
        
        // 伪代码演示：遍历章节并填充 AST 节点
        let chapter = ChapterNode::new(0, "第一章 穿越仙途", "正文内容...");
        doc.chapters.push(chapter);

        Ok(doc)
    }
}
```

---

## 最佳实践与注意事项

1. **绝对禁止 `panic!`**：
   - 所有的 I/O、序列化、网络请求，必须使用 `map_err(|e| CoreError::...)?` 向上转换为 `CoreResult`。
2. **轻量依赖，职责单一**：
   - 保持插件高内聚。一个插件只做一件事（例如纯清洗、纯分章、纯特定 TTS 驱动）。
3. **单元测试完备**：
   - 每一个插件内部都应包含独立的 `#[cfg(test)]` 单元测试，保证在不依赖外部全局环境的情况下可通过 `cargo test -p <your-plugin-name>` 独立验证。
