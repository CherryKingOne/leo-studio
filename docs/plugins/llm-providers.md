# LLM 供应商矩阵

Leo Studio 的 LLM 层是“共享协议库 + 注册层 + 供应商薄客户端”。本文只记录供应商差异：默认模型、协议支持、模型列表来源，以及“只接自家模型”的边界。

本次新增 7 个驱动，加上已有的 `deepseek` / `volcengine` / `bailian`，当前共 10 个 LLM 驱动。

## 1. 协议矩阵

| 驱动 | OpenAI Chat | OpenAI Responses | Anthropic | 默认模型 |
| :--- | :---: | :---: | :---: | :--- |
| `agnes` | ✅ | ✅ | ✅ | `agnes-3.0-flash` |
| `spark` | ✅ | ✅ | ✅ | `spark-x2.6` |
| `qianfan` | ✅ | — | — | `ernie-5.5-flash-preview` |
| `longcat` | ✅ | ✅ | ✅ | `LongCat-2.5` |
| `gemini` | ✅ | — | — | `gemini-4-hydrogen` |
| `minimax` | ✅ | ✅ | ✅ | `MiniMax-M3.1-Flash` |
| `zai` | ✅ | — | ✅ | `glm-5.4` |

“支持”指的是上游确实提供对应兼容入口；没有公开入口的协议不硬做。`qianfan` 目前只声明 OpenAI Chat；`gemini` 走 Gemini 的 OpenAI 兼容层，原生 Gemini 协议不在本项目的三协议范围内；`zai` 中国站公开文档没有 Responses 入口。

## 2. 协议根地址不一样

`base_url` 是**协议根地址**，不是完整 endpoint。插件会按协议追加路径：

- OpenAI Chat：`{base_url}/chat/completions`
- OpenAI Responses：`{base_url}/responses`
- Anthropic：`{base_url}/v1/messages`

同一家供应商的三套协议不一定共用一个根地址，切协议时要同时改 `protocol` 和 `base_url`：

| 驱动 | Chat / Responses 根地址 | Anthropic 根地址 |
| :--- | :--- | :--- |
| `agnes` | `https://apihub.agnes-ai.com/v1`（Chat、Responses 都用这个） | `https://apihub.agnes-ai.com` |
| `spark` | Chat `https://maas-api.cn-huabei-1.xf-yun.com/v2`；Responses `https://maas-api.cn-huabei-1.xf-yun.com/v1` | `https://maas-api.cn-huabei-1.xf-yun.com/anthropic` |
| `qianfan` | `https://qianfan.baidubce.com/v2` | — |
| `longcat` | `https://api.longcat.chat/openai/v1` | `https://api.longcat.chat/anthropic` |
| `gemini` | `https://generativelanguage.googleapis.com/v1beta/openai` | — |
| `minimax` | `https://api.minimax.cn/v1` | `https://api.minimax.cn/anthropic` |
| `zai` | `https://open.bigmodel.cn/api/paas/v4` | `https://open.bigmodel.cn/api/anthropic` |

例如把 Agnes 切到 Anthropic：

```json
{
  "settings": {
    "leo-plugin-llm-agnes": {
      "protocol": "anthropic",
      "base_url": "https://apihub.agnes-ai.com",
      "model": "agnes-3.0-flash"
    }
  }
}
```

## 3. 模型列表从哪里来

`LlmProvider::list_models()` 的目标是回答两个问题：**这家支持哪些模型**、**当前驱动在用哪个模型**。

- 有统一 `/models` 的厂商：默认远端拉取，按自家前缀过滤，过滤失败或网络失败时回退内置清单；结果缓存 600 秒。
- 没有统一 `/models` 的厂商：直接返回内置白名单。公开文档没有稳定列表接口，硬猜一个 `/models` 只会在运行时 404。
- 所有厂商都支持 `allowed_models` 补充私有化模型；也支持配置 `models_url` 指向私有网关的列表接口。远端结果仍会经过前缀过滤，不会把第三方模型混进来。

| 驱动 | 模型列表来源 | 自家模型边界 |
| :--- | :--- | :--- |
| `agnes` | 远端 `https://apihub.agnes-ai.com/v1/models` + 内置 | `agnes-*` |
| `spark` | 内置白名单（讯飞公开文档没有统一 `/models`） | `spark-*` / `xspark*`；过滤模型广场里托管的 GLM / DeepSeek / Kimi / MiniMax / Qwen |
| `qianfan` | 内置白名单（千帆公开文档没有统一 `/models`） | `ernie-*`；过滤千帆托管的 DeepSeek / Qwen 等 |
| `longcat` | 远端 `https://api.longcat.chat/v1/models` + 内置 | `LongCat-*` |
| `gemini` | 远端 `https://generativelanguage.googleapis.com/v1beta/openai/models` + 内置 | `gemini-*`；过滤 TTS / Live / Image / Embedding / Omni 等 |
| `minimax` | 远端 `https://api.minimax.cn/v1/models` + 内置 | `MiniMax-M*`；过滤 Speech / Hailuo / Image / Music 等 |
| `zai` | 内置白名单（中国站公开文档没有统一 `/models`） | `glm-*`；过滤 embedding / rerank 等 |

