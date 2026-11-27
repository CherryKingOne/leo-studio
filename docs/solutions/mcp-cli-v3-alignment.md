# MCP 与 CLI 的 v3 对齐及角色支持规划

> 状态：已实现
> MCP 工具契约、角色工具、canonical 多音字参数、CLI v3 命令分组、统一 Store 入口、测试与文档均已完成。

## 1. 背景与当前问题

### 1.1 MCP 现状

- 工具定义与 dispatch 全部集中在 `registry.rs`，文件接近 800 行，继续扩展角色工具会进一步膨胀。
- `leo_get_sentence_polyphones`、`leo_save_sentence_polyphones`、`leo_clear_sentence_polyphones` 仍使用旧短路径，只传 `project_id + sentence_id`。
- v3 多音字接口要求 `project_id + chapter_id + sentence_id`，MCP 与 v3 契约不一致。
- 缺少单条多音字删除工具。
- `leo_rename_project` 与 `leo_update_project` 职责重复。
- 输出 Schema 目前是宽松 object，Agent 无法从工具定义准确知道返回字段。
- 缺少角色管理工具。
- 清空多音字标注没有按破坏性工具处理。

### 1.2 CLI 现状

- 命令集中在单个 `cli.rs`，所有业务分支堆在一个 `match` 中。
- `list` 手工按逗号拆分 `projects.csv`，没有复用 CSV Store 和项目 Service。
- 没有项目章节、角色、多音字、回收站命令。
- `--data-dir` 当前实际被当作 CSV 目录使用，与 API 的 `LEO_DATA_DIR` 语义不一致。
- `import/export/voices/synthesize` 走 AST StorageHub，与 v3 的 CSV Store 是两套数据入口，Agent 和用户容易混淆。

## 2. 规划目标

1. MCP 与 CLI 都复用 API 当前使用的项目 Service，不重复实现业务规则。
2. MCP 工具与 v3 路由保持相同的项目作用域和 canonical 参数。
3. MCP 工具名称、输入类型、输出模型、读写属性全部明确。
4. CLI 按功能拆分命令组，支持项目、章节、角色、多音字和回收站。
5. CLI 明确区分 v3 项目管理数据与旧 AST 生产数据。
6. 拆分 MCP/CLI 源文件，避免继续堆叠超大文件。
7. 补齐工具契约测试、CLI 参数解析测试和命令文档。

## 3. 统一架构原则

```text
MCP tools / CLI commands
        ↓
同一个 Project Service / Character Service
        ↓
CSV Store（projects、chapters、sentences、polyphones、characters）
```

- MCP 不通过 HTTP 自调用，直接使用 Service。
- CLI 不手工解析 CSV，直接构造 `Settings + Store` 后调用 Service。
- REST、MCP、CLI 的参数差异只发生在传输层，业务校验必须一致。
- 旧 AST 生产命令继续使用 `StorageHub / ExchangeHub / TtsHub`，不伪装成 v3 项目管理命令。

## 4. MCP 工具分类与命名

### 4.1 类型定义

| 类型 | 含义 | Agent 调用策略 |
| --- | --- | --- |
| `read_only` | 只读查询 | 可自动调用 |
| `write` | 创建或更新数据 | 可调用，但应展示变更结果 |
| `destructive` | 删除、清空、彻底删除 | 必须 `confirm=true`，且服务端开启破坏性工具 |

每个工具必须同时提供：

- `name`
- `title`
- `description`
- `inputSchema`
- `outputSchema`
- `annotations.readOnlyHint`
- `annotations.destructiveHint`
- `annotations.idempotentHint`
- `annotations.openWorldHint=false`

### 4.2 只读工具

