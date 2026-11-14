# Leo Studio MCP Server 建设规划

> 状态：**规划中，尚未编码**
> 目标：为 Leo Studio 增加 MCP（Model Context Protocol）能力，让 Claude Desktop、Cursor、VS Code、Cherry Studio 等 MCP 客户端可以直接查询与操作剧目、章节、切句与多音字标注。
> 本文只定义范围、协议策略、工具契约与目录结构，不含实现代码。

---

## 1. 背景与目标

### 1.1 现状

仓库已经是「Rust + Axum 后端 + Next.js 前端」的分离结构，业务侧已经具备：

- 剧目（项目）CRUD、软删除、回收站、恢复、物理删除
- 章节大纲、章节正文、逐句切分
- 章节追加上传（`dry_run` 预检 + 防污染覆盖）
- 句子级多音字标注（全量覆盖保存、清空）
- 统一响应包装 `{ code, message, data, request_id }`

但**只有 HTTP/JSON 一个出口**：浏览器和前端服务端能调，AI 客户端无法直接使用。MCP 的价值是把这些能力以「工具 + 资源 + 提示词」的形式标准化暴露，让模型在对话里直接完成「看一下我的剧目 → 打开某章 → 标出多音字 → 追更第 101 章」这类工作。

### 1.2 目标

| 编号 | 目标 | 验收口径 |
| --- | --- | --- |
| G1 | 只支持**当前最新** MCP 规范 `2026-07-28`（无状态核心） | 官方 conformance 场景通过；`server/discover`、无 `_meta` 拒绝、`Mcp-Method` 校验生效 |
| G2 | 旧协议（`2025-11-25` 及更早）明确拒绝 | 旧客户端收到 `-32022` 与 `supported=["2026-07-28"]`，不会误连 |
| G3 | 复用现有 `projects` / `text_files` 业务服务，**不复制业务规则** | MCP 工具最终都落到 `projects::service` / `text_files::service` |
| G4 | 只读能力默认可用，写能力受配置开关与工具注解约束 | 默认配置下模型只能查询，不能改数据 |
| G5 | 提供 Streamable HTTP 入口 | `POST /mcp` 可完成工具、资源与提示词调用 |
| G6 | 破坏性操作有二次确认与审计日志 | 清空回收站、物理删除、覆盖章节必须带 `confirm` |

### 1.3 非目标（本期不做）

- 不做 MCP **客户端**（不代理别的 MCP Server）。
- 不做鉴权（无用户体系，MCP 与 REST 同域暴露）。
- 不实现 Sampling / Roots / Logging（`2026-07-28` 已弃用）。
- 不实现 Tasks 扩展、MCP Apps 扩展。
- 不做音频合成、母带导出等尚未落地的业务能力。

---

## 2. 协议版本：到底要支持哪几版

### 2.1 先澄清一个说法

MCP **没有「v2」这种大版本号**。它用日期作为协议版本：

| 版本 | 形态 | 关键变化 |
| --- | --- | --- |
| `2024-11-05` | 有状态，HTTP+SSE | 初版，已弃用 |
| `2025-03-26` | 有状态，Streamable HTTP | 引入 OAuth、工具注解 |
| `2025-06-18` | 有状态 | 去掉 JSON-RPC 批量、结构化输出、Elicitation |
| `2025-11-25` | 有状态 | 图标、URL 模式 Elicitation、Tasks（实验） |
| **`2026-07-28`** | **无状态（当前最新）** | 去掉 `initialize` 与会话、强制 `server/discover`、MRTR、`subscriptions/listen`、结果带 `resultType` |

本期**只支持 `2026-07-28`**。旧协议（`2025-11-25` 及更早）直接拒绝，不做兼容层：

- HTTP：不支持的版本返回 `400` + `-32022`，`data.supported = ["2026-07-28"]`；
- 所有请求只接受 `2026-07-28`；旧客户端发 `initialize` 时返回 `-32022`；
- HTTP+SSE（`2024-11-05`）与旧版会话头一律不支持。

### 2.2 方法矩阵

图例：✅ 已实现；❌ 不支持。

