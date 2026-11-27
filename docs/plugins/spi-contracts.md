# 🧩 Leo Studio 核心 SPI 扩展点详解 (SPI Contracts)

SPI（Service Provider Interface）是 Leo Studio 微内核提供给插件开发者实现具体业务能力的接口契约。所有扩展点均定义于 `leo-core::plugin::traits` 中。

---

## 1. 导入器扩展点 (`Importer`)

导入器插件负责接收用户上传的原始文件字节流（如 `.txt`, `.epub`, `.docx`, `.pdf`），解析文本结构，并转换为微内核的标准 **剧本抽象语法树 (`ScriptDocument`)**。

### Trait 定义

```rust
use async_trait::async_trait;
use leo_core::ast::ScriptDocument;
use leo_core::error::CoreResult;

#[async_trait]
pub trait Importer: Send + Sync {
    /// 该导入器所支持的文件扩展名列表（全小写，不带点号）
    fn supported_extensions(&self) -> &[&str];

    /// 解析文件字节流并构造 ScriptDocument AST
    async fn import(&self, filename: &str, content: &[u8]) -> CoreResult<ScriptDocument>;
}
```

### 核心数据模型：剧本语法树 (`ScriptDocument` AST)

微内核采用层级分明的 AST 结构表达一部有声剧：

```text
ScriptDocument (剧本文档)
  └── ChapterNode (章节)
        └── ParagraphNode (自然段落)
              └── SentenceNode (单句)
                    └── SegmentNode (对白/旁白/音效切片)
```

- **`ScriptDocument`**：顶层剧目，包含 `id`, `title`, `author`, `chapters` 列表。
- **`ChapterNode`**：单章节，包含 `id`, `chapter_index`, `title`, `paragraphs` 列表。
- **`ParagraphNode`**：段落，包含 `id`, `paragraph_index`, `sentences` 列表。
- **`SentenceNode`**：分句，包含 `id`, `sentence_index`, `raw_text`, `segments` 列表。
- **`SegmentNode`**：最小执行单位，包含 `id`, `text`, `kind` (`SegmentKind::Dialogue`, `SegmentKind::Narration`, `SegmentKind::Sfx`), `speaker_id`, `audio_ref`。

---

## 2. 导出器扩展点 (`Exporter`)

导出器插件负责将微内核的 `ScriptDocument` AST 序列化转换为用户指定的格式，如影视制作配音台账（CSV）、前后端通用 JSON、或剪辑工程文件（如 Final Cut Pro XML）。

### Trait 定义

```rust
use async_trait::async_trait;
use leo_core::ast::ScriptDocument;
use leo_core::error::CoreResult;

#[async_trait]
pub trait Exporter: Send + Sync {
    /// 导出格式名称标识，如 "csv", "json", "fcpxml"
    fn format_name(&self) -> &str;

    /// 导出的 HTTP MIME-Type，如 "text/csv; charset=utf-8"
    fn mime_type(&self) -> &str;

    /// 将 AST 剧本文档转换为目标格式字节流
    async fn export(&self, script: &ScriptDocument) -> CoreResult<Vec<u8>>;
}
```

---

## 3. 语音合成扩展点 (`TTSProvider`)

TTS 插件连接外部语音合成服务（本地模型如 CosyVoice / GPT-SoVITS / ChatTTS，或云端 API 如微软 Edge-TTS / 阿里百炼 / Azure Speech），负责将纯文本台词合成为真实音频。

### Trait 定义

```rust
use async_trait::async_trait;
use leo_core::error::CoreResult;
use leo_core::plugin::{SynthesisOutput, SynthesisPayload, VoiceInfo};

#[async_trait]
pub trait TTSProvider: Send + Sync {
    /// 供应商全局唯一标识（如 "edge", "cosyvoice", "azure"）
    fn provider_id(&self) -> &str;

    /// 查询该供应商所提供的全部声线角色列表
    async fn list_voices(&self) -> CoreResult<Vec<VoiceInfo>>;

    /// 执行单条台词/旁白的语音合成
    async fn synthesize(&self, payload: &SynthesisPayload) -> CoreResult<SynthesisOutput>;
}
```

