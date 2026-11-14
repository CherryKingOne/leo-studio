# 客户端配置与调用方法

MCP 端点：`http://<主机>:58685/mcp`（与 REST 同进程，默认开启，无鉴权）

- 协议：`2026-07-28`（Streamable HTTP，无会话）
- 不需要 `initialize`，每个请求都要带 `_meta`
- 不支持旧版握手与 SSE

## 一、客户端 JSON 配置

### Cursor

项目级 `.cursor/mcp.json`，或全局 `~/.cursor/mcp.json`：

```json
{
  "mcpServers": {
    "leo-studio": {
      "url": "http://127.0.0.1:58685/mcp"
    }
  }
}
```

### VS Code

`.vscode/mcp.json`：

```json
{
  "servers": {
    "leo-studio": {
      "type": "http",
      "url": "http://127.0.0.1:58685/mcp"
    }
  }
}
```

### Claude Code

项目根目录 `.mcp.json`：

```json
{
  "mcpServers": {
    "leo-studio": {
      "type": "http",
      "url": "http://127.0.0.1:58685/mcp"
    }
  }
}
```

### Cherry Studio / 其他支持 Streamable HTTP 的客户端

新增 MCP 服务器时选择「HTTP / Streamable HTTP」，URL 填：

```text
http://127.0.0.1:58685/mcp
```

### Claude Desktop

Claude Desktop 原生只支持 stdio，不能直接填 HTTP URL。若必须接入，需要额外的 stdio→HTTP 桥（如 `mcp-remote`），这会在本机多一个桥接进程；本项目不内置。

## 二、调用方法

所有请求都是 `POST /mcp`，`Content-Type: application/json`，`Accept: application/json, text/event-stream`。

`2026-07-28` 必需请求头：

| 请求头 | 值 | 说明 |
| --- | --- | --- |
| `MCP-Protocol-Version` | `2026-07-28` | 必须与请求 `_meta` 一致 |
| `Mcp-Method` | 方法名，如 `tools/list` | `tools/call` 还要带 `Mcp-Name` |
| `Mcp-Name` | 工具名 | 仅 `tools/call` / `resources/read` / `prompts/get` |

每个请求的 `params._meta` 至少包含：

```json
{
  "io.modelcontextprotocol/protocolVersion": "2026-07-28",
  "io.modelcontextprotocol/clientInfo": { "name": "my-client", "version": "1.0" },
  "io.modelcontextprotocol/clientCapabilities": {}
}
```

### 1. 发现服务

```bash
curl -sS -X POST http://127.0.0.1:58685/mcp \
  -H 'Content-Type: application/json' \
  -H 'Accept: application/json, text/event-stream' \
  -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: server/discover' \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "server/discover",
    "params": {
      "_meta": {
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientInfo": { "name": "curl", "version": "1.0" },
        "io.modelcontextprotocol/clientCapabilities": {}
      }
    }
  }'
```

返回：

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "resultType": "complete",
    "supportedVersions": ["2026-07-28"],
    "capabilities": { "prompts": {}, "resources": {}, "tools": {} }
  }
}
```

### 2. 列出工具

```bash
curl -sS -X POST http://127.0.0.1:58685/mcp \
  -H 'Content-Type: application/json' \
  -H 'Accept: application/json, text/event-stream' \
  -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: tools/list' \
  -d '{
    "jsonrpc": "2.0",
    "id": 2,
    "method": "tools/list",
    "params": {
      "_meta": {
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientInfo": { "name": "curl", "version": "1.0" },
        "io.modelcontextprotocol/clientCapabilities": {}
      }
    }
  }'
```

### 3. 调用工具

```bash
curl -sS -X POST http://127.0.0.1:58685/mcp \
  -H 'Content-Type: application/json' \
  -H 'Accept: application/json, text/event-stream' \
  -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: tools/call' \
  -H 'Mcp-Name: leo_list_projects' \
  -d '{
    "jsonrpc": "2.0",
    "id": 3,
    "method": "tools/call",
    "params": {
      "name": "leo_list_projects",
      "arguments": { "page": 1, "page_size": 20 },
      "_meta": {
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientInfo": { "name": "curl", "version": "1.0" },
        "io.modelcontextprotocol/clientCapabilities": {}
      }
    }
  }'
```

返回：

```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "result": {
    "resultType": "complete",
    "content": [{ "type": "text", "text": "{\"total\":0,\"page\":1,\"page_size\":20,\"pages\":0,\"items\":[]}" }],
    "structuredContent": { "total": 0, "page": 1, "page_size": 20, "pages": 0, "items": [] },
    "isError": false
  }
}
```

### 4. 读取资源

```bash
curl -sS -X POST http://127.0.0.1:58685/mcp \
  -H 'Content-Type: application/json' \
  -H 'Accept: application/json, text/event-stream' \
  -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: resources/read' \
  -H 'Mcp-Name: leo://stats' \
  -d '{
    "jsonrpc": "2.0",
    "id": 4,
    "method": "resources/read",
    "params": {
      "uri": "leo://stats",
      "_meta": {
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientInfo": { "name": "curl", "version": "1.0" },
        "io.modelcontextprotocol/clientCapabilities": {}
      }
    }
  }'
```

### 5. 获取提示词

```bash
curl -sS -X POST http://127.0.0.1:58685/mcp \
  -H 'Content-Type: application/json' \
  -H 'Accept: application/json, text/event-stream' \
  -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: prompts/get' \
  -H 'Mcp-Name: leo_project_overview' \
  -d '{
    "jsonrpc": "2.0",
    "id": 5,
    "method": "prompts/get",
    "params": {
      "name": "leo_project_overview",
      "arguments": {},
      "_meta": {
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientInfo": { "name": "curl", "version": "1.0" },
        "io.modelcontextprotocol/clientCapabilities": {}
      }
    }
  }'
```

## 三、错误码

| 场景 | HTTP | code |
| --- | --- | --- |
| 缺少 `MCP-Protocol-Version` / `Mcp-Method`，或头与 `_meta` 不一致 | 400 | `-32020` |
| 不支持的协议版本 | 400 | `-32022` |
| 未知工具 | 200 | `-32602` |
| 参数错误、剧目/章节不存在 | 200 | 工具结果 `isError=true` |
| `LEO_MCP_ENABLED=false` | 404 | — |

## 四、验证清单

1. 启动后端：`cargo run --bin leo-studio-backend`
2. 确认 `curl http://127.0.0.1:58685/api/v1/system/health` 返回 `ok`
3. 确认 `POST /mcp` 的 `server/discover` 返回 `supportedVersions: ["2026-07-28"]`
4. 在客户端里添加上面的 JSON 配置，连接后应能看到 `leo_*` 工具列表