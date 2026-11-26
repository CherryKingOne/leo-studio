# LLM 协议层：OpenAI Chat / Responses / Anthropic

LLM 插件分成三层：**core 契约** → **共享协议库** → **供应商薄客户端**。这样新增一家供应商时，不需要重写三家协议的请求体；换协议时，也不需要换供应商代码。

```text
core/src/spi/llm.rs                  canonical 请求 / 响应 + LlmProvider
        ▲
        │
plugins/llm/protocol                 leo-llm-protocol（纯库，不是插件）
   ├── openai/chat_completions.rs    OpenAI 旧接口
   ├── openai/responses.rs           OpenAI 新接口
   ├── anthropic/messages.rs         Anthropic Messages
   ├── endpoint.rs                   路径 + 鉴权
   ├── convert.rs                    跨协议 request / response 转换
   └── error.rs                      三家错误体 → CoreError
        ▲
        │
plugins/llm/{deepseek,volcengine,bailian}   供应商插件：地址、Key、模型、特殊参数
        ▲
        │
plugins/llm/registry                 leo-plugin-llm：默认驱动 + 模型路由 + 协议桥
```

`leo-llm-protocol` 没有 `Plugin` 实现、不注册驱动、不进 `CATALOG`；它只是被注册层和供应商共同依赖的库。

## 1. canonical 模型

内部统一用 Chat Completions 的语义：`ChatRequest { messages, temperature, top_p, max_tokens, stop, response_format }`，响应统一成 `ChatResponse { content, reasoning_content, finish_reason, usage }`。

任何协议进来，先 `decode_request` 成 canonical；调用驱动；再把 canonical `encode_response` 回目标协议。跨协议转换因此只有一套逻辑。

当前 canonical 只覆盖**文本、非流式、单轮/多轮对话**：消息角色只有 `system / user / assistant`，不支持 tool / function calling、图片/音频内容块和流式事件。`reasoning_content` 会尽量从 DeepSeek 的 `reasoning_content`、Anthropic 的 `thinking` 块、Responses 的 reasoning 项目里提取，但不保证所有供应商都返回。

## 2. 三套协议对照

| | OpenAI Chat Completions | OpenAI Responses | Anthropic Messages |
|---|---|---|---|
| 路径 | `POST {base_url}/chat/completions` | `POST {base_url}/responses` | `POST {base_url}/v1/messages` |
| 鉴权 | `Authorization: Bearer` | `Authorization: Bearer` | `x-api-key` + `anthropic-version: 2023-06-01` |
| system 提示 | messages 里的 `system` 消息 | 顶层 `instructions` | 顶层 `system` |
| 输入 | `messages[]` | `input`（字符串或 message 项目数组） | `messages[]` |
| 输出上限 | `max_tokens` | `max_output_tokens` | `max_tokens`（必填） |
| JSON 输出 | `response_format` | `text.format` | 无原生字段，降级成 system 指令 |
| 停止词 | `stop`（字符串或数组） | 无原生参数，忽略 | `stop_sequences` |
| 响应文本 | `choices[0].message.content` | `output[]` / `output_text` | `content[]` 的 text / thinking 块 |
| 用量 | `prompt_tokens` / `completion_tokens` | `input_tokens` / `output_tokens` | `input_tokens` / `output_tokens` |

`base_url` 约定是**协议根地址**，不是完整 endpoint。OpenAI 兼容地址通常已经带 `/v1` 或 `/api/v3`，所以路径里不再重复版本段；Anthropic 的 `/v1` 属于 Messages 接口本身，保留在路径里。

例：

```text
https://api.deepseek.com/v1            + /chat/completions
https://api.deepseek.com/anthropic     + /v1/messages
https://ark.cn-beijing.volces.com/api/v3 + /responses
```

## 3. 跨协议转换

### 请求

- `system` 消息：Chat 保留在 messages；Responses 合并到 `instructions`；Anthropic 提到顶层 `system`。
- 连续同角色消息：Anthropic 要求 user / assistant 交替，编码时用空行合并成一条。
- `max_tokens`：Responses 改名为 `max_output_tokens`；Anthropic 没给时补 `4096`（接口要求必填）。
- `response_format`：
  - `json_object`：Chat / Responses 原生支持；Anthropic 降级为 system 指令。
  - `json_schema`：Chat 原样传 `json_schema`；Responses 包成 `text.format`；Anthropic 把 schema 写进 system 指令。
- `stop`：Chat / Anthropic 原生支持；Responses 没有对应参数，**有损忽略**。

### 响应

- 文本统一抽取成 `content`；`reasoning_content` 单独保留。
- `finish_reason` 归一化成 `stop / length / content_filter / tool_calls / other`。
- 用量统一成 `input_tokens / output_tokens / total_tokens`；缺失时 `usage = None`。

有损点集中在上表的“Anthropic JSON”“Responses stop”两处。需要严格 JSON Schema 时，优先选 Chat 或 Responses；需要停止词时，不要选 Responses。