| 方法 | `2026-07-28` | 本期 |
| --- | --- | --- |
| `server/discover` | ✅ 必须 | ✅ |
| `initialize` / `notifications/initialized` | ➖（已移除） | ❌ |
| `ping` | ➖（已移除） | ❌ |
| `tools/list` | ✅ | ✅ |
| `tools/call` | ✅ | ✅ |
| `resources/list` | ✅ | ✅ |
| `resources/templates/list` | ✅ | ✅ |
| `resources/read` | ✅ | ✅ |
| `prompts/list` | ✅ | ✅ |
| `prompts/get` | ✅ | ✅ |
| `subscriptions/listen` | ✅ | ❌（本期不做变更订阅） |
| `notifications/*` | 🟡 | ❌ |
| `completion/complete` | 🟡 | ❌ |
| `elicitation/create` | 🟡 | ❌（用工具参数 `confirm`） |
| `sampling/createMessage` / `roots/list` | 已弃用 | ❌ |
| Tasks 扩展 / MCP Apps | 扩展 | ❌ |
| HTTP GET SSE 流 | ➖ | 405 |
| HTTP+SSE（`2024-11-05`） | 已弃用 | ❌ |

HTTP 侧还要满足 `2026-07-28` 的请求元数据要求：

- `MCP-Protocol-Version` 头必带，且与 `_meta["io.modelcontextprotocol/protocolVersion"]` 一致，否则 `400` + `-32020 HeaderMismatch`；
- `Mcp-Method` 必带，`tools/call` / `resources/read` / `prompts/get` 额外带 `Mcp-Name`；
- `_meta` 里的 `protocolVersion`、`clientInfo`、`clientCapabilities` 必读；
- 不支持的版本返回 `400` + `-32022 UnsupportedProtocolVersion`，并在 `data.supported` 里列出支持的版本；
- `Origin` 校验失败返回 `403`；本机部署只绑 `127.0.0.1`。

### 2.4 技术选型

| 选项 | 结论 | 理由 |
| --- | --- | --- |
| `rmcp` 3.x（官方 Rust SDK） | **采用** | 同时覆盖两代协议、MRTR、无状态 HTTP、订阅、缓存提示；与现有 Axum/Tokio 栈一致 |
| 自己手写 JSON-RPC 分发 | 不采用 | 版本差异多，容易在 `resultType`、头部校验、订阅流上踩坑 |
| 独立 Node/Python MCP 服务 | 不采用 | 会变成第二个业务入口，还要再实现一遍 projects 服务 |

> 注意：`rmcp` 3.x 的 MSRV 是 **Rust 1.88**。仓库当前用 edition 2024（≥1.85），实施前需确认 CI 与本地工具链版本。

---

## 3. 定位：MCP 是第三个入口，不是新业务

```
                    ┌──────────────────────────────┐
                    │  projects / text_files 服务层  │  ← 唯一的业务规则
                    └──────────────┬───────────────┘
                                   │
        ┌──────────────────────────┼──────────────────────────┐
        │                          │                          │
┌───────▼────────┐        ┌────────▼────────┐        ┌────────▼────────┐
│ REST /api/v1   │        │ REST /api/v2    │        │ MCP /mcp│  ← 新增
│ 前端 / 脚本     │        │ 结构化客户端     │        │ AI 客户端        │
└────────────────┘        └─────────────────┘        └─────────────────┘
```

MCP 层只做四件事：

1. 把工具参数映射成 `service` 调用；
2. 把领域模型映射成 MCP 的 `content` + `structuredContent`；
3. 把 `AppError` 映射成「工具执行错误」（`isError: true`，让模型自我纠正）或协议错误；
4. 开关与审计。

**禁止**在 MCP 层写章节切分、防污染比对、多音字校验等规则——这些必须留在 `service`。

---

## 4. 对外暴露的工具清单

### 4.1 命名与通用约定

- 统一前缀 `leo_`，全小写 + 下划线，长度 ≤ 128，符合规范建议的字符集。
- 每个工具都有 `title`、`description`、`inputSchema`、`outputSchema`、`annotations`。
- 返回同时给：
  - `content: [{ type: "text", text: "<人类可读中文摘要>" }]`（兼容不支持结构化输出的客户端）；
  - `structuredContent: { ... }`（与 `outputSchema` 一致）。