| 工具名 | 输入参数 | 输出模型 | 说明 |
| --- | --- | --- | --- |
| `leo_health` | 无 | `HealthInfo` | MCP 服务自身健康状态，不映射 REST |
| `leo_list_projects` | `page:integer`, `page_size:integer`, `status:string?`, `keyword:string?`, `sort:enum?`, `order:enum?` | `ProjectListResponse` | 对应 `GET /api/v3/projects` |
| `leo_get_project` | `project_id:string` | `ProjectInfo` | 对应 `GET /api/v3/projects/{project_id}` |
| `leo_list_chapters` | `project_id:string`, `keyword:string?`, `limit:integer?` | `ChapterOutlineListResponse` | 对应章节列表，`limit` 为 MCP 扩展 |
| `leo_get_chapter` | `project_id:string`, `chapter_id:string`, `include_sentences:boolean?`, `max_chars:integer?` | `ChapterContentInfo + truncated` | 对应章节详情，增加 MCP 截断信息 |
| `leo_search_chapters` | `project_id:string`, `keyword:string`, `limit:integer?` | `ChapterSearchHit[]` | Service-only，当前 REST 未暴露 |
| `leo_list_recycle_bin` | `page:integer?`, `page_size:integer?` | `ProjectListResponse` | 对应 `GET /api/v3/recycle-bin` |
| `leo_list_chapter_polyphones` | `project_id:string`, `chapter_id:string` | `ChapterPolyphoneList` | 对应章节多音字聚合查询 |
| `leo_get_sentence_polyphones` | `project_id:string`, `chapter_id:string`, `sentence_id:string` | `SentencePolyphoneListResponse` | 改为 v3 canonical 参数 |
| `leo_get_stats` | 无 | `ProjectStats` | Service-only，当前 REST 未暴露 |
| `leo_list_characters` | `project_id:string`, `page:integer?`, `page_size:integer?`, `keyword:string?`, `gender:enum?`, `role_type:enum?`, `status:enum?`, `tag:string?`, `sort:enum?`, `order:enum?` | `CharacterListResponse` | 新增，对应角色列表 |
| `leo_get_character` | `project_id:string`, `character_id:string` | `CharacterInfo` | 新增，对应角色详情 |

### 4.3 写入工具

| 工具名 | 输入参数 | 输出模型 | 说明 |
| --- | --- | --- | --- |
| `leo_create_project` | `name:string`, `text:string` | `ProjectInfo` | 对应项目创建 |
| `leo_update_project` | `project_id:string`, `name:string?`, `status:string?` | `ProjectInfo` | 合并原 `leo_rename_project` |
| `leo_append_chapters` | `project_id:string`, `text:string`, `overwrite_existing:boolean?`, `dry_run:boolean?`, `confirm:boolean?` | `ProjectChapterAppendInfo` | 对应章节追加 |
| `leo_update_chapter` | `project_id:string`, `chapter_id:string`, `title:string?`, `content:string?`, `confirm:boolean?` | `ChapterContentInfo` | Service-only；REST v3 尚未提供章节更新接口 |
| `leo_create_character` | `project_id:string`, `name:string`, `aliases:string[]?`, `gender:enum?`, `age:string?`, `role_type:enum?`, `status:enum?`, `description:string?`, `tags:string[]?` | `CharacterInfo` | 新增 |
| `leo_update_character` | `project_id:string`, `character_id:string`, `name:string?`, `aliases:string[]?`, `gender:enum?`, `age:string/null?`, `role_type:enum?`, `status:enum?`, `description:string?`, `tags:string[]?` | `CharacterInfo` | 新增，`age:null` 表示清空 |
| `leo_save_sentence_polyphones` | `project_id:string`, `chapter_id:string`, `sentence_id:string`, `annotations:PolyphoneAnnotation[]` | `SentencePolyphoneListResponse` | 改为 v3 canonical 参数 |
| `leo_restore_project` | `project_id:string` | `ProjectRestoreResponse` | 对应恢复项目 |

### 4.4 破坏性工具

| 工具名 | 输入参数 | 输出模型 | 说明 |
| --- | --- | --- | --- |
| `leo_delete_chapter` | `project_id:string`, `chapter_id:string`, `confirm:boolean` | `ChapterDeleteInfo` | 删除章节 |
| `leo_move_to_trash` | `project_id:string`, `confirm:boolean` | `ProjectDeleteResponse` | 项目软删除 |
| `leo_purge_project` | `project_id:string`, `confirm:boolean` | `ProjectHardDeleteResponse` | 项目彻底删除，级联角色 |
| `leo_empty_recycle_bin` | `confirm:boolean` | `ProjectClearRecycleBinResponse` | 清空回收站，级联角色 |
| `leo_clear_sentence_polyphones` | `project_id:string`, `chapter_id:string`, `sentence_id:string`, `confirm:boolean` | `SentencePolyphoneDeleteResponse` | 清空全部标注 |
| `leo_delete_sentence_polyphone` | `project_id:string`, `chapter_id:string`, `sentence_id:string`, `char_index:integer`, `confirm:boolean` | `SentencePolyphoneItemDeleteInfo` | 新增单条删除 |
| `leo_delete_character` | `project_id:string`, `character_id:string`, `confirm:boolean` | `CharacterDeleteResponse` | 新增角色删除 |

### 4.5 明确移除或合并

- 删除 `leo_rename_project`，统一使用 `leo_update_project`。
- 不再提供省略 `chapter_id` 的多音字工具。
- 不增加音频、声线、合成、导入导出 MCP 工具，保持已删除能力不在 v3 MCP 中复活。
- `leo_health` 保留为 MCP 自身健康检查，不等同于已删除的 REST system 模块。

## 5. MCP Schema 要求

### 5.1 输入 Schema

