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
   ├── thinking.rs                   stream / thinking 参数映射
   ├── stream.rs                     SSE 增量解析 + 三协议增量解码
   ├── endpoint.rs                   路径 + 鉴权
   ├── convert.rs                    跨协议 request / response 转换
   └── error.rs                      三家错误体 → CoreError
        ▲
        │
plugins/llm/{deepseek,volcengine,bailian,agnes,spark,qianfan,longcat,gemini,minimax,zai}   供应商插件：地址、Key、模型、特殊参数
        ▲
        │
plugins/llm/registry                 leo-plugin-llm：默认驱动 + 模型路由 + 协议桥
```

`leo-llm-protocol` 没有 `Plugin` 实现、不注册驱动、不进 `CATALOG`；它只是被注册层和供应商共同依赖的库。

## 1. canonical 模型

内部统一用 Chat Completions 的语义：

```rust
ChatRequest {
    messages, temperature, top_p, max_tokens, stop, response_format,
    stream: Option<bool>,       // None = 用驱动自己的默认流式开关
    thinking: ThinkingConfig,   // mode / effort / budget_tokens
}
```

响应统一成 `ChatResponse { content, reasoning_content, finish_reason, usage }`。流式则产出 `ChatDelta { content, reasoning_content, finish_reason, usage }` 序列，`ChatResponseAccumulator` 可以把增量拼回完整 `ChatResponse`。

`ThinkingConfig` 的三个维度：

- `mode`：`default`（不表态，交给供应商默认）、`enabled`、`disabled`、`adaptive`（让模型自己决定）。
- `effort`：`none` / `minimal` / `low` / `medium` / `high` / `xhigh` / `max`；`none` 等价于关闭思考。
- `budget_tokens`：思考预算，主要给 Anthropic 和百炼使用。

`ChatRequest` 里显式设置的 `stream` / `thinking` 优先；缺省时与驱动 `settings` 里的默认值合并，再交给供应商插件适配成自己的专有字段。

任何协议进来，先 `decode_request` 成 canonical；调用驱动；再把 canonical `encode_response` 回目标协议。跨协议转换因此只有一套逻辑。

当前 canonical 覆盖**文本、单轮/多轮对话、流式与非流式**：消息角色只有 `system / user / assistant`，不支持 tool / function calling、图片/音频内容块。`reasoning_content` 会尽量从 DeepSeek 的 `reasoning_content`、Anthropic 的 `thinking` 块、Responses 的 reasoning 项目里提取，但不保证所有供应商都返回。

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
| 流式开关 | `stream: true` | `stream: true` | `stream: true` |
| 流式用量 | `stream_options.include_usage: true` | 事件里自带 `usage` | `message_start` / `message_delta` 分两段 |
| 思考深度 | 顶层 `reasoning_effort` | `reasoning.effort` | `output_config.effort` |
| 思考开关 | 无统一字段，供应商专有 | 无统一字段，供应商专有 | `thinking: { type, budget_tokens }` |

`base_url` 约定是**协议根地址**，不是完整 endpoint。OpenAI 兼容地址通常已经带 `/v1` 或 `/api/v3`，所以路径里不再重复版本段；Anthropic 的 `/v1` 属于 Messages 接口本身，保留在路径里。

例：

```text
https://api.deepseek.com/v1            + /chat/completions
https://api.deepseek.com/anthropic     + /v1/messages
https://ark.cn-beijing.volces.com/api/v3 + /responses
```

### SSE 增量对照

协议库的 `SseParser` 逐行解析 `text/event-stream`，支持 CRLF、跨 chunk 断行、末尾 flush，忽略注释和 `event:` 行；`decode_stream_event(protocol, data)` 把 `data:` 的 JSON 解成 `ChatDelta`。

| | OpenAI Chat | OpenAI Responses | Anthropic Messages |
|---|---|---|---|
| 文本增量 | `choices[0].delta.content` | `response.output_text.delta` 的 `delta` | `content_block_delta` 的 `text_delta.text` |
| 思考增量 | `choices[0].delta.reasoning_content` | `response.reasoning_summary_text.delta` / `response.reasoning_text.delta` | `content_block_delta` 的 `thinking_delta.thinking` |
| 结束 | `choices[0].finish_reason` + `[DONE]` | `response.completed` / `response.incomplete` | `message_delta.stop_reason` |
| 用量 | 末尾带 `usage` 的 chunk（`choices` 可能为空） | `response.completed.response.usage` | `message_start` 给 input，`message_delta` 给 output |
| 错误 | HTTP 非 2xx，或流内 `error` 对象 | `response.failed` / `error` 事件 | `error` 事件 |

Anthropic 的用量是分段给的，所以 `ChatResponseAccumulator` 对 `input_tokens` / `output_tokens` 取最大值合并，而不是简单相加。

## 3. 跨协议转换

### 请求

- `system` 消息：Chat 保留在 messages；Responses 合并到 `instructions`；Anthropic 提到顶层 `system`。
- 连续同角色消息：Anthropic 要求 user / assistant 交替，编码时用空行合并成一条。
- `max_tokens`：Responses 改名为 `max_output_tokens`；Anthropic 没给时补 `4096`（接口要求必填）。
- `response_format`：
  - `json_object`：Chat / Responses 原生支持；Anthropic 降级为 system 指令。
  - `json_schema`：Chat 原样传 `json_schema`；Responses 包成 `text.format`；Anthropic 把 schema 写进 system 指令。
- `stop`：Chat / Anthropic 原生支持；Responses 没有对应参数，**有损忽略**。
- `stream`：三协议都写顶层 `stream`。Chat 额外写 `stream_options.include_usage: true`，否则流末尾拿不到用量。
- `thinking`：
  - Chat / Responses 用 `reasoning_effort`（Responses 包在 `reasoning.effort`）；`disabled` 且没有 effort 时写 `"none"`。
  - Anthropic 写 `thinking: { type: enabled / disabled / adaptive, budget_tokens }` 和 `output_config.effort`；它只接受 `low / medium / high / xhigh / max`，`none` / `minimal` 映射到 `low`，预算保证小于 `max_tokens`。
  - `adaptive` 在 Chat / Responses 里不写额外开关，交给供应商默认；在 Anthropic 写 `type: adaptive`。

### 响应

- 文本统一抽取成 `content`；`reasoning_content` 单独保留。
- `finish_reason` 归一化成 `stop / length / content_filter / tool_calls / other`。
- 用量统一成 `input_tokens / output_tokens / total_tokens`；缺失时 `usage = None`。

有损点集中在上表的“Anthropic JSON”“Responses stop”两处。需要严格 JSON Schema 时，优先选 Chat 或 Responses；需要停止词时，不要选 Responses。

## 4. 注册层的协议桥

`LlmHub::chat` 接收 canonical 请求并返回完整响应；`LlmHub::chat_stream` 返回 `ChatStream`，边收边出 `ChatDelta`；`LlmHub::chat_raw(protocol, body)` 接收任一协议的原始 JSON，解码 → canonical → 调驱动 → 再按**入参协议**编码返回。

```rust
// 流式：底层驱动可以是任意供应商
let mut stream = hub.chat_stream(&request).await?;
while let Some(delta) = stream.next().await {
    let delta = delta?;
    if let Some(text) = delta.content {
        print!("{text}");
    }
}

