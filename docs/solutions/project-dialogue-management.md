# 项目台词管理与说话人绑定规划

> 状态：已实现
> 本文档记录项目台词管理、说话人绑定、MCP 工具、CLI 命令和目录结构的最终设计与实现结果。

## 1. 功能定位

项目台词管理用于管理小说章节中已经切分出来的台词、旁白和音效片段，并在片段上维护说话人绑定。

资源关系：

```text
项目 projects
└── 章节 chapters
    └── 台词片段 dialogues
        └── 说话人绑定 speaker_id → characters
```

- `characters` 管理“小说中有哪些角色”。
- `dialogues` 管理“章节中有哪些台词片段”。
- `speaker_id` 表示“这句台词由哪个角色说”。
- 旁白可以绑定到 `role_type=narrator` 的角色，也可以保持未绑定。

本功能不新增“说话人管理”顶层模块，说话人就是项目角色。

## 2. 当前现状

### 2.1 已有能力

- 章节正文已经按行切句。
- 句子已经按中文双引号拆为 `narration` 和 `dialogue` 片段。
- API CSV 中已有 `novel_dialogues.csv`。
- AST 的 `SegmentNode` 已有 `speaker_id` 字段。

### 2.2 当前缺口

- API 的 `DIALOGUE_HEADERS` 没有 `speaker_id`。
- 没有台词片段列表和详情接口。
- 没有角色与台词的绑定、解绑接口。
- MCP 和 CLI 没有台词管理命令。
- 角色删除后没有明确的台词解绑规则。
- 章节正文重建后没有明确的台词绑定失效规则。

## 3. 本期范围

### 3.1 本期实现

- 查询章节台词片段列表
- 查询单个台词片段详情
- 绑定说话人角色
- 解绑说话人
- 未绑定台词过滤
- MCP 台词工具
- CLI 台词命令
- 角色删除时自动解绑
- 章节重建时清除旧片段绑定

### 3.2 本期不实现

- 自动识别人名并推荐角色
- 自动判断对话归属
- 角色声线配置
- TTS 合成
- 台词正文编辑
- 台词删除
- 情绪、停顿、多音字标注

自动匹配可以作为下一期功能，本期先完成手动绑定闭环。

## 4. 数据模型

### 4.1 现有 `novel_dialogues.csv` 字段

```text
dialogue_id
novel_id
chapter_id
parent_sentence_id
source_sentence_index
split_index
segment_type
content
word_count
parse_status
```

### 4.2 新增字段

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `speaker_id` | `string` | 否 | 说话人角色 ID；空字符串表示未绑定 |
| `binding_source` | `string` | 否 | `manual` / `auto` / `imported` / `system` |
| `binding_confidence` | `number` | 否 | 自动匹配置信度，范围 `0.0~1.0` |
| `speaker_updated_at` | `string` | 否 | 最近一次绑定或解绑时间，RFC 3339 UTC |

`speaker_name` 不落库，查询时通过 `speaker_id` 关联角色表返回。

### 4.3 对外 `DialogueInfo`

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `project_id` | `string` | 所属项目 ID |
| `chapter_id` | `string` | 所属章节 ID |
| `dialogue_id` | `string` | 台词片段 ID |
| `parent_sentence_id` | `string` | 来源句子 ID |
| `sentence_index` | `integer` | 来源句子序号 |
| `segment_index` | `integer` | 片段在句子内的序号 |
| `segment_type` | `string` | `narration` / `dialogue` |
| `content` | `string` | 片段正文 |
| `word_count` | `integer` | 片段字数 |
| `parse_status` | `string` | `ok` / `unpaired_quote` / `empty` |
| `speaker_id` | `string/null` | 说话人角色 ID |
| `speaker_name` | `string/null` | 说话人名称 |
| `binding_source` | `string/null` | 绑定来源 |
| `binding_confidence` | `number/null` | 自动匹配置信度 |
| `speaker_updated_at` | `string/null` | 绑定更新时间 |

`binding_status` 是计算字段：