- 所有对象设置 `additionalProperties=false`。
- ID 类型统一为 `string`。
- 分页、下标、数量统一为 `integer`。
- 布尔值统一为 `boolean`。
- 枚举必须写 `enum`，不能只写在 description 中。
- 可空字段使用 `anyOf` 或明确 `type` 数组，例如 `age: ["string", "null"]`。
- 数组必须定义 `items` 和 `maxItems`。
- 破坏性工具必须把 `confirm` 放入 `required`。

### 5.2 输出 Schema

不再使用统一的宽松 object，而是按工具定义明确结构：

- `ProjectInfo`
- `ProjectListResponse`
- `ChapterOutlineListResponse`
- `ChapterContentInfo`
- `CharacterInfo`
- `CharacterListResponse`
- `SentencePolyphoneListResponse`
- `ChapterPolyphoneList`
- 删除响应对象
- 统计与搜索响应对象

Schema 字段名、类型和可选性与 Rust 返回模型保持一致。

## 6. MCP 资源与提示词

### 6.1 新增资源

- `leo://projects/{project_id}/characters`
- `leo://projects/{project_id}/characters/{character_id}`

### 6.2 提示词调整

- `leo_project_overview` 增加角色数量与角色概况。
- 新增 `leo_character_overview`：汇总项目角色、类型、状态和标签。
- `leo_chapter_digest` 增加“本章出场角色与角色档案差异”提示。
- MCP `instructions` 增加角色管理工具说明。

## 7. CLI 目标命令结构

### 7.1 顶层命令

```text
leo-cli
├── project
│   ├── create
│   ├── list
│   ├── get
│   ├── update
│   └── delete
├── project chapters
│   ├── list
│   ├── append
│   ├── get
│   └── delete
├── project characters
│   ├── create
│   ├── list
│   ├── get
│   ├── update
│   └── delete
├── project polyphones
│   ├── chapter-list
│   ├── sentence-list
│   ├── save
│   ├── clear
│   └── delete-item
├── recycle-bin
│   ├── list
│   ├── clear
│   ├── restore
│   └── purge
├── import
├── export
├── plugins
├── voices
└── synthesize
```

### 7.2 项目命令

| 命令 | 参数 | 对应能力 |
| --- | --- | --- |
| `project create` | `--name`, `--file` | 创建项目并上传小说 |
| `project list` | `--page`, `--page-size`, `--status`, `--keyword`, `--sort`, `--order` | 项目列表 |
| `project get` | `<project_id>` | 项目详情 |
| `project update` | `<project_id>`, `--name?`, `--status?` | 更新项目 |
| `project delete` | `<project_id>`, `--confirm` | 软删除项目 |

### 7.3 章节命令

| 命令 | 参数 | 对应能力 |
| --- | --- | --- |
| `project chapters list` | `<project_id>`, `--keyword?` | 章节大纲 |
| `project chapters append` | `<project_id>`, `--file`, `--overwrite-existing`, `--dry-run`, `--confirm` | 追加章节 |
| `project chapters get` | `<project_id>`, `<chapter_id>` | 章节详情 |
| `project chapters delete` | `<project_id>`, `<chapter_id>`, `--confirm` | 删除章节 |

### 7.4 角色命令

| 命令 | 参数 | 对应能力 |
| --- | --- | --- |
| `project characters create` | `<project_id>`, `--name`, `--alias` 可重复, `--gender?`, `--age?`, `--role-type?`, `--status?`, `--description?`, `--tag` 可重复 | 新建角色 |
| `project characters list` | `<project_id>`, 分页与过滤参数 | 角色列表 |
| `project characters get` | `<project_id>`, `<character_id>` | 角色详情 |
| `project characters update` | `<project_id>`, `<character_id>`, 可选字段 | 更新角色 |
| `project characters delete` | `<project_id>`, `<character_id>`, `--confirm` | 删除角色 |

角色更新支持：

- `--clear-age` 对应 JSON `age:null`
- `--clear-aliases`
- `--clear-tags`
- `--clear-description`

### 7.5 多音字命令

| 命令 | 参数 | 对应能力 |
| --- | --- | --- |
| `project polyphones chapter-list` | `<project_id>`, `<chapter_id>` | 章节多音字聚合 |
| `project polyphones sentence-list` | `<project_id>`, `<chapter_id>`, `<sentence_id>` | 句子标注列表 |
| `project polyphones save` | 完整 ID 链, `--annotations-file <json>` | 全量保存标注 |
| `project polyphones clear` | 完整 ID 链, `--confirm` | 清空标注 |
| `project polyphones delete-item` | 完整 ID 链, `<char_index>`, `--confirm` | 删除单条标注 |

### 7.6 回收站命令