// 协议桥：底层驱动可以是 Anthropic 配置
let out = hub.chat_raw(LlmProtocol::OpenAiChat, body).await?;
```

`chat_raw` 目前仍是**一次性 JSON 返回**的协议桥：即使入参 body 里 `stream: true`，供应商内部会走流式并聚合成完整响应，再由注册层编码成非流式 JSON。需要真正的 SSE 输出时，直接用 `chat_stream`。

驱动实际走什么协议由驱动自己的 `protocol` 配置决定，和调用方协议无关。因此不同供应商可以各自选协议：DeepSeek 配 `anthropic`、火山配 `openai-responses`、百炼配 `openai-chat`；`qianfan` / `gemini` 只能配 `openai-chat`，`zai` 可配 `openai-chat` / `anthropic`。上层只认一种格式。

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
      "max_tokens": 4096,
      "stream": true,
      "thinking": { "mode": "enabled", "effort": "high" }
    },
    "leo-plugin-llm-volcengine": {
      "protocol": "openai-chat",
      "base_url": "https://ark.cn-beijing.volces.com/api/v3",
      "model": "ep-2026xxxxxxxx",
      "stream": true,
      "thinking": "enabled",
      "reasoning_effort": "high"
    },
    "leo-plugin-llm-bailian": {
      "protocol": "openai-chat",
      "base_url": "https://dashscope.aliyuncs.com/compatible-mode/v1",
      "model": "qwen3.8-flash",
      "stream": true,
      "reasoning_effort": "high",
      "enable_search": false
    }
  }
}
```

