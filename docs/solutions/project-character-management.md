# 项目角色管理功能规划

> 状态：已实现
> 项目角色管理已按本规划完成 v3 接口、CSV 存储、项目级 Service、测试与接口文档。

## 1. 功能定位

角色管理是**指定小说项目内部的角色档案管理**，与项目管理、项目章节管理保持相同的资源归属规则：

```text
项目（小说）
└── 角色
```

一个角色只属于一个项目。不同项目可以存在同名角色，互不影响。

核心原则：

1. 所有角色接口必须携带 `project_id`。
2. 角色 ID 在项目内唯一，服务层必须校验角色是否属于当前项目。
3. 第一期只做角色档案 CRUD，不自动从正文提取角色。
4. 不重新引入已删除的音频、导入导出、系统接口。
5. 数据存储沿用当前 CSV Store 模式，新增独立角色表。

## 2. 第一期范围

### 2.1 本期实现

- 新建项目角色
- 查询项目角色列表
- 查询角色详情
- 更新角色档案
- 删除角色
- 项目内角色名称与别名冲突校验
- 分页、关键字和分类过滤
- 项目软删除、恢复、彻底删除与角色数据的生命周期联动

### 2.2 本期不实现

- 全局角色库
- 角色关系图，例如师徒、亲属、敌对关系
- 角色出场章节分析
- 自动识别人名并生成角色
- 角色头像上传
- 角色声线绑定
- 批量导入角色
- MCP 角色工具

以上能力可以作为后续独立功能规划，不在本次 CRUD 中混入。

## 3. 角色数据模型

### 3.1 对外字段

| 字段 | 类型 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- | --- |
| `character_id` | `string` | 是 | 系统生成 | 角色唯一标识，UUID v4 32 位无连字符格式 |
| `project_id` | `string` | 是 | Path 提供 | 所属项目 ID |
| `name` | `string` | 是 | — | 角色主名称，1~50 个字符 |
| `aliases` | `string[]` | 否 | `[]` | 别名，最多 20 个，每个 1~50 个字符 |
| `gender` | `string` | 否 | `unknown` | `male` / `female` / `unknown` / `other` |
| `age` | `string/null` | 否 | `null` | 年龄描述，例如 `18`、`二十七`、`上千岁`，最多 32 个字符 |
| `role_type` | `string` | 否 | `supporting` | `protagonist` / `supporting` / `antagonist` / `narrator` / `other` |
| `status` | `string` | 否 | `active` | `active` / `archived` |
| `description` | `string` | 否 | `""` | 角色简介，最多 2000 个字符 |
| `tags` | `string[]` | 否 | `[]` | 标签，最多 20 个，每个 1~20 个字符 |
| `created_at` | `string` | 是 | 系统生成 | 创建时间，RFC 3339 UTC |
| `updated_at` | `string` | 是 | 系统生成 | 更新时间，RFC 3339 UTC |

### 3.2 枚举值

#### `gender`

| 值 | 含义 |
| --- | --- |
| `male` | 男性 |
| `female` | 女性 |
| `unknown` | 未知 |
| `other` | 其他 |

#### `role_type`

| 值 | 含义 |
| --- | --- |
| `protagonist` | 主角 |
| `supporting` | 配角 |
| `antagonist` | 反派 |
| `narrator` | 旁白/叙述角色 |
| `other` | 其他 |

#### `status`

| 值 | 含义 |
| --- | --- |
| `active` | 正常使用 |
| `archived` | 已归档，不参与默认列表展示 |

### 3.3 唯一性规则

同一项目内：

- `name` 去除首尾空格并忽略大小写后唯一。
- 任一角色的名称或别名不能与其他角色的名称或别名冲突。
- 同一角色自身的 `aliases` 去重后保存。
- 角色名称不能与自己的别名冲突。

冲突时返回 HTTP `409`，业务码 `40900`。

## 4. API 路由设计

统一前缀：`/api/v3`

| 方法 | 路径 | 说明 |
| --- | --- | --- |
| POST | `/api/v3/projects/{project_id}/characters` | 新建角色 |
| GET | `/api/v3/projects/{project_id}/characters` | 角色列表 |
| GET | `/api/v3/projects/{project_id}/characters/{character_id}` | 角色详情 |
| PATCH | `/api/v3/projects/{project_id}/characters/{character_id}` | 更新角色 |
| DELETE | `/api/v3/projects/{project_id}/characters/{character_id}` | 删除角色 |

不设计以下全局路径：

- `/api/v3/characters`
- `/api/v3/characters/{character_id}`
- `/api/v3/projects/{project_id}/sentences/{sentence_id}/characters`