- 文本字段默认截断到 **20,000 字符**，返回 `truncated` 与 `returned_chars`，避免一次性把整章塞进上下文。
- 分页游标统一用 `cursor`（不透明 Base64），默认 `page_size = 20`，上限 100。

### 4.2 只读工具（默认开启）

| 工具 | 入参 | 返回 | 复用服务 | 危险级别 |
| --- | --- | --- | --- | --- |
| `leo_health` | — | `status / app / version / supported_versions` | `system::health` | 无 |
| `leo_list_projects` | `page, page_size, status, keyword, sort, order` | `items[] / total / page / page_size / pages` | `projects::service::query_projects` | 无 |
| `leo_get_project` | `project_id` | `ProjectInfo` | `projects::service::get_project` | 无 |
| `leo_list_chapters` | `project_id, keyword?, limit?` | 章节大纲数组（含 `word_count`） | `get_project_chapters` | 无 |
| `leo_get_chapter` | `project_id, chapter_id, include_sentences=true, max_chars=20000` | 章节正文 + 逐句 + 多音字 | `get_project_chapter_content` | 无 |
| `leo_search_chapters` | `project_id, keyword, limit=50` | 命中章节（标题/正文片段、字数） | **新增** `search_chapters`（service 层） | 无 |
| `leo_list_recycle_bin` | `page, page_size` | 回收站剧目 + `deleted_at` | `list_projects(in_recycle_bin=true)` | 无 |
| `leo_get_sentence_polyphones` | `project_id, sentence_id` | 标注数组 | `get_sentence_polyphones` | 无 |
| `leo_list_chapter_polyphones` | `project_id, chapter_id` | 章节全部标注（按句分组） | `get_chapter_polyphones` | 无 |
| `leo_get_stats` | — | 剧目数 / 章节数 / 总字数 / 回收站数 | 由 `store` 聚合（新增只读函数） | 无 |

### 4.3 写入工具（`LEO_MCP_WRITE_TOOLS=true` 才注册）

| 工具 | 入参 | 返回 | 复用服务 | 危险级别 |
| --- | --- | --- | --- | --- |
| `leo_create_project` | `name, text` | `ProjectInfo` | `create_project` | 新增数据 |
| `leo_append_chapters` | `project_id, text, overwrite_existing=false, dry_run=true, confirm=false` | `ProjectChapterAppendInfo` | `append_chapters` | 覆盖时危险 |
| `leo_rename_project` | `project_id, name` | `ProjectInfo` | `update_project` | 低 |
| `leo_update_project` | `project_id, name?, status?` | `ProjectInfo` | `update_project_fields`（v2 服务） | 低 |
| `leo_save_sentence_polyphones` | `project_id, sentence_id, annotations[]` | 保存后的标注数组 | `save_sentence_polyphones` | 覆盖整句标注 |
| `leo_clear_sentence_polyphones` | `project_id, sentence_id` | `deleted_count` | `delete_sentence_polyphones` | 低（可重标） |
| `leo_restore_project` | `project_id` | `ProjectInfo` | `restore_project` | 低 |

### 4.4 破坏性工具（额外要求 `LEO_MCP_DESTRUCTIVE_TOOLS=true` + `confirm=true`）

| 工具 | 入参 | 返回 | 复用服务 | 说明 |
| --- | --- | --- | --- | --- |
| `leo_delete_chapter` | `project_id, chapter_id, confirm` | 删除结果 + 新统计 | `delete_chapter`（v2 服务） | 级联删切句与标注 |
| `leo_move_to_trash` | `project_id, confirm` | 软删除结果 | `delete_project` | 可恢复 |
| `leo_purge_project` | `project_id, confirm` | 硬删除结果 | `hard_delete_project` | **不可逆** |
| `leo_empty_recycle_bin` | `confirm` | `cleared_count` | `clear_recycle_bin` | **不可逆** |

### 4.5 工具注解（annotations）

| 工具类别 | `readOnlyHint` | `destructiveHint` | `idempotentHint` | `openWorldHint` |
| --- | --- | --- | --- | --- |
| 只读 | `true` | `false` | `true` | `false` |
| 创建 / 追加 | `false` | `false` | `false` | `false` |
| 重命名 / 标注 | `false` | `false` | `true` | `false` |
| 删除章节 / 移入回收站 | `false` | `true` | `true` | `false` |
| 物理删除 / 清空 | `false` | `true` | `true` | `false` |