| 驱动 | 环境变量 | 默认 `base_url` | 默认模型 | 流式默认 | 专有参数 |
|---|---|---|---|---|---|
| `deepseek` | `DEEPSEEK_API_KEY` | `https://api.deepseek.com/v1` | `deepseek-chat` | `true` | `thinking: { mode, effort }`（默认 enabled + high） |
| `volcengine` | `ARK_API_KEY` | `https://ark.cn-beijing.volces.com/api/v3` | 空，需填 `ep-...` 或模型 ID | `true` | `thinking: enabled / disabled / auto`、`reasoning_effort` |
| `bailian` | `DASHSCOPE_API_KEY` | `https://dashscope.aliyuncs.com/compatible-mode/v1` | `qwen-plus` | `true` | `enable_thinking`、`thinking_budget`、`reasoning_effort`、`enable_search` |
| `agnes` | `AGNES_API_KEY` | `https://apihub.agnes-ai.com/v1` | `agnes-3.0-flash` | `true` | 通用 `thinking: { mode, effort }` |
| `spark` | `SPARK_API_KEY`（兼容 `XFYUN_API_KEY`） | `https://maas-api.cn-huabei-1.xf-yun.com/v2` | `spark-x2.6` | `true` | 通用 `thinking`；Responses / Anthropic 根地址见供应商矩阵 |
| `qianfan` | `QIANFAN_API_KEY` | `https://qianfan.baidubce.com/v2` | `ernie-5.5-flash-preview` | `true` | 只支持 OpenAI Chat |
| `longcat` | `LONGCAT_API_KEY` | `https://api.longcat.chat/openai/v1` | `LongCat-2.5` | `true` | 通用 `thinking`；Anthropic 根地址见供应商矩阵 |
| `gemini` | `GEMINI_API_KEY`（兼容 `GOOGLE_API_KEY`） | `https://generativelanguage.googleapis.com/v1beta/openai` | `gemini-4-hydrogen` | `true` | 只支持 OpenAI Chat 兼容层 |
| `minimax` | `MINIMAX_LLM_API_KEY`（兼容 `MINIMAX_API_KEY`） | `https://api.minimax.cn/v1` | `MiniMax-M3.1-Flash` | `true` | 通用 `thinking`；Anthropic 根地址见供应商矩阵 |
| `zai` | `ZAI_API_KEY`（兼容 `ZHIPU_API_KEY`） | `https://open.bigmodel.cn/api/paas/v4` | `glm-5.4` | `true` | `thinking.type`（Chat）；Anthropic 根地址见供应商矩阵 |

这些驱动分别声明自己实际支持的协议：`agnes` / `spark` / `longcat` / `minimax` 支持三种；`zai` 支持 Chat + Anthropic；`qianfan` / `gemini` 只支持 OpenAI Chat。完整矩阵、每家的模型清单和“只接自家模型”的过滤规则见 [LLM 供应商矩阵](./llm-providers.md)。

能否真的用某个协议，取决于供应商是否提供对应端点。切换上游协议时改 `protocol` 和对应的 `base_url`，请求/响应体由协议库转换；没有公开端点的协议不硬做。调用方仍可用其他协议格式进来，由注册层的 `chat_raw` 做协议转换。

**供应商专有字段由各插件自己负责**：

- DeepSeek 的 `thinking: { type: enabled / disabled }` 只在 `openai-chat` 协议下注入；`reasoning_effort` 由协议库先写，DeepSeek 插件再把自己的可选档位收敛到 `high / max`：`low / medium / high` 都发 `high`，`xhigh / max` 发 `max`。
- 火山方舟的 `thinking.type` 只在 `openai-chat` 下注入；`auto` 会映射成 `auto`，当前官方 Chat 文档主要列 `enabled / disabled`，部分模型不接受 `auto` 时会报错。方舟本身支持完整的 `none / minimal / low / medium / high / xhigh / max`，插件不再二次映射。
- 百炼的 `enable_thinking`、`thinking_budget` 只在 `openai-chat` 下注入；`reasoning_effort` 由协议库先写，百炼插件再收敛到 Qwen3.8 的 `low / medium / xhigh`：`minimal / low` 发 `low`，`medium` 发 `medium`，`high / xhigh / max` 发 `xhigh`。qwen3.8 / qwen4 系列**不能同时**设置 `reasoning_effort` 与 `thinking_budget`，插件会在编码前直接报错；`none` 会关闭思考。

**逐请求覆盖**：`ChatRequest` 里的 `stream` / `thinking` 优先于 `settings`，所以同一个驱动可以按请求切换：

```rust
// 临时关闭流式
ChatRequest { stream: Some(false), ..Default::default() }

// 临时关闭思考
ChatRequest {
    thinking: ThinkingConfig { mode: ThinkingMode::Disabled, ..Default::default() },
    ..Default::default()
}

// 临时提高思考深度
ChatRequest {
    thinking: ThinkingConfig { effort: Some(ReasoningEffort::Max), ..Default::default() },
    ..Default::default()
}
```

