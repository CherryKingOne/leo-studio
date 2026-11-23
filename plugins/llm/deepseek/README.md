# leo-plugin-llm-deepseek：DeepSeek 大模型子插件

调用 DeepSeek 的 OpenAI 兼容接口，**同时支持 Chat Completions 与 Responses 两种接口风格**，注册为 LLM 驱动 `deepseek`（插件 id `leo-plugin-llm-deepseek`）。
只依赖 `leo-core`，通过 LLM 注册层 `leo-plugin-llm` 的 `LlmHub` 对外使用。

- 鉴权：`Authorization: Bearer <API Key>`，两种接口都是非流式（`stream: false`）
- 配置 `api` 选默认接口（默认 `chat_completions`，保持原有行为），单次请求可用 `ChatRequest::api` / `.with_api(LlmApi::Responses)` 覆盖
- 两种接口的结果统一映射为 `ChatResponse`（content / model / usage / finish_reason），重试与错误映射完全相同

| | Chat Completions | Responses |
| --- | --- | --- |
| 地址 | `POST {base_url}/chat/completions` | `POST {base_url}/responses` |
| 消息 | `messages` | `input`（`{"type":"message","role","content"}` 列表，system 角色官方支持） |
| 最大输出 | `max_tokens` | `max_output_tokens` |
| `ResponseFormat::Text` | `response_format: {"type":"text"}` | `text.format: {"type":"text"}` |
| `ResponseFormat::JsonObject` | `response_format: {"type":"json_object"}` | `text.format: {"type":"json_object"}` |
| `ResponseFormat::JsonSchema` | **不支持**（官方 `response_format` 只有 text / json_object），直接报错 | `text.format: {"type":"json_schema","name","schema","strict"}` |
| 思考模式 | `thinking: {"type": "enabled/disabled"}` + `reasoning_effort` | `reasoning: {"effort": "none/low/high/max"}`（`none` = 关闭） |
| 结果 | `choices[0].message.content` | `output[]` 里 `message` 项的 `output_text` 拼接（`reasoning` 项忽略） |
| 用量 | `prompt_tokens / completion_tokens / total_tokens` | `input_tokens / output_tokens / total_tokens` → 映射为左边三个字段 |
| 结束原因 | `finish_reason` | `status`：completed → `stop`；incomplete + max_output_tokens → `length`；incomplete + content_filter → `content_filter`；failed → 报错 |

> Chat Completions 接口收到 `JsonSchema` 时驱动直接报错，不会偷偷改格式。经 `LlmHub` 调用时，注册层可按 `llm.schema_fallback`
> 降级为 `json_object` + 本地 Schema 校验（默认），或报错；也可以把这次请求切到 `api = responses` 用服务端原生结构化输出。

- 429 与 5xx（以及网络错误）按指数退避重试（基准 500 ms，翻倍，单次最长 8 秒；服务端给出 `Retry-After` 时按它等待）
- 400 / 401 / 402 / 422 不重试，返回带中文说明的错误（402 = 余额不足）
- 没有 API Key 时驱动照常注册，**调用时**返回“缺少 API Key”的错误，不会切换到别的驱动

官方文档：

- Chat Completions：<https://api-docs.deepseek.com/api/create-chat-completion>
- Responses：<https://api-docs.deepseek.com/api/create-response>，兼容性说明：<https://api-docs.deepseek.com/guides/responses_api>
- 模型与价格：<https://api-docs.deepseek.com/quick_start/pricing>
- JSON 输出：<https://api-docs.deepseek.com/guides/json_mode>
- 思考模式：<https://api-docs.deepseek.com/guides/thinking_mode>
- 错误码：<https://api-docs.deepseek.com/quick_start/error_codes>

> **关于模型名**：按官方更新日志，旧模型名 `deepseek-chat` / `deepseek-reasoner` 已于 2026-07-24 停用，
> 当前模型名为 `deepseek-flash` 与 `deepseek-v4-pro`，所以本插件默认 `deepseek-flash`。
> 新模型默认开启思考模式，本插件默认发送 `"thinking": {"type": "disabled"}`，行为与原来的 `deepseek-chat`（非思考模式）一致，
> 且 `temperature` 只在非思考模式下生效。

## 配置字段

位置：`leo-plugins.json` → `"settings"` → `"leo-plugin-llm-deepseek"`。全部可选；写错字段名或取值越界时启动失败（`deny_unknown_fields` + 校验）。

| 字段 | 类型 | 默认值 | 必填 | 说明 |
| --- | --- | --- | --- | --- |
| `api_key_env` | string | `"DEEPSEEK_API_KEY"` | 否 | 读取 API Key 的环境变量名（Key 本身不写进配置文件） |
| `base_url` | string | `"https://api.deepseek.com"` | 否 | 接口地址，必须以 `http://` / `https://` 开头 |
| `api` | string | `"chat_completions"` | 否 | 默认接口风格：`chat_completions` / `responses`；请求里的 `api` 优先 |
| `model` | string | `"deepseek-flash"` | 否 | 默认模型：`deepseek-flash` / `deepseek-v4-pro`；请求里的 `model` 优先 |
| `temperature` | number \| null | `null`（不传，官方默认 1） | 否 | 采样温度 [0, 2]；请求里的值优先；思考模式下无效 |
| `max_tokens` | integer \| null | `null`（不传） | 否 | 最大输出 token [1, 393216]；请求里的值优先 |
| `thinking` | bool \| null | `false` | 否 | `false` 关闭思考模式（Responses 发 `reasoning.effort = "none"`），`true` 开启，`null` 不传（官方默认开启） |
| `reasoning_effort` | string \| null | `null`（不传，官方默认 high） | 否 | 思考强度 `low` / `high` / `max`；只在思考模式下有意义，`thinking=false` 时填写会启动失败 |
| `timeout_secs` | integer | `120` | 否 | 单次请求超时（秒），必须大于 0 |
| `max_retries` | integer | `2` | 否 | 429 / 5xx / 网络错误的最大重试次数，0 ~ 10 |