> 注解只是给客户端的提示，客户端必须把它当不可信输入；真正的保护在工具执行前的服务端校验。

### 4.6 正文入参

导入与追更只接受 `text`（UTF-8 正文）。不提供 `file_path`：MCP 客户端可能在远端，服务端路径没有意义，且会引入路径穿越风险。

---

## 5. 资源（Resources）

| URI | 内容 | MIME |
| --- | --- | --- |
| `leo://projects` | 剧目列表（前 100 条） | `application/json` |
| `leo://projects/{project_id}` | 剧目详情 | `application/json` |
| `leo://projects/{project_id}/chapters` | 章节大纲 | `application/json` |
| `leo://projects/{project_id}/chapters/{chapter_id}` | 章节详情（含逐句） | `application/json` |
| `leo://projects/{project_id}/chapters/{chapter_id}/text` | 清洗后的章节正文 | `text/plain; charset=utf-8` |
| `leo://stats` | 全库统计 | `application/json` |

- 同时实现 `resources/templates/list`，让客户端能自动发现参数化 URI。
- 资源默认只读；`notifications/resources/updated` 在写操作成功后通过 `subscriptions/listen` 流推送。
- 大数据资源同样截断，并在 `annotations` 里标 `audience`。

---

## 6. 提示词（Prompts）

| 名称 | 参数 | 作用 |
| --- | --- | --- |
| `leo_project_overview` | `project_id?` | 汇总剧目数量、章节进度、回收站状态，给出下一步建议 |
| `leo_import_novel` | `name, text?` | 引导模型：创建剧目 → 拉章节 → 报告分章结果 |
| `leo_chapter_digest` | `project_id, chapter_id` | 拉取章节正文与逐句，要求输出结构化摘要（人物/事件/待确认词） |
| `leo_polyphone_review` | `project_id, chapter_id` | 找出多音字候选，逐句确认后调用标注工具 |
| `leo_update_check` | `project_id, text` | 先 `dry_run` 追更预检，解释冲突，再询问是否覆盖 |

提示词只返回 `messages`，不直接改数据；真正的写入仍由模型调用工具完成。

---

## 7. 传输与安全

### 7.1 传输

| 传输 | 入口 | 适用 | 备注 |
| --- | --- | --- | --- |
| **Streamable HTTP** | `POST /mcp`（复用现有 Axum 服务） | 团队共享、远程客户端、网关 | 无会话、无 `Mcp-Session-Id`；不提供 stdio 进程 |

### 7.2 安全基线

- 默认绑定 `127.0.0.1`；对外暴露必须显式改 `LEO_MCP_HOST`。
- `Origin` 校验：缺失或不在白名单 → `403`。
- 不做 Token 鉴权；暴露范围由 `LEO_HOST` 决定。
- 写工具与破坏性工具默认关闭，分别由两个开关控制。
- 所有写操作记审计日志：`request_id / clientInfo / tool / 参数摘要 / 结果 / 耗时`，只记元数据，不记正文全文。
- 速率限制：单客户端 `60 req/min`，超限返回 `429`。
- 错误信息不外泄内部路径与 SQL/CSV 细节；`AppError` 的 `details` 不直接透传给模型，只保留可操作的说明。

### 7.3 与现有 `request_id` 中间件的关系

现有 `middleware.rs` 会把 `request_id` 注入所有 JSON 响应。MCP 的 JSON-RPC 响应**不能**被二次改写（会破坏 schema），因此：

- `/mcp` 路由在挂载 MCP 路由时**跳过**该中间件；
- MCP 自己生成 `request_id` 并写入审计日志与 `_meta`（若客户端提供 `X-Request-ID` 则复用）。

---

## 8. 目录结构

### 8.1 现状（相关部分）

```text
backend/
├── Cargo.toml
├── src/
│   ├── app.rs              # 路由装配（v1 / v2）
│   ├── config.rs           # 环境变量
│   ├── error.rs
│   ├── lib.rs
│   ├── main.rs
│   ├── middleware.rs       # request_id
│   ├── modules/
│   │   ├── projects/       # router / service / schema / model
│   │   ├── system.rs
│   │   ├── text_files/
│   │   └── v2/
│   ├── storage/
│   └── text/
├── docs/api/{v1,v2}/
└── tests/{api.rs, v2.rs, parsers.rs}
```