## 4. 注册层的协议桥

`LlmHub::chat` 接收 canonical 请求；`LlmHub::chat_raw(protocol, body)` 接收任一协议的原始 JSON，解码 → canonical → 调驱动 → 再按**入参协议**编码返回。

```rust
// 调用方发 OpenAI Chat 格式，底层驱动可以是 Anthropic 配置
let out = hub.chat_raw(LlmProtocol::OpenAiChat, body).await?;

// 调用方发 Anthropic 格式，底层驱动可以是 OpenAI Chat 配置
let out = hub.chat_raw(LlmProtocol::Anthropic, body).await?;
```

驱动实际走什么协议由驱动自己的 `protocol` 配置决定，和调用方协议无关。因此三家供应商可以任意混用三套协议：例如 DeepSeek 配 `anthropic`、火山配 `openai-responses`、百炼配 `openai-chat`，上层只认一种格式。

模型路由：

- `request.model = "deepseek:deepseek-chat"` → 走 `deepseek` 驱动，模型 `deepseek-chat`。
- `request.model = "qwen-plus"`（没有已注册前缀）→ 走默认驱动，模型 `qwen-plus`。
- `request.model = None` → 走默认驱动，用它配置里的 `model`。

## 5. 供应商配置

```json
{
  "llm": { "default": "deepseek" },
  "settings": {
    "leo-plugin-llm-deepseek": {
      "protocol": "openai-chat",
      "base_url": "https://api.deepseek.com/v1",
      "model": "deepseek-chat",
      "temperature": 0.7,
      "max_tokens": 4096
    },
    "leo-plugin-llm-volcengine": {
      "protocol": "openai-chat",
      "base_url": "https://ark.cn-beijing.volces.com/api/v3",
      "model": "ep-2026xxxxxxxx",
      "thinking": "auto"
    },
    "leo-plugin-llm-bailian": {
      "protocol": "openai-chat",
      "base_url": "https://dashscope.aliyuncs.com/compatible-mode/v1",
      "model": "qwen-plus",
      "enable_thinking": false,
      "enable_search": false
    }
  }
}
```

| 驱动 | 环境变量 | 默认 `base_url` | 默认模型 | 专有参数 |
|---|---|---|---|---|
| `deepseek` | `DEEPSEEK_API_KEY` | `https://api.deepseek.com/v1` | `deepseek-chat` | `extra_headers` / `extra_body` |
| `volcengine` | `ARK_API_KEY` | `https://ark.cn-beijing.volces.com/api/v3` | 空，需填 `ep-...` 或模型 ID | `thinking: enabled / disabled / auto` |
| `bailian` | `DASHSCOPE_API_KEY` | `https://dashscope.aliyuncs.com/compatible-mode/v1` | `qwen-plus` | `enable_thinking` / `enable_search` |

三个驱动都能编解码 `openai-chat` / `openai-responses` / `anthropic`。能否真的用某个协议，取决于供应商是否提供对应端点：例如 DeepSeek 同时提供 OpenAI Chat 与 Anthropic 兼容地址；如果某家只提供 OpenAI Chat，就让驱动保持 `openai-chat`，调用方仍可用 Anthropic 格式进来，由注册层的 `chat_raw` 做协议转换。切换上游协议时改 `protocol` 和对应的 `base_url`，请求/响应体由协议库转换。

`extra_body` 会合并进最终请求体，用于供应商的新参数；`extra_headers` 追加到请求头。DeepSeek 没有专有字段；火山方舟的 `thinking`、百炼的 `enable_thinking` / `enable_search` 在 `extra_body` 之后写入，同名时以这些专有字段为准。`temperature` 限 0.0~2.0，`top_p` 限 0.0~1.0，`max_tokens` 必须大于 0。

API Key 只从环境变量读，不写进 `leo-plugins.json`。没有 Key 时插件照常装配，但不注册驱动，日志里会有警告。

## 6. 新增一家供应商

1. 新建 `plugins/llm/<name>/`（crate `leo-plugin-llm-<name>`），依赖 `leo-core` + `leo-llm-protocol`。
2. `config.rs` 定义 `base_url` / `protocol` / `model` / 采样参数，按需加 `extra_headers` / `extra_body` 和供应商专有字段。
3. `client.rs` 用 `leo_llm_protocol::{endpoint, auth_headers, encode_request, decode_response, decode_error}`，不要自己拼协议体。
4. `lib.rs` 在 `apply` 里 `ctx.register::<dyn LlmProvider>("<name>", ...)`；没有 Key 就只警告、不注册。
5. 根 `Cargo.toml` 加 member，`plugins/catalog` 加依赖和 `CATALOG` 项。
6. 测试：三套协议各一个假服务器用例，再加一个 `#[ignore]` 真实调用用例。

不需要改 core、协议库、注册层、API/MCP/CLI。
