# 📜 Leo Studio 插件协议与契约规范 (Plugin Protocol Specification)

本规范详细定义了 Leo Studio 插件系统的标准协议、契约接口、元数据格式、生命周期以及依赖注入规范。所有官方或第三方插件均需遵循本规范。

---

## 1. 插件基础契约 (`Plugin` Trait)

所有插件的入口都必须实现微内核定义的 `Plugin` 特征：

```rust
use async_trait::async_trait;
use leo_core::context::Context;
use leo_core::error::CoreResult;
use leo_core::plugin::manifest::PluginManifest;

#[async_trait]
pub trait Plugin: Send + Sync {
    /// 返回插件的元数据清单
    fn manifest(&self) -> PluginManifest;

    /// 挂载插件：向 Context 注入服务、监听事件或初始化环境
    async fn apply(&self, ctx: &Context) -> CoreResult<()>;
}
```

### 契约要求：
1. **线程安全**：实现必须满足 `Send + Sync + 'static`，支持多线程并发安全调用。
2. **异步初始化**：`apply` 方法为 `async` 异步调用，允许插件在初始化阶段执行异步 I/O 操作（例如建立远程连接池、拉取离线字典等）。
3. **错误冒泡**：若初始化失败，返回 `CoreResult::Err`，宿主启动器将捕获该错误并安全退出或发出告警，严禁在 `apply` 中直接 `panic!`。

---

## 2. 插件元数据清单 (`PluginManifest`)

`PluginManifest` 记录了插件的身份标识、版本、所属分类以及所具备的能力声明。

### 结构体定义

```rust
pub struct PluginManifest {
    pub id: String,                 // 插件全局唯一标识符
    pub name: String,               // 插件可读展示名称
    pub version: String,            // 遵循语义化版本号 (SemVer)
    pub description: String,        // 插件功能简述
    pub category: PluginCategory,   // 插件所属分类
    pub capabilities: Vec<String>,  // 插件暴露的具体能力声明标签
}
```

### 字段规范：

