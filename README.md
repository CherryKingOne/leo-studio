# Leo Studio

现代化、可扩展的微内核插件化有声书制作与剧本生产工作台。

底层基于类似 Cordis 的微内核架构设计：**`core` 抽象规约 + `plugins` 分类能力扩展 + `app` 极简总装宿主**。无论是剧本文档导入/导出、文本分章与台词切分、多音字注音纠偏、还是 TTS 语音合成，皆以插件形式挂载与管理。

---

## 目录结构

```text
leo-studio/
├── core/                                  # 【微内核】纯净 AST 抽象、Context 依赖注入、扩展契约 SPI
│
├── plugins/                               # 【分类归集插件集】
│   ├── importers/                         # 📥 导入器插件（txt 等）
│   ├── exporters/                         # 📤 导出器插件（csv 台账、json 剧本等）
│   ├── text/                              # ✍️ 文本处理插件（颜文字清洗、正则分章、引号台词切分）
│   ├── tts/                               # 🎙️ 语音合成插件（微软 Edge TTS 等）
│   ├── storage/                           # 💾 数据存储插件（纯文本 CSV 文件存储）
│   ├── api/                               # 🌐 RESTful API 传输服务插件（Axum HTTP /api/v2）
│   └── mcp/                               # 🤖 MCP 智能体协议服务插件（Model Context Protocol）
│
├── app/                                   # 【启动器 Host App】极简总装入口（main.rs + lib.rs）
│
├── docs/solutions/                        # 📐 架构设计方案与演进路线图
│
├── 开局合欢宗，被师姐拿捏命脉.txt           # 📖 示例语料
├── 测试.txt                               # 📖 测试语料
└── Cargo.toml                             # 根工作区配置
```

---

## 快速开始

### 1. 环境准备
- **Rust** 1.85+（推荐 Stable，支持 Rust 2024 edition）
- 检查环境：
  ```bash
  rustc --version
  cargo --version
  ```

### 2. 启动服务指令

在项目根目录下，直接执行以下命令即可启动服务：

```bash
# 开发模式启动（编译快、适合本地开发调试）
cargo run -p leo-app

# 或 Release 模式启动（极致性能）
cargo run --release -p leo-app
```

> **提示**：你也可以进入 `app` 目录直接运行 `cargo run`：
> ```bash
> cd app
> cargo run
> ```

终端输出以下日志即表示启动成功：
```text
INFO leo_app: 已挂载 CsvStoragePlugin，数据存储目录: ".leo-studio"
INFO leo_app: 官方内置插件已全部装配完毕
INFO leo_app: 挂载 MCP HTTP 服务端点: /mcp
INFO leo_app: Leo Studio 后端微内核已启动：http://127.0.0.1:58685
```

---

## 服务访问与接口验证

默认服务地址为：**`http://127.0.0.1:58685`**

### 1. RESTful API 常用端点

#### ① 健康检查
```bash
curl http://127.0.0.1:58685/health
```
响应示例：
```json
{"app":"leo-studio","status":"ok","version":"0.2.0"}
```

#### ② 查询当前系统已激活的插件清单
```bash
curl http://127.0.0.1:58685/api/v2/plugins
```
响应中将列出所有当前加载的官方插件（包括 Importer、Exporter、TTS、Storage、Cleaner、Parser 等）。

#### ③ 查询支持的 TTS 合成声线列表
```bash
curl http://127.0.0.1:58685/api/v2/voices
```
输出包含微软 Edge 高保真男女声线（如 `zh-CN-YunxiNeural`, `zh-CN-XiaoxiaoNeural`, `zh-CN-YunjianNeural` 等）。

#### ④ 剧本文档导入（使用 Importer 插件解析为 AST）
```bash
curl -X POST http://127.0.0.1:58685/api/v2/import \
  -F "file=@测试.txt"
```

#### ⑤ 剧本文档导出（使用 Exporter 插件）
```bash
# 导出为配音制作台账 CSV
curl -O -J "http://127.0.0.1:58685/api/v2/export/{project_id}?format=csv"

# 导出为完整结构化剧本 JSON
curl -O -J "http://127.0.0.1:58685/api/v2/export/{project_id}?format=json"
```

#### ⑥ 章节语音合成与音频切片获取
```bash
# 对指定章节的所有台词/旁白切片执行一键语音合成
curl -X POST http://127.0.0.1:58685/api/v2/projects/{project_id}/chapters/{chapter_id}/synthesize \
  -H "Content-Type: application/json" \
  -d '{"voice":"zh-CN-YunxiNeural","speed":1.0,"pitch":0.0}'

# 试听或下载生成的音频切片（MP3 字节流）
curl -O "http://127.0.0.1:58685/api/v2/audio/{clip_id}"
```

---

### 2. MCP 智能体协议服务（Model Context Protocol）

服务内置了完整的 MCP 协议支持，挂载于 **`http://127.0.0.1:58685/mcp`**。

无需单独启动进程，AI 客户端（如 Claude Desktop、Cursor、Antigravity、VS Code 等）可直接连接该 URL，调用内置的 20+ 个自动化工具：
- `leo_list_projects` / `leo_get_project`：查询剧目库
- `leo_create_project` / `leo_append_chapters`：导入新剧本或追加分章
- `leo_get_chapter_content`：读取章节及台词/旁白切片
- `leo_synthesize_chapter`：智能体直接指挥生成章节语音
- `leo_list_voices` / `leo_list_plugins`：查询可用声线与插件

---

## 环境变量配置（可选）

如需自定义端口或数据目录，可在根目录下创建 `.env` 文件：

```bash
# 服务监听地址与端口
LEO_HOST=127.0.0.1
LEO_PORT=58685

# 存储目录配置
LEO_DATA_DIR=.
LEO_CSV_DIR=.leo-studio
LEO_UPLOAD_DIR=storage/uploads

# MCP 协议开关与路径
LEO_MCP_ENABLED=true
LEO_MCP_HTTP_PATH=/mcp
LEO_MCP_WRITE_TOOLS=true
LEO_MCP_DESTRUCTIVE_TOOLS=true

# 日志输出级别（trace, debug, info, warn, error）
RUST_LOG=info
```

---

## 自动化测试验证

项目中包含了完整的单元测试与端到端集成测试（包括 Edge-TTS 在线合成真机验证）：

```bash
# 运行全工作区所有模块的全部测试
cargo test --workspace

# 仅测试主启动器与 HTTP API 端点
cargo test -p leo-app

# 仅测试微内核与流水线 AST
cargo test -p leo-core

# 仅测试 Edge TTS 语音合成插件
cargo test -p leo-plugin-tts-edge
```

---

## 架构扩展说明

若要开发自己的扩展插件（例如接入 CosyVoice 或 GPT-SoVITS 合成引擎，或者扩展 SQLite 存储）：
1. 在 `plugins/` 对应分类目录下创建新的 Cargo Crate；
2. 引入 `leo-core`，实现核心提供的对应 SPI 特征（如 `TTSProvider`, `StorageProvider`, `Importer`, `Exporter`）；
3. 在 `app/src/main.rs` 中通过 `plugin_manager.register(...)` 注册挂载即可。详细设计思路见 [docs/solutions/plugin-framework-architecture.md](docs/solutions/plugin-framework-architecture.md)。