| 命令 | 参数 | 对应能力 |
| --- | --- | --- |
| `recycle-bin list` | 分页参数 | 回收站列表 |
| `recycle-bin clear` | `--confirm` | 清空回收站 |
| `recycle-bin restore` | `<project_id>` | 恢复项目 |
| `recycle-bin purge` | `<project_id>`, `--confirm` | 彻底删除项目 |

## 8. CLI 数据目录语义

建议统一为：

- `--data-dir`：数据根目录，等价于 API 的 `LEO_DATA_DIR`。
- CSV 目录：`<data-dir>/.leo-studio`。
- 上传目录：`<data-dir>/storage/uploads/text-files`。
- 可用 `--csv-dir` 显式覆盖。
- 不再手工读取 `projects.csv`，统一通过 `Store::new(&settings)` 初始化。

### 8.1 v3 管理命令与旧生产命令的关系

| 命令组 | 存储入口 | 说明 |
| --- | --- | --- |
| `project`、`project chapters`、`project characters`、`project polyphones`、`recycle-bin` | v3 CSV Store | 与 REST/MCP 共用同一 Service |
| `import`、`export`、`voices`、`synthesize` | AST StorageHub / ExchangeHub / TTS Hub | 暂保留为旧生产流水线，不混入 v3 管理命令 |

后续如需统一，再单独规划“AST 导入创建 v3 项目”的桥接功能，不在本次强行合并两套存储。

## 9. 目录结构规划

### 9.1 MCP

```text
plugins/interfaces/mcp/src/
├── config.rs
├── lib.rs
├── prompts.rs
├── resources.rs
├── server.rs
├── registry.rs                 # 只保留工具路由与公共定义
└── tools/
    ├── mod.rs
    ├── schema.rs               # JSON Schema 与输出模型 helper
    ├── health.rs
    ├── projects.rs
    ├── chapters.rs
    ├── characters.rs
    ├── polyphones.rs
    └── recycle_bin.rs
```

### 9.2 CLI

```text
plugins/interfaces/cli/src/
├── cli.rs                      # Clap 顶层定义与命令分发
├── lib.rs
├── main.rs
├── runtime.rs                  # Settings + Store 初始化
├── output.rs                   # 人类可读输出与 JSON 输出
└── commands/
    ├── mod.rs
    ├── projects.rs
    ├── chapters.rs
    ├── characters.rs
    ├── polyphones.rs
    ├── recycle_bin.rs
    ├── import.rs
    ├── export.rs
    └── synthesize.rs
```

### 9.3 测试

```text
plugins/interfaces/mcp/tests/
├── tool_contract_test.rs       # 工具名称、Schema、annotations
└── character_tools_test.rs     # 角色工具调用与项目隔离

plugins/interfaces/cli/tests/
├── cli_test.rs                 # Clap 命令解析
└── command_test.rs             # Service 调用与输出
```

### 9.4 文档

```text
docs/
├── mcp/
│   └── tools.md                # MCP 工具完整目录与类型说明
├── cli/
│   └── commands.md             # CLI 命令、参数与示例
├── guides/
│   ├── mcp-agent-guide.md      # 更新为 v3 工具用法
│   └── cli-operations-guide.md # 更新为 v3 命令用法
└── api/                        # 保持现有 v3 REST 文档
```

## 10. 推荐实施顺序

### 阶段一：MCP 契约修正

- 拆分 `registry.rs`。
- 合并项目重命名工具。
- 修正多音字 canonical 参数。
- 增加明确输出 Schema。
- 调整破坏性工具与 `confirm`。

### 阶段二：MCP 角色支持

- 增加 5 个角色工具。
- 增加角色资源模板。
- 更新项目概览与章节摘要提示词。

### 阶段三：CLI v3 管理命令

- 拆分命令模块。
- 统一 `Settings + Store` 初始化。
- 实现项目、章节、角色、多音字、回收站命令。
- 保留旧 AST 生产命令，但明确标注存储边界。

### 阶段四：文档与测试

- 生成 MCP 工具目录和 CLI 命令文档。
- 补齐 Schema 快照测试、角色工具测试和 CLI 解析测试。

## 11. 验收标准

- [x] MCP 不再暴露省略 `chapter_id` 的多音字工具。
- [x] MCP 工具具备明确输入/输出 Schema 和 annotations。
- [x] MCP 支持角色创建、列表、详情、更新、删除。
- [x] 所有 MCP 破坏性工具要求 `confirm=true`。
- [x] CLI 支持项目、章节、角色、多音字、回收站完整命令。
- [x] CLI v3 命令不再手工解析 CSV。
- [x] CLI `--data-dir` 与 API 数据目录语义一致。
- [x] MCP/CLI/REST 使用同一套项目与角色业务规则。
- [x] 工具目录和 CLI 命令文档完整可查。