| 字段 | 类型 | 规范与命名约束 | 示例 |
| :--- | :--- | :--- | :--- |
| `id` | `String` | 推荐使用 `leo-plugin-<分类>-<名称>` 风格的小写中划线命名。全局唯一。 | `"leo-plugin-tts-edge"`, `"leo-plugin-importer-txt"` |
| `name` | `String` | 简短精炼的人类可读名称（中英文均可）。 | `"TXT 纯文本小说导入插件"`, `"微软 Edge-TTS 语音合成"` |
| `version` | `String` | 严格遵循 [SemVer 2.0.0](https://semver.org/lang/zh-CN/) 格式 (`MAJOR.MINOR.PATCH`)。 | `"0.1.0"`, `"1.2.3"` |
| `description` | `String` | 1-2 句话阐述插件的核心作用和适用场景。 | `"基于微软 Edge 神经语音的免费高品质 TTS 插件，提供多种小说声线。"` |
| `category` | `PluginCategory` | 枚举分类（见下节）。 | `PluginCategory::Tts` |
| `capabilities` | `Vec<String>` | 能力标签列表，格式为 `<domain>:<action>`。便于前端或 MCP 智能体按能力索引。 | `vec!["tts:synthesize".into(), "tts:list_voices".into()]` |

---

## 3. 插件分类规范 (`PluginCategory`)

为了杜绝插件混乱平铺，Leo Studio 在微内核中确立了强制枚举分类，每类插件归纳在其专属目录中：

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginCategory {
    Importer,    // 文档导入器 (plugins/importers/)
    Exporter,    // 剧本与台账导出器 (plugins/exporters/)
    Cleaner,     // 文本清洗过滤 (plugins/text/)
    Parser,      // 小说分章与大纲解析 (plugins/text/)
    Dialogue,    // 台词与旁白切分提取 (plugins/text/)
    Tts,         // 语音合成引擎 (plugins/tts/)
    Storage,     // 持久化与存储层 (plugins/storage/)
    Api,         // Web RESTful 传输协议 (plugins/api/)
    Mcp,         // Model Context Protocol 智能体协议 (plugins/mcp/)
    Cli,         // 命令行交互工具 (plugins/cli/)
    Transport,   // 其他网络/RPC 传输协议
    Other,       // 其他通用扩展
}
```

---

## 4. 上下文依赖注入机制 (`Context`)

微内核提供的 `Context` 是插件间解耦通信的核心媒介。它集成了 **IoC 服务容器** 与 **EventBus 事件总线**。

```rust
pub struct Context {
    pub services: Arc<RwLock<ServiceContainer>>,
    pub events: EventBus,
    pub scope: String,
}
```

### 4.1 服务注册与提供 (`ctx.provide`)

插件在 `apply` 阶段将自己实现的核心 SPI 服务注册至 `Context`：

```rust
// 注册 TTSProvider SPI 实现
let provider: Arc<dyn TTSProvider> = Arc::new(MyCustomTTSProvider::new());
ctx.provide(provider).await;

// 注册纯结构体或工具服务
ctx.provide(Arc::new(MyDictionaryService::new())).await;
```

### 4.2 服务发现与消费 (`ctx.inject`)

任何需要使用某项能力的组件（如 API 路由、MCP 工具、CLI 命令或其他插件），均通过类型安全地提取服务：

```rust
// 从 Context 提取注册的 StorageProvider
match ctx.inject::<Arc<dyn StorageProvider>>().await {
    Ok(storage) => {
        storage.save_script(&script).await?;
    }
    Err(_) => {
        tracing::warn!("系统当前未装载任何持久化存储插件，操作被跳过");
    }
}
```

### 4.3 作用域派生 (`ctx.derive`)

微内核支持分层作用域，通过 `ctx.derive("project_123")` 可以创建携带相同服务与事件总线、但拥有隔离上下文 ID 的子上下文，用于隔离单个剧目制作生命周期。

---

## 5. 事件总线协议 (`EventBus`)

插件可以通过 `ctx.events` 实现跨模块异步解耦通信，遵循 **发布/订阅 (Pub/Sub)** 模式。

### 5.1 核心事件命名标准

事件名称采用分级冒号格式：`<domain>:<entity>:<action>`。

| 标准事件名 | 触发时机 | 载荷参数规范 (`serde_json::Value`) |
| :--- | :--- | :--- |
| `project:created` | 新小说剧本完成导入解析并创建 | `{"project_id": "...", "title": "...", "chapter_count": 10}` |
| `chapter:synthesized`| 单章节的音频合成全部就绪 | `{"project_id": "...", "chapter_id": "...", "clips": 25}` |
| `audio:clip_saved` | 单条台词音频切片已落盘 | `{"clip_id": "...", "duration_ms": 1200, "storage_key": "..."}` |

### 5.2 订阅事件示例

```rust
// 插件在 apply 中监听音频生成就绪事件
ctx.events.on("chapter:synthesized", Arc::new(|payload| {
    Box::pin(async move {
        let chapter_id = payload["chapter_id"].as_str().unwrap_or_default();
        tracing::info!("监听到章节音频合成完毕: {}", chapter_id);
        Ok(())
    })
})).await;
```

---

## 6. 统一错误处理规范 (`CoreResult`)

所有插件与 SPI 方法必须统一返回 `leo_core::error::CoreResult<T>`：

```rust
pub type CoreResult<T> = Result<T, CoreError>;

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("I/O 错误: {0}")]
    Io(#[from] std::io::Error),

    #[error("插件未找到或未注入: {0}")]
    PluginNotFound(String),

    #[error("存储异常: {0}")]
    Storage(String),

    #[error("TTS 合成失败: {0}")]
    TTS(String),

    #[error("格式解析错误: {0}")]
    Parser(String),

    #[error("内核异常: {0}")]
    Internal(String),
}
```

### 错误处理最佳实践：
- 自定义插件发生网络异常或远端鉴权失败时，应封装为 `CoreError::TTS(format!("API 请求失败: {e}"))`。
- 不要在插件库内部调用 `std::process::exit` 或导致崩溃的解包操作（如 `.unwrap()`、`.expect()`），始终使用 `?` 向上冒泡。
