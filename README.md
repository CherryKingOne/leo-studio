# 🎙️ Leo Studio

现代化、可扩展的微内核插件化有声书制作与剧本生产工作台。

底层基于类似 **Cordis** 的微内核架构设计：**`core` 抽象规约 + `plugins` 分类能力扩展 + `app` 极简总装宿主**。无论是剧本文档导入/导出、文本分章与台词切分、多音字注音纠偏、还是 TTS 语音合成，皆以插件形式解耦挂载与分类管理。

---

## ⚡ 核心功能与文档快速导航

Leo Studio 原生支持三种使用范式，并提供完备的插件生态开发标准：

| 专区 | 说明 | 快速链接 |
| :--- | :--- | :--- |
| 🚀 **三大使用方式概览** | 全景了解 MCP 智能体驱动、CLI 命令行生产、RESTful API 接口的对比与选型 | [👉 点击查阅指南](docs/guides/three-ways-to-use.md) |
| 🤖 **方式一：MCP 智能体驱动** | 面向 AI Agent（Claude, Cursor, Antigravity, OpenManus）全自动制作有声书 | [👉 查看 MCP 使用手册](docs/guides/mcp-agent-guide.md) |
| 💻 **方式二：CLI 命令行交互** | 免启动服务、本地极速批量流水线（`leo-cli`）导入、切分、导出与合成 | [👉 查看 CLI 操作手册](docs/guides/cli-operations-guide.md) |
| 🌐 **方式三：RESTful API 接口** | 面向现代 Web 工作台与多端跨平台系统的标准 HTTP 接口（`/api/v2/*`） | [👉 查看 API 开发者文档](docs/guides/api-integration-guide.md) |
| 🔌 **插件开发者协议中心** | 插件架构哲学、`PluginManifest` 协议规范、核心 SPI 扩展点与保姆级实战开发 | [👉 进入插件开发者中心](docs/plugins/README.md) |

---

## 📐 纯净微内核目录结构

所有业务能力皆以插件形式分类归集在 `plugins/` 对应子目录中，内核与插件单向解耦：

```text
leo-studio/
├── core/                                  # 【微内核】纯净 AST 抽象、Context 依赖注入、扩展契约 SPI
│
├── plugins/                               # 【分类归集插件集】
│   ├── importers/                         # 📥 导入器插件（txt 等，解析为 Script AST）
│   ├── exporters/                         # 📤 导出器插件（csv 配音台账、json 剧本等）
│   ├── text/                              # ✍️ 文本处理插件（颜文字清洗、正则分章、引号台词切分）
│   ├── tts/                               # 🎙️ 语音合成插件（微软 Edge TTS、CosyVoice 等）
│   ├── storage/                           # 💾 数据存储插件（纯文本 CSV 文件存储等）
│   ├── api/                               # 🌐 RESTful API 传输服务插件（Axum HTTP /api/v2）
│   ├── mcp/                               # 🤖 MCP 智能体协议服务插件（Model Context Protocol）
│   └── cli/                               # 💻 CLI 命令行生产工具插件（leo-cli）
│
├── app/                                   # 【启动器 Host App】极简总装入口（main.rs + lib.rs）
│
├── docs/                                  # 📚 完备文档库
│   ├── plugins/                           # 🔌 插件开发者协议专区（协议标准、SPI 契约、开发教程）
│   ├── guides/                            # 🚀 三大使用方式操作手册（MCP、CLI、API）
│   └── solutions/                         # 📐 架构设计方案与演进路线
│
├── 开局合欢宗，被师姐拿捏命脉.txt           # 📖 示例语料
├── 测试.txt                               # 📖 测试语料
└── Cargo.toml                             # 根工作区配置
```

---

## 🏁 快速开始：服务启动指令

### 1. 环境准备
- **Rust** 1.85+（推荐 Stable，支持 Rust 2024 edition）
- 检查环境版本：
  ```bash
  rustc --version
  cargo --version
  ```

### 2. 启动服务指令 (API & MCP 传输服务)

在项目根目录下直接运行：

```bash
# 开发模式启动（编译快，适合本地开发与调试）
cargo run -p leo-app

# 或 Release 模式启动（极致性能）
cargo run --release -p leo-app
```

> **提示**：你也可以直接进入 `app` 目录执行 `cargo run`。

终端输出以下就绪日志即表示启动成功：
```text
INFO leo_app: 已挂载 CsvStoragePlugin，数据存储目录: ".leo-studio"
INFO leo_app: 官方内置插件已全部装配完毕
INFO leo_app: 挂载 MCP HTTP 服务端点: /mcp
INFO leo_app: Leo Studio 后端微内核已启动：http://127.0.0.1:58685
```

---

## 🎯 三大使用方式简介