- `speaker_id` 为空：`unbound`
- `speaker_id` 非空：`bound`

## 5. API 路由设计

统一前缀：`/api/v3`

| 方法 | 路径 | 说明 |
| --- | --- | --- |
| GET | `/api/v3/projects/{project_id}/chapters/{chapter_id}/dialogues` | 台词片段列表 |
| GET | `/api/v3/projects/{project_id}/chapters/{chapter_id}/dialogues/{dialogue_id}` | 台词片段详情 |
| PUT | `/api/v3/projects/{project_id}/chapters/{chapter_id}/dialogues/{dialogue_id}/speaker` | 绑定或修改说话人 |
| DELETE | `/api/v3/projects/{project_id}/chapters/{chapter_id}/dialogues/{dialogue_id}/speaker` | 解绑说话人 |

不设计全局 `/api/v3/dialogues` 路由。

## 6. 接口详细契约

### 6.1 查询台词片段列表

`GET /api/v3/projects/{project_id}/chapters/{chapter_id}/dialogues`

#### Path

| 参数 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `project_id` | `string` | 是 | 项目 ID |
| `chapter_id` | `string` | 是 | 章节 ID |

#### Query

| 参数 | 类型 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- | --- |
| `page` | `integer` | 否 | `1` | 页码 |
| `page_size` | `integer` | 否 | `20` | 每页数量，1~100 |
| `segment_type` | `string` | 否 | — | `narration` / `dialogue` |
| `binding_status` | `string` | 否 | `all` | `all` / `bound` / `unbound` |
| `speaker_id` | `string` | 否 | — | 按说话人过滤 |
| `keyword` | `string` | 否 | — | 按片段正文过滤 |

结果固定按 `sentence_index`、`segment_index` 升序排列。

#### 返回 `data`

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `project_id` | `string` | 项目 ID |
| `chapter_id` | `string` | 章节 ID |
| `total` | `integer` | 过滤后的片段总数 |
| `page` | `integer` | 当前页 |
| `page_size` | `integer` | 每页数量 |
| `pages` | `integer` | 总页数 |
| `items` | `object[]` | `DialogueInfo` 列表 |

### 6.2 查询台词片段详情

`GET /api/v3/projects/{project_id}/chapters/{chapter_id}/dialogues/{dialogue_id}`

#### Path

| 参数 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `project_id` | `string` | 是 | 项目 ID |
| `chapter_id` | `string` | 是 | 章节 ID |
| `dialogue_id` | `string` | 是 | 片段 ID |

#### 返回 `data`

返回完整 `DialogueInfo`。

### 6.3 绑定或修改说话人

`PUT /api/v3/projects/{project_id}/chapters/{chapter_id}/dialogues/{dialogue_id}/speaker`

#### Body

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `speaker_id` | `string` | 是 | 要绑定的角色 ID；解绑请使用 DELETE 接口 |
| `binding_source` | `string` | 否 | 默认 `manual` |
| `binding_confidence` | `number/null` | 否 | 自动匹配时使用，范围 `0.0~1.0` |

手动接口建议只允许 `manual` 或 `imported`；`auto` 留给后续自动匹配服务。

#### 返回 `data`

返回绑定后的完整 `DialogueInfo`。

### 6.4 解绑说话人

`DELETE /api/v3/projects/{project_id}/chapters/{chapter_id}/dialogues/{dialogue_id}/speaker`

#### 返回 `data`

```json
{
  "dialogue_id": "xxx",
  "unbound": true
}
```

重复解绑也返回成功，保持幂等。前端如需批量绑定或全选解绑，遍历调用单条接口即可，不提供批量接口。

## 7. 业务规则

### 7.1 归属校验

所有接口必须校验：

- 项目存在且未删除。
- 章节属于当前项目。
- 台词片段属于当前章节。
- 说话人角色属于当前项目。

### 7.2 旁白规则

- `segment_type=narration` 允许绑定角色。
- 推荐绑定到 `role_type=narrator` 的角色。
- 也允许保持未绑定。
- 不强制要求旁白必须有说话人。