## 环境变量

| 变量 | 必填 | 说明 |
| --- | --- | --- |
| `DEEPSEEK_API_KEY` | 调用时必填 | DeepSeek API Key，获取：<https://platform.deepseek.com/api_keys>。变量名可用 `api_key_env` 修改 |

## 配置示例

```json
{
  "llm": { "default": "deepseek" },
  "settings": {
    "leo-plugin-llm-deepseek": {
      "api_key_env": "DEEPSEEK_API_KEY",
      "base_url": "https://api.deepseek.com",
      "api": "chat_completions",
      "model": "deepseek-flash",
      "temperature": 0.2,
      "max_tokens": 4096,
      "thinking": false,
      "timeout_secs": 120,
      "max_retries": 2
    }
  }
}
```

```bash
export DEEPSEEK_API_KEY=sk-xxxx
```

## 使用示例

通过注册层（推荐）：

```rust
use leo_core::spi::{ChatMessage, ChatRequest};
use leo_plugin_llm::LlmHub;

let llm = LlmHub::of(&ctx).await;
let req = ChatRequest {
    messages: vec![
        ChatMessage::system("判断引号里的台词是谁说的，只输出 JSON：{\"speaker\": \"人名\", \"confidence\": 0~1}"),
        ChatMessage::user("林动看着师姐，师姐笑道：“你怎么来了？”"),
    ],
    temperature: Some(0.1),
    max_tokens: Some(256),
    ..Default::default()
};
let v = llm.chat_json(req).await?;   // 自动设置 response_format = json_object
println!("{}", v["speaker"]);
```

结构化输出（JSON Schema）：

```rust
use leo_core::spi::{ChatMessage, ChatRequest, LlmApi};
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
struct Line { speaker: String, confidence: f64 }

let schema = json!({
    "type": "object",
    "properties": { "speaker": {"type": "string"}, "confidence": {"type": "number"} },
    "required": ["speaker", "confidence"],
    "additionalProperties": false
});
let req = ChatRequest::new(vec![
    ChatMessage::system("判断引号里的台词是谁说的，输出 JSON。"),
    ChatMessage::user("林动看着师姐，师姐笑道：“你怎么来了？”"),
])
.json_schema("line", schema, true)
.with_api(LlmApi::Responses);          // 本次走 Responses，服务端原生结构化输出
let line: Line = llm.chat_typed(req).await?;   // 返回后还会按 Schema 本地校验，不符合返回清楚的错误
```

如果不加 `.with_api(LlmApi::Responses)` 且配置是 `chat_completions`，注册层按 `llm.schema_fallback` 处理（默认降级为 json_object + 本地校验，并打 warn 日志）。

全局切到 Responses：

```json
{ "settings": { "leo-plugin-llm-deepseek": { "api": "responses", "model": "deepseek-flash", "thinking": false } } }
```

直接使用驱动：

```rust
use leo_core::spi::{ChatMessage, ChatRequest, LlmProvider};
use leo_plugin_llm_deepseek::{DeepSeekConfig, DeepSeekLlmProvider};

let provider = DeepSeekLlmProvider::new(DeepSeekConfig::default())?;
let resp = provider.chat(ChatRequest::new(vec![ChatMessage::user("你好")])).await?;
println!("{} / {:?} / {} tokens", resp.content, resp.finish_reason, resp.usage.total_tokens);
```

## 错误说明

| HTTP | 是否重试 | 含义 |
| --- | --- | --- |
| 400 | 否 | 请求格式错误 |
| 401 | 否 | API Key 错误 |
| 402 | 否 | 余额不足，需要充值 |
| 422 | 否 | 参数错误（模型名、max_tokens 等） |
| 429 | 是 | 速率 / 并发超限 |
| 5xx | 是 | 服务端错误 / 繁忙 |

Responses 接口返回 `status = "failed"` 时返回带 `error.code / message` 的错误；`incomplete` 时照常返回内容，`finish_reason` 为 `length` / `content_filter`。

JSON 模式下若模型输出无法识别为 JSON，官方会把内容替换成空格；`chat_json` 会返回“内容为空”的明确错误（`finish_reason=length` 时提示可能被截断）。

## 测试

```bash
# 本地假服务器测试（不联网、不需要 Key）。如果设置了 HTTP(S)_PROXY，需要同时设置 NO_PROXY=127.0.0.1,localhost
cargo test -p leo-plugin-llm-deepseek

# 真实接口测试（会产生少量费用；live_chat_json 走 Chat Completions，live_responses_json_schema 走 Responses）
DEEPSEEK_API_KEY=sk-xxxx cargo test -p leo-plugin-llm-deepseek -- --ignored
```