### 输入载荷：`SynthesisPayload`

```rust
pub struct SynthesisPayload {
    /// 需要合成的纯文本内容
    pub text: String,
    /// 目标声线 ID（如 "zh-CN-YunxiNeural" 或 "cosyvoice:char_001"）
    pub voice: String,
    /// 语速倍率（0.5 ~ 2.0，基准为 1.0）
    pub speed: f32,
    /// 音调调整（-100% ~ +100%，基准为 0.0）
    pub pitch: f32,
    /// 可选情感标记（如 "angry", "happy", "whispering"）
    pub emotion: Option<String>,
}
```

### 输出结果：`SynthesisOutput`

```rust
pub struct SynthesisOutput {
    /// 原始音频字节流数据 (如 MP3 或 WAV 二进制)
    pub audio_bytes: Vec<u8>,
    /// 音频编码格式 (AudioFormat::Mp3, AudioFormat::Wav, AudioFormat::Aac)
    pub format: AudioFormat,
    /// 采样率 (如 24000, 44100, 48000)
    pub sample_rate: u32,
    /// 音频时长 (毫秒)
    pub duration_ms: u64,
}
```

---

## 4. 持久化存储扩展点 (`StorageProvider`)

存储插件负责将剧本 AST 树以及生成的配音音频片段安全落盘（例如存储在本地 CSV、SQLite 数据库、本地文件系统、或 S3 对象存储中）。

### Trait 定义

```rust
use async_trait::async_trait;
use leo_core::ast::{AudioClipRef, AudioFormat, ScriptDocument};
use leo_core::error::CoreResult;

#[async_trait]
pub trait StorageProvider: Send + Sync {
    /// 保存剧本 AST 文档
    async fn save_script(&self, script: &ScriptDocument) -> CoreResult<()>;

    /// 按项目 ID 读取剧本 AST 文档
    async fn load_script(&self, project_id: &str) -> CoreResult<Option<ScriptDocument>>;

    /// 保存单条音频切片二进制，并返回可引用的 AudioClipRef 凭据
    async fn save_audio_clip(&self, clip_id: &str, data: &[u8], format: AudioFormat) -> CoreResult<AudioClipRef>;

    /// 按存储 key 读取音频切片原始字节流
    async fn load_audio_clip(&self, storage_key: &str) -> CoreResult<Option<Vec<u8>>>;
}
```

---

## 5. 文本流水线处理扩展点 (`Text Processing SPIs`)

文本流水线包含三个细粒度的拆解与清洗接口：

### 5.1 文本清洗 (`TextCleaner`)
去除文本中的无意义干扰字符（如颜文字、非法控制符、多余空格等）：
```rust
pub trait TextCleaner: Send + Sync {
    fn clean_line(&self, text: &str) -> String;
}
```

### 5.2 小说分章拆分 (`ChapterSplitter`)
基于正则或大纲规则识别章节标题并切分正文：
```rust
pub struct RawChapter {
    pub title: String,
    pub content: String,
}

pub trait ChapterSplitter: Send + Sync {
    fn split_chapters(&self, full_text: &str) -> Vec<RawChapter>;
    fn resolve_fingerprint(&self, title: &str) -> String;
}
```

### 5.3 句子内台词与旁白切分 (`DialogueSplitter`)
识别单句中的对话引号（“…”、「…」）与旁白部分：
```rust
pub struct RawDialogueSegment {
    pub text: String,
    pub kind: SegmentKind, // Dialogue (台词) 或 Narration (旁白)
}

pub trait DialogueSplitter: Send + Sync {
    fn split_sentence(&self, sentence: &str) -> Vec<RawDialogueSegment>;
}
```

---

## 6. 大语言模型扩展点 (`LlmProvider`)