### 7.3 角色删除

删除角色时：

- 不删除任何台词片段。
- 将所有绑定到该角色的片段 `speaker_id` 清空。
- `binding_source` 置为 `system`。
- 刷新 `speaker_updated_at`。

### 7.4 章节正文重建

以下操作会重建台词片段：

- 更新章节正文
- 覆盖同名章节
- 删除章节

旧片段会被删除，新片段不继承旧 `speaker_id`，避免角色绑定错位。

### 7.5 项目删除

- 项目软删除：台词和绑定保留。
- 项目恢复：绑定继续有效。
- 项目彻底删除：台词片段随项目数据级联删除。

## 8. MCP 工具设计

### 8.1 只读工具

| 工具名 | 输入参数 | 输出 |
| --- | --- | --- |
| `leo_list_dialogues` | `project_id`, `chapter_id`, `segment_type?`, `binding_status?`, `speaker_id?`, `keyword?`, `page?`, `page_size?` | `DialogueListResponse` |
| `leo_get_dialogue` | `project_id`, `chapter_id`, `dialogue_id` | `DialogueInfo` |
| `leo_list_unbound_dialogues` | `project_id`, `chapter_id`, `segment_type?`, `page?`, `page_size?` | `DialogueListResponse` |

### 8.2 写入工具

| 工具名 | 输入参数 | 输出 |
| --- | --- | --- |
| `leo_bind_dialogue_speaker` | `project_id`, `chapter_id`, `dialogue_id`, `speaker_id`, `binding_source?`, `binding_confidence?` | `DialogueInfo` |
| `leo_unbind_dialogue_speaker` | `project_id`, `chapter_id`, `dialogue_id` | `UnbindDialogueSpeakerResponse` |

解绑属于可逆写操作，不标记为 destructive，不要求 `confirm=true`。

### 8.3 MCP Schema 要求

- 所有工具提供明确 `inputSchema` 和 `outputSchema`。
- `binding_status` 使用枚举 `all/bound/unbound`。
- `segment_type` 使用枚举 `narration/dialogue`。
- `binding_source` 使用枚举 `manual/auto/imported/system`。
- `binding_confidence` 使用 `number`，范围 `0.0~1.0`。

### 8.4 MCP 资源

新增：

- `leo://projects/{project_id}/chapters/{chapter_id}/dialogues`
- `leo://projects/{project_id}/chapters/{chapter_id}/dialogues/{dialogue_id}`

### 8.5 MCP 提示词

新增：

- `leo_dialogue_binding_review`：汇总未绑定台词、已绑定角色和可疑旁白，辅助 Agent 逐条确认。

## 9. CLI 命令设计

```text
leo-cli project dialogues
├── list
├── get
├── bind
└── unbind
```

### 9.1 列表

```bash
leo-cli project dialogues list <project_id> <chapter_id> \
  --binding-status unbound \
  --segment-type dialogue \
  --page 1 \
  --page-size 20
```

### 9.2 详情

```bash
leo-cli project dialogues get <project_id> <chapter_id> <dialogue_id>
```

### 9.3 绑定说话人

```bash
leo-cli project dialogues bind <project_id> <chapter_id> <dialogue_id> \
  --speaker-id <character_id>
```

### 9.4 解绑说话人

```bash
leo-cli project dialogues unbind <project_id> <chapter_id> <dialogue_id>
```

所有 v3 命令继续使用统一 `Settings + Store`，不手工解析 CSV。

## 10. 目录结构规划

### 10.1 API 代码

```text
plugins/interfaces/api/src/
├── modules/
│   ├── projects/
│   │   ├── dialogues/
│   │   │   ├── mod.rs          # 台词子模块导出
│   │   │   ├── model.rs        # DialogueInfo
│   │   │   ├── schema.rs       # 列表、绑定、解绑 DTO
│   │   │   └── service.rs      # 查询、绑定、解绑
│   │   └── ...
│   └── v3/
│       ├── chapters.rs
│       ├── characters.rs
│       ├── dialogues.rs        # 项目台词路由与 HTTP 处理器
│       ├── polyphones.rs
│       ├── projects.rs
│       └── recycle_bin.rs
├── storage/
│   └── store.rs                # DIALOGUE_HEADERS 增加说话人绑定字段
└── ...
```