角色必须通过项目作用域访问。

## 5. 接口详细契约

### 5.1 新建角色

`POST /api/v3/projects/{project_id}/characters`

#### Path

| 参数 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `project_id` | `string` | 是 | 项目 ID |

#### Body

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `name` | `string` | 是 | 角色主名称 |
| `aliases` | `string[]` | 否 | 别名列表 |
| `gender` | `string` | 否 | 性别 |
| `age` | `string/null` | 否 | 年龄描述 |
| `role_type` | `string` | 否 | 角色类型 |
| `status` | `string` | 否 | 状态 |
| `description` | `string` | 否 | 角色简介 |
| `tags` | `string[]` | 否 | 标签列表 |

#### 返回 `data`

返回完整 `CharacterInfo` 对象。

#### 请求示例

```json
{
  "name": "林冲",
  "aliases": ["豹子头", "林教头"],
  "gender": "male",
  "age": "三十出头",
  "role_type": "protagonist",
  "status": "active",
  "description": "原东京八十万禁军枪棒教头，后上梁山。",
  "tags": ["武将", "梁山", "枪法"]
}
```

### 5.2 查询角色列表

`GET /api/v3/projects/{project_id}/characters`

#### Query

| 参数 | 类型 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- | --- |
| `page` | `integer` | 否 | `1` | 页码，必须大于 0 |
| `page_size` | `integer` | 否 | `20` | 每页数量，1~100 |
| `keyword` | `string` | 否 | — | 匹配名称、别名或简介 |
| `gender` | `string` | 否 | — | 按性别过滤 |
| `role_type` | `string` | 否 | — | 按角色类型过滤 |
| `status` | `string` | 否 | `active` | 按状态过滤；传 `all` 返回全部 |
| `tag` | `string` | 否 | — | 按单个标签过滤 |
| `sort` | `string` | 否 | `created_at` | `created_at` / `updated_at` / `name` |
| `order` | `string` | 否 | `desc` | `asc` / `desc` |

#### 返回 `data`

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `project_id` | `string` | 项目 ID |
| `total` | `integer` | 过滤后的角色总数 |
| `page` | `integer` | 当前页 |
| `page_size` | `integer` | 每页数量 |
| `pages` | `integer` | 总页数 |
| `items` | `object[]` | 角色列表，元素为 `CharacterInfo` |

### 5.3 查询角色详情

`GET /api/v3/projects/{project_id}/characters/{character_id}`

#### Path

| 参数 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `project_id` | `string` | 是 | 项目 ID |
| `character_id` | `string` | 是 | 角色 ID |

#### 返回 `data`

返回完整 `CharacterInfo` 对象。

### 5.4 更新角色

`PATCH /api/v3/projects/{project_id}/characters/{character_id}`

#### Body

所有字段可选，但至少提供一个：

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `name` | `string` | 新角色名称 |
| `aliases` | `string[]` | 新别名列表，传入 `[]` 可清空 |
| `gender` | `string` | 新性别 |
| `age` | `string/null` | 新年龄描述，`null` 可清空 |
| `role_type` | `string` | 新角色类型 |
| `status` | `string` | 新状态 |
| `description` | `string` | 新简介，传入 `""` 可清空 |
| `tags` | `string[]` | 新标签列表，传入 `[]` 可清空 |

未提供的字段保持不变。更新成功后刷新 `updated_at`。

#### 返回 `data`

返回更新后的完整 `CharacterInfo` 对象。

### 5.5 删除角色

`DELETE /api/v3/projects/{project_id}/characters/{character_id}`

#### 返回 `data`

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `project_id` | `string` | 项目 ID |
| `character_id` | `string` | 被删除角色 ID |
| `name` | `string` | 被删除角色名称 |
| `deleted` | `boolean` | 固定为 `true` |

删除为物理删除，不进角色回收站。项目回收站只管理项目本身。

## 6. 存储设计

### 6.1 新增 CSV 表

新增文件：

```text
{LEO_CSV_DIR}/novel_characters.csv
```

建议表头：

```text
character_id,project_id,name,aliases_json,gender,age,role_type,status,description,tags_json,created_at,updated_at
```

字段说明：

| 列 | 说明 |
| --- | --- |
| `character_id` | 角色 ID |
| `project_id` | 所属项目 ID |
| `name` | 主名称 |
| `aliases_json` | 别名数组的 JSON 字符串 |
| `gender` | 性别枚举 |
| `age` | 年龄描述，可空 |
| `role_type` | 角色类型枚举 |
| `status` | 角色状态 |
| `description` | 角色简介 |
| `tags_json` | 标签数组的 JSON 字符串 |
| `created_at` | 创建时间 |
| `updated_at` | 更新时间 |

