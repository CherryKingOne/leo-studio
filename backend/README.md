# Leo Studio Backend

Leo Studio 后端服务，使用 **Rust + Axum + Tokio** 实现，前后端通过 HTTP / JSON 分离。
前端位于仓库根目录的 `desktop/`，本服务负责文本文件上传、小说章节解析、切句、项目与多音字标注管理。

## 特性

- 统一响应包装 `{ code, message, data, request_id }`，`X-Request-ID` 贯穿请求头与响应体
- CSV 持久化，兼容旧版 Python 服务的数据文件
- 章节标题识别、中文数字归一化、防脏数据覆盖的追加上传
- 行级切句与颜文字 / 分隔线 / 省略号清洗
- 项目软删除、回收站、恢复、物理级联删除
- 句子多音字标注的校验、全量覆盖保存与级联清理
- **MCP Server**：把剧目/章节/切句/多音字能力暴露给 AI 客户端（仅 `2026-07-28`）
- 默认监听 `58685`

## 快速开始

后端以 **API Server 模式**运行，启动后对外提供 REST 接口：

```bash
cd backend
cp .env.example .env          # 可选
cargo run --release --bin leo-studio-backend   # 以 API Server 模式启动
cargo run --bin leo-studio-backend             # 开发模式启动
```

服务地址：<http://127.0.0.1:58685>

健康检查：

```bash
curl http://127.0.0.1:58685/health
```

## MCP Server

MCP 层是 REST 之外的第二个入口，复用同一套 `projects` 业务服务，挂载在现有 Axum 进程内。当前只支持最新协议 `2026-07-28`（无状态核心），不兼容旧版握手。

```bash
LEO_MCP_ENABLED=true LEO_MCP_TOKEN=replace-me \
  cargo run --bin leo-studio-backend
```

- 端点默认 `/mcp`，需要 `MCP-Protocol-Version`、`Mcp-Method` 头与请求内 `_meta`。
- 只读工具默认可用；写入/破坏性工具分别由 `LEO_MCP_WRITE_TOOLS`、`LEO_MCP_DESTRUCTIVE_TOOLS` 控制。
- 不提供 stdio 进程；客户端需支持 Streamable HTTP。

详见 [docs/mcp/README.md](docs/mcp/README.md) 与 [docs/mcp/tools.md](docs/mcp/tools.md)。

## 常用配置

| 环境变量 | 默认值 | 说明 |
| --- | --- | --- |
| `LEO_HOST` | `0.0.0.0` | 监听地址 |
| `LEO_PORT` | `58685` | 监听端口 |
| `LEO_DATA_DIR` | `.` | 数据根目录 |
| `LEO_CSV_DIR` | `<data>/.leo-studio` | CSV 目录 |
| `LEO_UPLOAD_DIR` | `<data>/storage/uploads/text-files` | 上传目录 |
| `LEO_MAX_UPLOAD_SIZE_MB` | `50` | 全局上传上限 |
| `LEO_TEXT_FILES_ALLOWED_EXTENSIONS` | `.txt` | 允许的文件扩展名 |

完整配置见 [docs/api/configuration.md](docs/api/configuration.md)。

## 数据文件

```text
.leo-studio/
├── novels.csv
├── novel_contents.csv
├── novel_sentences.csv
├── novel_polyphones.csv
└── projects.csv
storage/uploads/text-files/
```

## 开发

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

## 目录结构

详见 [STRUCTURE.md](STRUCTURE.md)。接口文档见 [docs/api](docs/api)。