### 10.2 MCP

```text
plugins/interfaces/mcp/src/
├── registry.rs
├── resources.rs
├── prompts.rs
└── tools/
    ├── chapters.rs
    ├── characters.rs
    ├── dialogues.rs            # 台词工具定义
    ├── mod.rs
    ├── polyphones.rs
    ├── projects.rs
    ├── recycle_bin.rs
    └── schema.rs
```

### 10.3 CLI

```text
plugins/interfaces/cli/src/
├── cli.rs                      # 增加 ProjectCommands::Dialogues
├── commands/
│   ├── chapters.rs
│   ├── characters.rs
│   ├── dialogues.rs            # 台词查询、绑定、解绑
│   ├── mod.rs
│   ├── polyphones.rs
│   ├── projects.rs
│   └── recycle_bin.rs
├── runtime.rs
└── output.rs
```

### 10.4 测试

```text
app/tests/
└── v3_dialogues.rs

plugins/interfaces/mcp/tests/
└── dialogue_tools_test.rs

plugins/interfaces/cli/tests/
└── dialogue_command_test.rs
```

### 10.5 文档

```text
docs/
├── api/
│   └── project-dialogues/
│       ├── list.md
│       ├── get.md
│       ├── bind-speaker.md
│       └── unbind-speaker.md
├── mcp/
│   └── tools.md                # 增加台词工具
├── cli/
│   └── commands.md              # 增加 project dialogues 命令
└── solutions/
    └── project-dialogue-management.md
```

## 11. 错误设计

| HTTP | 业务码 | 场景 |
| --- | --- | --- |
| 404 | 40400 | 项目、章节、台词或角色不存在 |
| 409 | 40900 | 台词或角色不属于当前项目/章节 |
| 422 | 40000 | 分页、枚举或置信度非法 |
| 500 | 50000 | CSV 存储失败 |
| 500 | 50001 | 服务内部错误 |

MCP 工具错误继续返回 `code + message`，CLI 返回非零退出码。

## 12. 测试范围

### API

- 台词列表与详情
- `narration/dialogue` 过滤
- `bound/unbound/all` 过滤
- 说话人过滤与关键字过滤
- 绑定、改绑、解绑、重复解绑
- 跨项目角色绑定拒绝
- 角色删除后自动解绑
- 章节正文重建后绑定清空
- 项目软删除、恢复、彻底删除联动

### MCP

- 工具 Schema 与 annotations
- 角色工具和台词工具共存
- 未绑定台词查询

### CLI

- `project dialogues` 命令解析
- 列表、绑定、解绑
- `--data-dir` 与 CSV Store 一致性

## 13. 推荐实施顺序

1. 扩展 `novel_dialogues.csv` 表头。
2. 实现 `DialogueInfo` 与台词查询 Service。
3. 实现单条绑定与解绑 Service。
4. 实现 v3 台词路由。
5. 接入角色删除自动解绑。
6. 接入章节正文重建绑定清空。
7. 增加 MCP 台词工具、资源和提示词。
8. 增加 CLI `project dialogues` 命令。
9. 生成 4 份接口文档并更新工具/命令手册。
10. 跑全工作区测试与 Clippy。

## 14. 验收标准

- [x] 台词只能在项目和章节作用域内访问。
- [x] 台词列表支持绑定状态、片段类型、说话人和关键字过滤。
- [x] 说话人只能是当前项目的角色。
- [x] 绑定、改绑、解绑全部可用。
- [x] 角色删除后不会留下失效说话人引用。
- [x] 章节重建后不会继承错位的旧绑定。
- [x] MCP 提供完整台词工具与显式 Schema。
- [x] CLI 提供完整 `project dialogues` 命令组。
- [x] 每个接口都有独立 Markdown 文档。