内部统一使用与协议无关的 canonical 请求 / 响应；OpenAI Chat、OpenAI Responses、Anthropic Messages 的互相转换放在 `leo-llm-protocol` 共享库里，不进入 SPI。

### Trait 定义

```rust
#[async_trait]
pub trait LlmProvider: Send + Sync {
    fn provider_id(&self) -> &str;
    fn protocol(&self) -> LlmProtocol;
    async fn list_models(&self) -> CoreResult<Vec<ModelInfo>>;
    async fn chat(&self, request: &ChatRequest) -> CoreResult<ChatResponse>;

    /// 默认实现回退到 `chat`，只产出一个增量。
    async fn chat_stream(&self, request: &ChatRequest) -> CoreResult<ChatStream>;
}
```

### 协议枚举

```rust
pub enum LlmProtocol {
    OpenAiChat,       // POST {base_url}/chat/completions
    OpenAiResponses,  // POST {base_url}/responses
    Anthropic,        // POST {base_url}/v1/messages
}
```

### 请求载荷

```rust
pub struct ChatRequest {
    pub model: Option<String>,
    pub messages: Vec<ChatMessage>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub max_tokens: Option<u32>,
    pub stop: Vec<String>,
    pub response_format: ResponseFormat,
    /// None = 用插件配置的默认流式开关
    pub stream: Option<bool>,
    pub thinking: ThinkingConfig,
}

pub struct ThinkingConfig {
    pub mode: ThinkingMode,                 // Default / Enabled / Disabled / Adaptive
    pub effort: Option<ReasoningEffort>,    // None / Minimal / Low / Medium / High / XHigh / Max
    pub budget_tokens: Option<u32>,         // 主要给 Anthropic / 百炼
}

pub struct ChatMessage {
    pub role: ChatRole,        // System / User / Assistant
    pub content: String,
    pub name: Option<String>,
}

pub enum ResponseFormat {
    Text,
    JsonObject,
    JsonSchema { schema: serde_json::Value },
}
```

覆盖文本、单轮/多轮、流式与非流式对话；不支持 tool / function calling、图片/音频内容块。`stream = None` 时用驱动默认值；`ThinkingConfig::default()` 不覆盖驱动默认。`effort = None` 会在供应商侧归一化成关闭思考。

### 响应结果

```rust
pub struct ChatResponse {
    pub id: Option<String>,
    pub model: String,
    pub content: String,
    pub reasoning_content: Option<String>,
    pub finish_reason: FinishReason,
    pub usage: Option<TokenUsage>,
}

pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
}
```

`finish_reason` 归一化为 `stop / length / content_filter / tool_calls / other`。

### 流式增量

```rust
pub struct ChatDelta {
    pub content: Option<String>,
    pub reasoning_content: Option<String>,
    pub finish_reason: Option<FinishReason>,
    pub usage: Option<TokenUsage>,
}

pub type ChatStream = Pin<Box<dyn Stream<Item = CoreResult<ChatDelta>> + Send>>;
```

`ChatResponseAccumulator` 可以把 `ChatDelta` 序列拼回 `ChatResponse`：文本和思考内容直接拼接，`finish_reason` 取最后一个非空值，`usage` 按字段取最大值合并（兼容 Anthropic `message_start` + `message_delta` 两段式用量）。

### 实现约定

- 供应商子插件只负责地址、鉴权、模型和供应商专有参数；公共协议字段用 `leo-llm-protocol` 编解码，SSE 用它的 `SseParser` / `decode_stream_event`。
- 思考开关 / 深度的公共档位在 SPI；各供应商能接受什么（`thinking.type`、`reasoning_effort`、`thinking_budget`、`enable_thinking`）由子插件自己映射，注册层和协议库不猜供应商字段。
- 注册层 `LlmHub` 负责默认驱动、`驱动id:模型` 路由、`chat_stream` 流式入口，以及 `chat_raw(protocol, body)` 一次性协议桥。
- 协议细节与转换规则见 [llm-protocols.md](./llm-protocols.md)。