### 方式一：让 AI Agent 驱动有声书制作 (MCP)
- **协议端点**：`http://127.0.0.1:58685/mcp`
- 将此 URL 填入 Claude Desktop、Cursor 或 Antigravity 的 `mcpServers` 配置，AI 即可直接调用 `leo_list_projects`, `leo_create_project`, `leo_get_chapter`, `leo_synthesize_chapter` 等 20+ 个智能工具，实现自然语言指挥生产。
- 详情查阅：[📘 MCP 智能体驱动指南](docs/guides/mcp-agent-guide.md)

### 方式二：命令行本地极速批量生产 (`leo-cli`)
- **零网络端口依赖**：无需启动 HTTP 服务，直接读写本地目录。
  ```bash
  # 导入一部小说并自动清洗分章
  cargo run -p leo-plugin-cli -- import 测试.txt

  # 查看已存储的剧目列表
  cargo run -p leo-plugin-cli -- list

  # 导出配音制作台账 CSV
  cargo run -p leo-plugin-cli -- export <PROJECT_ID> --format csv

  # 一键批量合成指定章节所有片段
  cargo run -p leo-plugin-cli -- synthesize <PROJECT_ID> --chapter 0 --voice zh-CN-YunxiNeural
  ```
- 详情查阅：[📘 CLI 命令行极速生产手册](docs/guides/cli-operations-guide.md)

### 方式三：RESTful API 现代 Web 接口
- **服务基础地址**：`http://127.0.0.1:58685`
  - 健康检查：`GET /health`
  - 插件清单：`GET /api/v2/plugins`
  - 可用声线：`GET /api/v2/voices`
  - 文档导入：`POST /api/v2/import` (Multipart)
  - 台账/AST 导出：`GET /api/v2/export/{project_id}?format=csv|json`
  - 章节批量合成：`POST /api/v2/projects/{project_id}/chapters/{chapter_id}/synthesize`
  - 音频流播放/下载：`GET /api/v2/audio/{clip_id}`
- 详情查阅：[📘 RESTful API 开发者集成指南](docs/guides/api-integration-guide.md)

---

## 🔌 插件开发者专区

Leo Studio 为开发者提供了高度扩展能力的插件生态体系。所有功能插件均基于微内核提供的扩展契约（SPI）进行开发：

1. [**插件协议与契约规范 (Protocol Specification)**](docs/plugins/protocol-specification.md)：
   - `Plugin` 根契约与 `apply(&self, ctx: &Context)` 初始化机制。
   - `PluginManifest` 元数据与分类约束（`PluginCategory`）。
   - Context 依赖注入（`ctx.provide` / `ctx.inject`）与 EventBus 事件总线。
2. [**核心 SPI 扩展点详解 (SPI Contracts)**](docs/plugins/spi-contracts.md)：
   - `Importer`：支持各类电子书与文档解析为 `ScriptDocument` AST。
   - `Exporter`：支持导出各类制作台账与工程文件。
   - `TTSProvider`：接入任意本地大模型或云端语音合成供应商。
   - `StorageProvider`：持久化扩展（CSV, SQLite, 对象存储）。
   - `TextCleaner`, `ChapterSplitter`, `DialogueSplitter`：文本流水线扩展。
3. [**插件实战开发教程 (Development Guide)**](docs/plugins/development-guide.md)：
   - 手把手保姆级教程：从 0 到 1 编写一个 CosyVoice TTS 插件与 EPUB 导入器。

---

## ⚙️ 环境变量配置（可选）

如需自定义端口或数据目录，可在根目录下创建 `.env` 文件：

```bash
# 服务监听地址与端口
LEO_HOST=127.0.0.1
LEO_PORT=58685

# 存储目录配置
LEO_DATA_DIR=.
LEO_CSV_DIR=.leo-studio
LEO_UPLOAD_DIR=storage/uploads

# MCP 协议开关与权限
LEO_MCP_ENABLED=true
LEO_MCP_HTTP_PATH=/mcp
LEO_MCP_WRITE_TOOLS=true
LEO_MCP_DESTRUCTIVE_TOOLS=true

# 日志输出级别（trace, debug, info, warn, error）
RUST_LOG=info
```

---

## 🧪 自动化测试验证

项目中包含了完整的单元测试与端到端集成测试（包括 Edge-TTS 在线合成真机验证）：

```bash
# 运行全工作区所有模块的全部测试（微内核、各类插件、命令行、宿主 API）
cargo test --workspace

# 仅测试主启动器与 HTTP API 端点
cargo test -p leo-app

# 仅测试微内核与流水线 AST
cargo test -p leo-core

# 仅测试 Edge TTS 语音合成插件
cargo test -p leo-plugin-tts-edge
```
