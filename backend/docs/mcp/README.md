# Leo Studio MCP Server

把 Leo Studio 的剧目、章节、切句与多音字标注能力，以 **Model Context Protocol** 暴露给 AI 客户端（Claude、Cursor、VS Code、Cherry Studio 等）。

当前实现**只支持最新协议 `2026-07-28`（无状态核心）**，通过 **Streamable HTTP** 挂载在现有 Axum 进程内；不提供 stdio 进程，也不兼容旧版 `initialize` 握手。

## 能力

| 类别 | 内容 |
| --- | --- |
| 传输 | Streamable HTTP：`POST /mcp`，与 REST 同进程 |
| 协议 | `server/discover`、`tools/list`、`tools/call`、`resources/*`、`prompts/*`；结果带 `resultType: "complete"` |
| 工具 | 10 个只读；7 个写入（默认关闭）；4 个破坏性（额外开关 + `confirm`） |
| 资源 | `leo://projects`、`leo://stats` 及剧目/章节参数化资源 |
| 提示词 | 剧目总览、导入小说、章节摘要 |

## 启动

```bash
cd backend
LEO_DATA_DIR=./.data \
cargo run --bin leo-studio-backend
```

端点：`http://127.0.0.1:58685/mcp`

请求示例：

```bash
curl -sS -X POST http://127.0.0.1:58685/mcp \
  -H 'Content-Type: application/json' \
  -H 'Accept: application/json, text/event-stream' \
  -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: server/discover' \
  -d '{"jsonrpc":"2.0","id":1,"method":"server/discover","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"curl","version":"1.0"},"io.modelcontextprotocol/clientCapabilities":{}}}}'
```

`2026-07-28` 要求：

- `MCP-Protocol-Version` 请求头；
- `Mcp-Method` 请求头，`tools/call` / `resources/read` / `prompts/get` 还要带 `Mcp-Name`；
- 请求 `params._meta` 内的 `io.modelcontextprotocol/protocolVersion`、`clientInfo`、`clientCapabilities`；
- 缺头返回 `400` + `-32020`，版本不符返回 `-32020`，不支持的版本返回 `400` + `-32022`。

## 客户端配置

端点固定为 `http://<主机>:58685/mcp`。

Cursor（`.cursor/mcp.json`）：

```json
{
  "mcpServers": {
    "leo-studio": { "url": "http://127.0.0.1:58685/mcp" }
  }
}
```

VS Code（`.vscode/mcp.json`）：

```json
{
  "servers": {
    "leo-studio": { "type": "http", "url": "http://127.0.0.1:58685/mcp" }
  }
}
```

Claude Code（`.mcp.json`）：

```json
{
  "mcpServers": {
    "leo-studio": { "type": "http", "url": "http://127.0.0.1:58685/mcp" }
  }
}
```

Claude Desktop 原生只支持 stdio，需要额外桥接。完整配置、JSON-RPC 请求示例与错误码见 [clients.md](clients.md)。

## 配置

| 环境变量 | 默认值 | 说明 |
| --- | --- | --- |
| `LEO_MCP_ENABLED` | `true` | 是否挂载 `/mcp`（默认开启） |
| `LEO_MCP_HTTP_PATH` | `/mcp` | HTTP 端点路径 |
| `LEO_MCP_WRITE_TOOLS` | `false` | 注册写入工具 |
| `LEO_MCP_DESTRUCTIVE_TOOLS` | `false` | 注册破坏性工具 |

文本返回上限固定 20,000 字符，分页上限固定 100，不再单独配置。

## 工具

完整参数与返回见 [tools.md](tools.md)。只读工具默认全部可用：

`leo_health`、`leo_list_projects`、`leo_get_project`、`leo_list_chapters`、`leo_get_chapter`、`leo_search_chapters`、`leo_list_recycle_bin`、`leo_get_sentence_polyphones`、`leo_list_chapter_polyphones`、`leo_get_stats`

写入工具（`LEO_MCP_WRITE_TOOLS=true`）：

`leo_create_project`、`leo_append_chapters`、`leo_rename_project`、`leo_update_project`、`leo_save_sentence_polyphones`、`leo_clear_sentence_polyphones`、`leo_restore_project`

破坏性工具（同时 `LEO_MCP_DESTRUCTIVE_TOOLS=true`，且必须传 `confirm=true`）：

`leo_delete_chapter`、`leo_move_to_trash`、`leo_purge_project`、`leo_empty_recycle_bin`

## 资源与提示词

| 资源 | 说明 |
| --- | --- |
| `leo://projects` | 剧目列表（最多 100 条） |
| `leo://projects/{project_id}` | 剧目详情 |
| `leo://projects/{project_id}/chapters` | 章节大纲 |
| `leo://projects/{project_id}/chapters/{chapter_id}` | 章节详情（正文截断） |
| `leo://projects/{project_id}/chapters/{chapter_id}/text` | 章节纯文本 |
| `leo://stats` | 全库统计 |

提示词：`leo_project_overview`、`leo_import_novel`、`leo_chapter_digest`。

## 安全边界

- 与 REST 共用 `LEO_HOST` / `LEO_PORT`（默认 `0.0.0.0:58685`），不做额外鉴权。
- 写工具与破坏性工具是双层开关；破坏性操作必须显式 `confirm=true`。
- `leo_append_chapters` 默认 `dry_run=true`，先预检再写。
- 导入只接受 `text` 参数，不读取服务器本地文件，避免路径穿越。
- `/mcp` 路由不套用 `X-Request-ID` 中间件，避免改写 JSON-RPC 响应。
- 当前版本不支持多实例共享：MCP 与 REST 共用进程内 CSV 存储，多实例部署需先补共享存储。

## 已验证

- HTTP：`server/discover` → `tools/list` → `tools/call`。
- 缺 `Mcp-Method` 返回 `-32020`；旧版本返回 `-32022`。
- 集成测试：`cargo test --test mcp`（6 例，覆盖工具开关、创建/查询/搜索/统计、追更预检与确认、多音字往返、破坏性确认）。

## 未实现

Sampling、Roots、Logging、`elicitation`、`subscriptions/listen`、Tasks 扩展、MCP Apps。这些能力不影响剧目/章节的查询与制作。