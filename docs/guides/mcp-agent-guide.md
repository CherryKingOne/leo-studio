# 🤖 MCP 智能体驱动实操指南 (Model Context Protocol)

**Model Context Protocol (MCP)** 是一种开放协议，它让大语言模型（LLM）与外部工具和数据源能够以标准化、类型安全的方式直接交互。

Leo Studio 内置了工业级的 MCP 服务插件（`leo-plugin-mcp`），通过标准的 **Streamable HTTP** 协议承载，为 AI Agent 提供了完整的有声剧编排与制作能力。

---

## 1. 启动 MCP 服务

MCP 服务内置于 Leo Studio 宿主应用中，无需单独运行其他进程。

在项目根目录下启动应用：
```bash
cargo run -p leo-app
```

终端将打印：
```text
INFO leo_app: 挂载 MCP HTTP 服务端点: /mcp
INFO leo_app: Leo Studio 后端微内核已启动：http://127.0.0.1:58685
```

此时，你的 MCP 访问端点即为：**`http://127.0.0.1:58685/mcp`**。

---

## 2. 客户端配置

将 Leo Studio MCP 端点添加到你常用的 AI 客户端中。

### 2.1 Claude Desktop 配置

在 Claude Desktop 配置文件中（macOS 位于 `~/Library/Application Support/Claude/claude_desktop_config.json`，Windows 位于 `%APPDATA%\Claude\claude_desktop_config.json`）：

```json
{
  "mcpServers": {
    "leo-studio": {
      "url": "http://127.0.0.1:58685/mcp"
    }
  }
}
```

### 2.2 Cursor 配置

在项目根目录或全局设置中的 `.cursor/mcp.json`：

```json
{
  "mcpServers": {
    "leo-studio": {
      "url": "http://127.0.0.1:58685/mcp"
    }
  }
}
```

### 2.3 Antigravity / Gemini CLI 配置

在 `~/.gemini/antigravity/mcp_config.json` 或项目配置中：

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

---

## 3. 核心工具一览 (Tools Reference)

一旦连接成功，AI Agent 会自动感知到 Leo Studio 暴露的 **20+ 个生产工具**：

### 3.1 剧目管理与检索

| 工具名称 | 权限 | 功能描述 | 关键参数 |
| :--- | :---: | :--- | :--- |
| `leo_health` | 只读 | 检查服务可用性、版本及 MCP 协议兼容性 | 无 |
| `leo_list_projects` | 只读 | 分页查询剧目列表，支持按关键字与状态过滤 | `page`, `page_size`, `keyword`, `status` |
| `leo_get_project` | 只读 | 获取剧目详情（名称、文件信息、章节数、总字数） | `project_id` |
| `leo_list_chapters` | 只读 | 获取指定剧目的大纲章节列表及字数统计 | `project_id`, `keyword`, `limit` |
| `leo_get_chapter` | 只读 | 获取单章正文详情，支持逐句切分与多音字标注 | `project_id`, `chapter_id`, `include_sentences` |
| `leo_search_chapters` | 只读 | 跨章节全文关键字搜索，返回命中摘要 | `project_id`, `keyword`, `limit` |

### 3.2 制作与文本处理

| 工具名称 | 权限 | 功能描述 | 关键参数 |
| :--- | :---: | :--- | :--- |
| `leo_create_project` | 写入 | 创建全新剧目并录入初始章节正文 | `name`, `content`, `chapter_title` |
| `leo_append_chapters` | 写入 | 为已有剧目快速追加后续章节 | `project_id`, `chapters` 数组 |
| `leo_update_chapter` | 写入 | 更新指定章节的标题或正文内容 | `project_id`, `chapter_id`, `title`, `content` |
| `leo_delete_chapter` | 破坏 | 删除剧目下的指定章节 | `project_id`, `chapter_id` |

### 3.3 语音合成与插件能力

| 工具名称 | 权限 | 功能描述 | 关键参数 |
| :--- | :---: | :--- | :--- |
| `leo_list_plugins` | 只读 | 查询当前微内核装载的所有插件及能力声明 | 无 |
| `leo_list_voices` | 只读 | 查询系统可用 TTS 声线角色列表（Edge/CosyVoice）| 无 |
| `leo_synthesize_chapter` | 写入 | 指挥微内核对指定章节逐句执行批量语音合成（属于写入工具组，`LEO_MCP_WRITE_TOOLS=false` 时不可用） | `project_id`, `chapter_id`, `voice`, `speed`, `pitch`, `force` |

> `leo_list_voices` 属于只读工具组，即使关闭写入/破坏性工具也始终可用。

---

## 4. 实战对话范例：Agent 全流程自动化制作

当你把 Leo Studio 的 MCP 配置给大模型后，你可以直接使用自然语言给 Agent 下达指令：

### 对话示例：
> **用户**：
> “你好，请帮我检查一下 Leo Studio 系统里目前有哪些剧目？另外查询一下当前有哪些可用的男声和女声配音员。”
>
> **Agent 调用工具**：
> 1. `leo_list_projects({})`
> 2. `leo_list_voices({})`
>
> **Agent 回复**：
> “当前系统共有 1 个剧目：《开局合欢宗，被师姐拿捏命脉》（ID: `proj_001`，共 10 章）。
> 当前系统可用的声线包括：
> - `zh-CN-YunxiNeural`（云希，阳光青年男声，适合玄幻男主）
> - `zh-CN-XiaoxiaoNeural`（晓晓，清丽活泼女声，适合女主）
> - `zh-CN-YunjianNeural`（云健，沉稳旁白男声，适合宏大叙事）”
>
> ---
>
> **用户**：
> “请查看第一章的内容，分析其中有几句女主的对白。然后使用云希作为旁白与男主，一键将第一章制作成有声书语音！”
>
> **Agent 调用工具**：
> 1. `leo_get_chapter({"project_id": "proj_001", "chapter_id": "chap_001", "include_sentences": true})`
> 2. （Agent 分析句中台词，提取人物情感）
> 3. `leo_synthesize_chapter({"project_id": "proj_001", "chapter_id": "chap_001", "voice": "zh-CN-YunxiNeural", "speed": 1.0, "pitch": 0.0})`
>
> **Agent 回复**：
> “第一章已成功合成完毕！共生成 28 个高质量音频切片，总耗时约 4 秒。您可以通过系统音频接口直接试听或下载最终成品。”

---

## 5. 权限与安全控制

如需在生产环境中限制 AI Agent 的破坏性操作，可通过 `.env` 环境变量调整：

```bash
# 是否开启破坏性工具（如删除章节 leo_delete_chapter），设为 false 将完全隐藏
LEO_MCP_DESTRUCTIVE_TOOLS=false

# 是否开启写工具（如创建或更新章节），设为 false 将使 Agent 仅具备只读审阅权限
LEO_MCP_WRITE_TOOLS=true
```