`LlmHub::list_models()` 会把所有已注册驱动的结果聚合起来：默认驱动的模型不带前缀，其他驱动返回 `driver:model`。这样前端或运维侧既能看到“支持哪些模型”，也能看到“哪些模型实际可用”。本次只改 LLM 插件，不改 REST / MCP 接口层；接口层后续可以直接调用这个聚合方法。

## 4. 内置模型清单

下面是内置兜底清单。远端列表可用时，以远端过滤后的结果为准；厂商上新后，改对应插件的 `src/models.rs` 即可。

### Agnes

- 默认：`agnes-3.0-flash`
- 其他：`agnes-2.5-pro`、`agnes-2.5-pro-beta`

### 讯飞星辰（spark）

- 默认：`spark-x2.6`
- 其他：`spark-x2.5`、`xsparkx2flash`

### 百度千帆（qianfan）

- 默认：`ernie-5.5-flash-preview`（展示名 `ERNIE-5.5-Flash-Preview`）
- 其他：`ernie-5.1`、`ernie-5.0`、`ernie-4.5-turbo-128k`、`ernie-4.5-vl-28b-a3b`

### LongCat 龙猫（longcat）

- 默认：`LongCat-2.5`
- 其他：`LongCat-2.0`、`LongCat-Flash-Chat`、`LongCat-Flash-Thinking-2601`、`LongCat-Flash-Lite`

### Google Gemini（gemini）

- 默认：`gemini-4-hydrogen`
- 其他：`gemini-4-carbon`、`gemini-4-neon`、`gemini-4-argon`、`gemini-3.8-flash`、`gemini-3.6-flash`、`gemini-3.5-flash-lite`、`gemini-3.1-pro-preview`

### MiniMax（minimax）

- 默认：`MiniMax-M3.1-Flash`
- 其他：`MiniMax-M3`（M2.x 属于历史模型，未放进内置清单；如果账号仍可用，远端 `/models` 或 `allowed_models` 可以补回来）

### 智谱 Z.ai（zai）

- 默认：`glm-5.4`
- 其他：`glm-5.3`、`glm-5.3-flash`、`glm-5.3-flashx`、`glm-5.2`、`glm-5.1`、`glm-5`、`glm-4.7`

## 5. 配置与 Key

API Key 只从环境变量读取，不写进 `leo-plugins.json`。没找到 Key 时插件照常装配，但不注册驱动，不影响其他插件。

| 驱动 | 主环境变量 | 兼容变量 |
| :--- | :--- | :--- |
| `agnes` | `AGNES_API_KEY` | — |
| `spark` | `SPARK_API_KEY` | `XFYUN_API_KEY` |
| `qianfan` | `QIANFAN_API_KEY` | `BAIDU_QIANFAN_API_KEY` |
| `longcat` | `LONGCAT_API_KEY` | `MEITUAN_LONGCAT_API_KEY` |
| `gemini` | `GEMINI_API_KEY` | `GOOGLE_API_KEY`、`GOOGLE_GENERATIVE_AI_API_KEY` |
| `minimax` | `MINIMAX_LLM_API_KEY` | `MINIMAX_API_KEY`（与 TTS 共用时可只填这个） |
| `zai` | `ZAI_API_KEY` | `ZHIPU_API_KEY`、`ZHIPUAI_API_KEY`、`BIGMODEL_API_KEY` |

示例配置见仓库根目录的 `leo-plugins.example.json`；环境变量模板见 `.env.example`。

## 6. 官方文档

- Agnes：<https://www.agnes-ai.com/zh-Hans/docs/quickstart>
- 讯飞星辰：<https://maas.xfyun.cn/doc-viewer?url=https://maas.xfyun.cn/document/guide/1%E3%80%81%E5%BF%AB%E9%80%9F%E5%BC%80%E5%A7%8B/1.1%20API%E6%8E%A5%E5%85%A5.html>
- 百度千帆：<https://cloud.baidu.com/doc/qianfan-api/s/3m7of64lb>
- LongCat：<https://longcat.chat/platform/docs/zh/api-docs>
- Google Gemini：<https://aistudio.google.com/docs>、<https://aistudio.google.com/api-reference>
- MiniMax：<https://platform.minimax.cn/docs/api-reference/api-overview>、<https://platform.minimax.cn/docs/guides/models-intro>
- 智谱 Z.ai：<https://docs.bigmodel.cn/cn/api/introduction>、<https://docs.bigmodel.cn/cn/guide/start/introduction>