`aliases_json` 与 `tags_json` 在 Service 层完成 JSON 序列化/反序列化，对外 API 始终使用数组。

### 6.2 Store 调整

实施时需要在现有 `Store` 中增加：

- `characters: CsvTable`
- `save_characters()`
- `save_all()` 中加入角色表保存
- 新配置项 `novel_characters_csv_filename`
- 测试构造函数同步传入角色 CSV 文件名

### 6.3 项目生命周期联动

| 项目操作 | 角色数据行为 |
| --- | --- |
| 软删除项目 | 保留角色数据 |
| 恢复项目 | 角色数据保持原样 |
| 彻底删除项目 | 级联删除该项目所有角色 |
| 清空回收站 | 级联删除所有被彻底删除项目的角色 |
| 删除章节 | 不删除角色 |
| 更新章节 | 不删除角色 |

## 7. 目录结构规划

### 7.1 后端代码

```text
plugins/interfaces/api/src/
├── modules/
│   ├── projects/
│   │   ├── characters/
│   │   │   ├── mod.rs          # 角色子模块导出
│   │   │   ├── model.rs        # CharacterInfo 等领域模型
│   │   │   ├── schema.rs       # 请求/响应 DTO
│   │   │   └── service.rs      # 角色 CRUD、过滤、冲突校验
│   │   └── ...
│   └── v3/
│       ├── chapters.rs
│       ├── characters.rs      # 项目角色路由与 HTTP 处理器
│       ├── polyphones.rs
│       ├── projects.rs
│       └── recycle_bin.rs
├── config.rs                   # 增加 novel_characters_csv_filename
├── storage/
│   ├── store.rs                # 增加 characters 表与保存方法
│   └── ...
└── ...
```

### 7.2 测试

```text
app/tests/
└── v3_characters.rs
```

测试至少覆盖：

- 创建、列表、详情、更新、删除
- 项目隔离
- 角色不属于指定项目时返回 404
- 名称与别名冲突返回 409
- 分页与过滤
- 项目彻底删除时角色级联删除
- 项目恢复后角色仍存在
- 旧全局角色路径返回 404

### 7.3 接口文档

```text
docs/api/
├── project-characters/
│   ├── create.md
│   ├── list.md
│   ├── get.md
│   ├── update.md
│   └── delete.md
└── README.md
```

正式文档仍遵循“一个接口一个 Markdown”，包含请求参数、返回字段和 curl 示例。

## 8. 错误设计

| HTTP | 业务码 | 场景 |
| --- | --- | --- |
| 404 | 40400 | 项目不存在、角色不存在或角色不属于该项目 |
| 409 | 40900 | 角色名称或别名冲突 |
| 422 | 40000 | 参数为空、枚举非法、数组超限、分页非法 |
| 500 | 50000 | CSV 存储读写失败 |
| 500 | 50001 | 服务内部错误 |

## 9. 安全与一致性要求

- 所有查询、更新、删除都必须同时使用 `project_id` 与 `character_id` 定位。
- 不允许先按 `character_id` 全局查询后再绕过项目校验。
- 名称与别名冲突校验必须在同一项目内进行。
- 写入操作只锁定一次 `Store`，避免检查与写入之间产生并发窗口。
- 角色删除前确认角色属于当前项目。
- 项目彻底删除时先删除角色，再删除项目，保证不会留下孤儿角色数据。

## 10. 推荐实施顺序

1. 增加 `novel_characters.csv`、Store 字段和配置项。
2. 实现角色 model、schema 和 service。
3. 实现 `modules/v3/characters.rs` 的 5 个接口。
4. 在 v3 路由中注册角色模块。
5. 补充项目彻底删除时的角色级联清理。
6. 编写 `v3_characters.rs` 集成测试。
7. 生成 5 份正式接口文档并更新总览。

## 11. 验收标准

- [ ] 角色只能通过 `/api/v3/projects/{project_id}/characters` 访问。
- [ ] 不同项目可以创建同名角色。
- [ ] 同一项目内名称与别名冲突会被拒绝。
- [ ] 角色查询、更新、删除不会跨项目。
- [ ] 项目软删除与恢复不丢失角色。
- [ ] 项目彻底删除会级联删除角色。
- [ ] 5 个接口均有独立 Markdown 文档。
- [ ] 旧全局角色路径返回 404。
