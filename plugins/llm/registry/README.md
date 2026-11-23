# leo-plugin-llm：大模型对话注册层

`leo-plugin-llm` 是 LLM（大模型对话）扩展点的**注册层插件**（“大箱子”），与 `leo-plugin-tts` 同一套模式：

- 子插件（如 `leo-plugin-llm-deepseek`）只依赖 `leo-core`，在 `apply` 里用
  `ctx.register::<dyn LlmProvider>("deepseek", provider)` 按驱动 id 注册，不需要知道本插件存在。
- 本插件读取 `leo-plugins.json` 的 `"llm"` 一节设置默认驱动，并通过 `ctx.provide` 对外提供统一服务 `LlmHub`。
- 注册顺序任意；子插件卸载后 `LlmHub` 下一次调用即可感知。
- **配置了默认驱动但它没有注册**（插件被停用、id 写错）时，调用直接返回 `ServiceNotFound` 错误，并列出已注册的驱动，**不会悄悄换成别的驱动**。
- 没有配置默认驱动时，使用最早注册的驱动；一个驱动都没有时返回错误。
- **接口风格与能力**：驱动通过 `capabilities()` 声明支持的接口（`chat_completions` / `responses`）以及每种接口是否支持 `json_object` / `json_schema`。
  `LlmHub::capabilities(driver)`、`drivers_with_capabilities()` 对外暴露；请求可用 `ChatRequest::api` 指定本次接口，驱动不支持该接口时直接报错。
- **JsonSchema 降级**：请求要 `ResponseFormat::JsonSchema`，但所选驱动 / 接口不支持时，按 `schema_fallback` 处理（见下表），**不会静默降级**。

扩展点契约在 `leo-core::spi::llm`：

```rust
#[async_trait]
pub trait LlmProvider: Send + Sync {
    fn provider_id(&self) -> &str;
    /// 默认：只有 Chat Completions + JsonObject（老驱动不用改）
    fn capabilities(&self) -> LlmCapabilities { LlmCapabilities::default() }
    async fn chat(&self, req: ChatRequest) -> CoreResult<ChatResponse>;
    /// 以 JSON 模式对话并把结果解析为 serde_json::Value（内容为空或非法 JSON 时报错；JsonSchema 时再本地校验）
    async fn chat_json(&self, req: ChatRequest) -> CoreResult<serde_json::Value>;
}
```

## 配置字段

位置：`leo-plugins.json` → `"llm"`。写错字段名会启动失败（`deny_unknown_fields`）。

| 字段 | 类型 | 默认值 | 必填 | 说明 |
| --- | --- | --- | --- | --- |
| `default` | string \| null | `null` | 否 | 默认驱动 id，如 `deepseek`。不填用最早注册的驱动；填了但未注册时调用报错 |
| `schema_fallback` | string | `"json_object"` | 否 | 请求 JsonSchema 但驱动 / 接口不支持时：`json_object` = 改用 JsonObject，把 Schema 写进一条系统提示词，返回后本地按 Schema 校验（打 warn 日志）；`error` = 直接报错、不发请求 |

本插件是注册层，**不能**写进 `disabled`（会启动失败）。

## 环境变量

本插件不读取环境变量。各驱动的 API Key 由对应子插件自己从环境变量读取（如 `DEEPSEEK_API_KEY`）。

## 配置示例

```json
{
  "llm": { "default": "deepseek", "schema_fallback": "json_object" },
  "settings": {
    "leo-plugin-llm-deepseek": { "api": "chat_completions", "model": "deepseek-flash", "temperature": 0.2 }
  }
}
```

## 使用示例

```rust
use leo_core::spi::{ChatMessage, ChatRequest, LlmApi};
use leo_plugin_llm::LlmHub;

let llm = LlmHub::of(&ctx).await;

// 默认驱动，普通文本
let resp = llm
    .chat(ChatRequest::new(vec![
        ChatMessage::system("你是有声书编辑。"),
        ChatMessage::user("用一句话概括这一章。"),
    ]))
    .await?;
println!("{}（{} tokens）", resp.content, resp.usage.total_tokens);

// JSON 模式（提示词里要写明输出 JSON）
let value = llm
    .chat_json(ChatRequest::new(vec![
        ChatMessage::system("判断台词的说话人，只输出 JSON：{\"speaker\": \"人名\"}"),
        ChatMessage::user("师姐笑道：“你怎么来了？”"),
    ]))
    .await?;

// 指定驱动
let resp = llm.chat_with("deepseek", ChatRequest::new(vec![ChatMessage::user("你好")])).await?;

// 结构化输出：JsonSchema + 反序列化为业务类型；指定本次走 Responses 接口
#[derive(serde::Deserialize)]
struct Line { speaker: String }
let schema = serde_json::json!({
    "type": "object", "properties": {"speaker": {"type": "string"}}, "required": ["speaker"]
});
let line: Line = llm
    .chat_typed(
        ChatRequest::new(vec![ChatMessage::user("师姐笑道：“你怎么来了？”谁说的？输出 JSON")])
            .json_schema("line", schema, true)
            .with_api(LlmApi::Responses),
    )
    .await?;

// 能力查询
let caps = llm.capabilities(Some("deepseek"))?;
println!("默认接口 {}，支持 {:?}", caps.default_api, caps.api_list());

// 诊断：列出已注册驱动
for d in llm.drivers() {
    println!("{} default={}", d.id, d.is_default);
}
```

## 新增一个 LLM 驱动

1. 新建 `plugins/llm/<名字>/`，只依赖 `leo-core`，实现 `LlmProvider`，在 `apply` 里 `ctx.register::<dyn LlmProvider>("<驱动id>", ..)`。
   支持 Responses 接口或服务端 JsonSchema 时，重写 `capabilities()` 如实声明；收到不支持的格式要报错，不要自行降级（降级由注册层统一做）。
2. 根 `Cargo.toml` 的 `members` 加一行。
3. `plugins/catalog/Cargo.toml` 加依赖，`plugins/catalog/src/lib.rs` 的 `CATALOG` 加一项。
4. 需要时把 `leo-plugins.json` 的 `"llm": {"default": "<驱动id>"}` 改过去。

## 测试

```bash
cargo test -p leo-plugin-llm
```