`stream: None` 表示用驱动默认（三家默认都是 `true`）；`ThinkingConfig::default()` 表示不覆盖驱动默认。`effort = none` 会被归一化成关闭思考。

`extra_body` 会合并进最终请求体，用于供应商的新参数；`extra_headers` 追加到请求头。供应商专有字段在 `extra_body` 之后写入，同名时以专有字段为准。`temperature` 限 0.0~2.0，`top_p` 限 0.0~1.0，`max_tokens` 必须大于 0。

API Key 只从环境变量读，不写进 `leo-plugins.json`。没有 Key 时插件照常装配，但不注册驱动，日志里会有警告。

## 6. 手动联调

### 6.1 不花钱的假服务器测试

三套协议、鉴权头、请求体映射都有本地假服务器测试，不需要 Key。每个供应商还会额外跑：

- `default_chat_uses_sse_and_aggregates`：默认流式，把增量聚合成完整响应。
- `chat_stream_yields_incremental_deltas`：`chat_stream` 逐条产出 `content` / `reasoning_content` / `usage`。

```bash
cargo test -p leo-plugin-llm-deepseek
cargo test -p leo-plugin-llm-volcengine
cargo test -p leo-plugin-llm-bailian
cargo test -p leo-plugin-llm-protocol
```

### 6.2 真实调用单个供应商

先把 Key 放进 `.env`，再导入当前 shell：

```bash
set -a; source .env; set +a
```

然后跑各插件自带的 `#[ignore]` 真实调用测试（会调用线上接口并计费）：

```bash
cargo test -p leo-plugin-llm-deepseek live_chat -- --ignored --nocapture
cargo test -p leo-plugin-llm-bailian live_chat -- --ignored --nocapture

# 火山方舟必须指定推理接入点 ID
ARK_MODEL=ep-2026xxxxxxxx \
  cargo test -p leo-plugin-llm-volcengine live_chat -- --ignored --nocapture
```

默认模型：DeepSeek `deepseek-chat`，百炼 `qwen-plus`，火山方舟由 `ARK_MODEL` 决定。

### 6.3 自定义问题 / 模型

`plugins/catalog/examples/llm_chat.rs` 会装配完整 catalog，再通过 `LlmHub::chat_stream` 发一轮对话，`[thinking]` 和 `[content]` 都边收边打印，结束后打印 `finish_reason` / `usage`：

```bash
# 用默认驱动 + 默认模型
cargo run -p leo-plugin-catalog --example llm_chat -- "用一句话介绍有声书制作"

# 指定驱动和模型
cargo run -p leo-plugin-catalog --example llm_chat -- "你好" deepseek:deepseek-chat
cargo run -p leo-plugin-catalog --example llm_chat -- "你好" volcengine:ep-2026xxxxxxxx
cargo run -p leo-plugin-catalog --example llm_chat -- "你好" bailian:qwen3.8-flash
```

它会先打印当前注册了哪些 LLM 驱动；如果一个都没有，说明对应的 API Key 没有加载成功。想用自定义 `leo-plugins.json` 时加 `LEO_PLUGINS_CONFIG=./leo-plugins.json`。

## 7. 新增一家供应商

1. 新建 `plugins/llm/<name>/`（crate `leo-plugin-llm-<name>`），依赖 `leo-core` + `leo-llm-protocol`。
2. `config.rs` 定义 `base_url` / `protocol` / `model` / 采样参数 / `stream` 默认值，按需加 `thinking`、`reasoning_effort`、`thinking_budget` 这类供应商专有字段，以及 `extra_headers` / `extra_body`。
3. `client.rs` 用 `leo_llm_protocol::{endpoint, auth_headers, encode_request, decode_response, decode_error, SseParser, decode_stream_event}`；公共协议字段交给协议库，供应商专有字段在 `build_body` 里自己注入。
4. 实现 `chat`（按配置决定是否流式，流式时聚合）和 `chat_stream`（强制流式）；两者共用同一个 `request_builder` / `request_stream`。
5. `lib.rs` 在 `apply` 里 `ctx.register::<dyn LlmProvider>("<name>", ...)`；没有 Key 就只警告、不注册。
6. 根 `Cargo.toml` 加 member，`plugins/catalog` 加依赖和 `CATALOG` 项。
7. 测试：三套协议各一个假服务器用例，加一个默认流式聚合用例、一个 `chat_stream` 增量用例，再加一个 `#[ignore]` 真实调用用例。

不需要改 core、协议库、注册层、API/MCP/CLI。
