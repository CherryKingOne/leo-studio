# Leo Studio Host App

Leo Studio 微内核主机启动程序，负责读取环境配置、装配启用的各类插件，并对外统一暴露现代化的 HTTP RESTful API 与 Model Context Protocol (MCP) 服务。

## 启动指令

在当前目录或项目根目录均可直接启动：

```bash
# 方式 1：在仓库根目录启动
cargo run -p leo-app

# 方式 2：在当前 app 目录下启动
cargo run

# Release 生产模式启动
cargo run --release
```

服务默认监听于 `http://127.0.0.1:58685`。

## 主要功能

- **极简总装**：仅包含启动引导代码与集成测试，不混杂具体业务与存储实现。
- **动态插件挂载**：启动时向微内核 Context 与 PluginManager 注册所选分类插件：
  - `leo-plugin-importer-txt`：TXT 剧本解析导入
  - `leo-plugin-exporter-csv` / `leo-plugin-exporter-json`：台账与 AST JSON 导出
  - `leo-plugin-tts-edge`：Edge 高保真语音合成
  - `leo-plugin-storage-csv`：CSV 透明持久化
  - `leo-plugin-cleaner-kaomoji` / `leo-plugin-parser-novel-regex` / `leo-plugin-dialogue-quotes`：文本清洗与切分
  - `leo-plugin-api`：RESTful HTTP 服务
  - `leo-plugin-mcp`：AI Agent MCP 协议端点
- **双协议共存**：REST API（`/api/v2/*`）与 MCP 智能体端点（`/mcp`）同端口运行。

## 测试

```bash
cargo test
```