### 8.2 新增后的结构（方案 A：并入现有 backend crate）

```text
backend/
├── Cargo.toml                     # 增加 rmcp；仅 Streamable HTTP 相关 feature
├── src/
│   ├── mcp/                       # ★ 新增模块
│   │   ├── mod.rs                 # 导出与 Axum /mcp 挂载
│   │   ├── config.rs              # LEO_MCP_* 配置与校验
│   │   ├── server.rs              # ServerHandler：discover / tools / resources / prompts
│   │   ├── registry.rs            # 21 个工具定义、参数校验与分发
│   │   ├── resources.rs           # leo:// 资源与模板
│   │   └── prompts.rs             # 三个工作流提示词
│   ├── app.rs                     # 挂载 /mcp；request_id 中间件只作用于 REST
│   └── modules/                   # 现有业务模块（只增不改）
├── docs/mcp/
│   ├── README.md                  # 总览、启动方式、配置、安全
│   └── tools.md                   # 工具契约（入参/出参/权限/示例）
└── tests/mcp.rs                   # 工具开关、CRUD、追更、多音字、破坏性确认
```

### 8.3 根目录与前端的变化

```text
.
├── README.md                      # 增加「MCP 能力」一节
├── docs/solutions/
│   └── mcp-server-plan.md         # 本文
├── backend/STRUCTURE.md           # 增加 mcp/ 与 docs/mcp/ 说明
└── desktop/
    ├── app/settings/mcp/page.tsx  # 可选：生成客户端配置片段（不内置密钥）
    └── lib/mcp-config.ts          # 可选：按操作系统拼配置 JSON
```

> 说明：`desktop/` 的改动是**可选**的便利功能，不属于 MCP Server 本体；不实现也不影响 MCP 可用性。

### 8.4 方案 B（备选）：独立 crate

```text
mcp-server/
├── Cargo.toml          # depends on leo-studio-backend (path)
├── src/main.rs
└── src/lib.rs
```

| 维度 | 方案 A（并入 backend） | 方案 B（独立 crate） |
| --- | --- | --- |
| 部署 | 单进程，一个端口 | 可独立部署/扩缩容 |
| 复用 service | 直接调用 | 通过 lib 调用 |
| 依赖 | 一份依赖树 | 两份，编译更慢 |
| 风险 | `rmcp` 与 Axum 版本耦合 | 需要稳定 lib 边界 |
| 结论 | **本期采用** | 等需要独立发布时再拆 |

---

## 9. 配置项

| 环境变量 | 默认值 | 说明 |
| --- | --- | --- |
| `LEO_MCP_ENABLED` | `true` | 是否挂载 `/mcp`（默认开启） |
| `LEO_MCP_HTTP_PATH` | `/mcp` | Streamable HTTP 端点 |
| `LEO_MCP_WRITE_TOOLS` | `false` | 注册写入工具 |
| `LEO_MCP_DESTRUCTIVE_TOOLS` | `false` | 注册破坏性工具 |

监听地址沿用 `LEO_HOST` / `LEO_PORT`；文本上限与分页上限固定为 20,000 字符和 100 条，不做成配置项。

配置解析放在 `mcp/config.rs`，与现有 `config.rs` 风格保持一致（环境变量 → 结构体 → 校验），启动时校验失败直接拒绝启动并给出明确原因。

---

## 10. 实施阶段

| 阶段 | 内容 | 产出 | 验收 |
| --- | --- | --- | --- |
| P0 | 规划与契约评审（本文） | 本文档 + 工具清单定稿 | 评审通过 |
| P1 | 骨架与发现 | `mcp/` 模块、HTTP 入口、`server/discover`、`leo_health` | `server/discover` 返回 `2026-07-28`；旧协议返回 `-32022` |
| P2 | 只读工具 + 资源 | 4.2 全部工具、第 5 节资源 | 契约测试覆盖每个工具的正常/404/参数错误 |
| P3 | HTTP 传输 | `/mcp`、头部校验、Origin | `-32020` / `-32022` / `403` 用例通过 |
| P4 | 写入与破坏性工具 | 4.3 / 4.4 工具、审计、`confirm` | 默认关闭时 `tools/list` 不出现；开启后危险操作留痕 |
| P5 | 提示词与订阅 | 第 6 节提示词、`subscriptions/listen` | 变更通知能推到订阅流 |
| P6 | 文档与联调 | `docs/mcp/*`、Claude/Cursor 实测 | 至少两个真实客户端跑通端到端 |

预估工作量（不含联调等待）：P1 2~3 人日，P2 3~4 人日，P3 2~3 人日，P4 3~4 人日，P5 2 人日，P6 2 人日。

---

## 11. 风险与对策

| 风险 | 影响 | 对策 |
| --- | --- | --- |
| `rmcp` MSRV 1.88 高于当前工具链 | 无法编译 | 先升级 CI/本地 Rust；或锁定 `rmcp` 2.x（但 2.x 不支持 2026-07-28） |
| 无状态 HTTP + 内存 CSV | 多实例部署时各自持有全量数据，写冲突 | 单实例部署；如需多实例，先补文件锁/SQLite，不把 MCP 当多活入口 |
| 工具返回整章正文 | 上下文爆炸、成本飙升 | `max_chars` 截断 + 分页 + 资源按需读取 |
| 写工具被模型误用 | 数据被覆盖/删除 | 双层开关 + `confirm` + `dry_run` 默认 true + 审计 |
| MCP 响应被 `request_id` 中间件改写 | 协议不合规 | `/mcp` 路由单独挂载，不套用该中间件 |
| 客户端版本碎片化 | 行为差异 | 只维护 `2026-07-28` 一套契约；旧客户端需升级 |
| 业务服务没有 `search_chapters` / `get_stats` | 需要补 service | 在 `projects::service` 增加只读函数，MCP 只调用不实现 |

---

## 12. 决策记录与待确认

已决策：

1. **协议**：只支持 `2026-07-28`，不做旧版兼容。
2. **传输**：只提供 Streamable HTTP（`POST /mcp`），不做 stdio。
3. **写工具**：默认关闭；`leo_append_chapters` 默认 `dry_run=true`，真正写入需 `confirm=true`。
4. **正文入参**：只接受 `text`，不读取服务器本地文件。

待确认：

1. **访问控制**：当前无鉴权；若需要公网暴露，再补 Token 或网关鉴权。
2. **订阅与变更通知**：是否补 `subscriptions/listen`（P5）？
3. **桌面端配置页**：是否要做「一键复制 Claude Desktop / Cursor 配置」的设置页？

---

## 13. 附：与现有 REST 接口的映射

| MCP 工具 | 对应 REST | 对应 service |
| --- | --- | --- |
| `leo_list_projects` | `GET /api/v2/projects` | `query_projects` |
| `leo_get_project` | `GET /api/v1/projects/{id}` | `get_project` |
| `leo_list_chapters` | `GET /api/v1/projects/{id}/chapters` | `get_project_chapters` |
| `leo_get_chapter` | `GET /api/v1/projects/{id}/chapters/{cid}` | `get_project_chapter_content` |
| `leo_append_chapters` | `POST /api/v1/projects/{id}/chapters/upload` | `append_chapters` |
| `leo_delete_chapter` | `DELETE /api/v2/projects/{id}/chapters/{cid}` | `delete_chapter` |
| `leo_get_sentence_polyphones` | `GET .../sentences/{sid}/polyphones` | `get_sentence_polyphones` |
| `leo_save_sentence_polyphones` | `PUT .../sentences/{sid}/polyphones` | `save_sentence_polyphones` |
| `leo_clear_sentence_polyphones` | `DELETE .../sentences/{sid}/polyphones` | `delete_sentence_polyphones` |
| `leo_move_to_trash` | `DELETE /api/v1/projects/{id}` | `delete_project` |
| `leo_restore_project` | `POST /api/v1/projects/{id}/restore` | `restore_project` |
| `leo_purge_project` | `DELETE /api/v1/projects/{id}/hard` | `hard_delete_project` |
| `leo_empty_recycle_bin` | `DELETE /api/v1/projects/recycle-bin` | `clear_recycle_bin` |