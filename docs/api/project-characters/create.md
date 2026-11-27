# 新建项目角色

| 属性 | 值 |
| --- | --- |
| 接口地址 | `POST /api/v3/projects/{project_id}/characters` |
| 请求格式 | `application/json` |
| 响应格式 | `application/json` |
| 认证方式 | 无 |

## 请求参数

### Path

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `project_id` | `string` | 是 | 项目唯一标识 |

### Body

| 字段 | 类型 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- | --- |
| `name` | `string` | 是 | — | 角色主名称，1~50 个字符 |
| `aliases` | `string[]` | 否 | `[]` | 别名，最多 20 个，每个 1~50 个字符 |
| `gender` | `string` | 否 | `unknown` | `male` / `female` / `unknown` / `other` |
| `age` | `string/null` | 否 | `null` | 年龄描述，最多 32 个字符 |
| `role_type` | `string` | 否 | `supporting` | `protagonist` / `supporting` / `antagonist` / `narrator` / `other` |
| `status` | `string` | 否 | `active` | `active` / `archived` |
| `description` | `string` | 否 | `""` | 角色简介，最多 2000 个字符 |
| `tags` | `string[]` | 否 | `[]` | 标签，最多 20 个，每个 1~20 个字符 |

同一项目内角色名称与别名不能冲突。

## 返回字段

### `data`（`CharacterInfo`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `character_id` | `string` | 角色唯一标识 |
| `project_id` | `string` | 所属项目 ID |
| `name` | `string` | 角色主名称 |
| `aliases` | `string[]` | 别名列表 |
| `gender` | `string` | 性别 |
| `age` | `string/null` | 年龄描述 |
| `role_type` | `string` | 角色类型 |
| `status` | `string` | 角色状态 |
| `description` | `string` | 角色简介 |
| `tags` | `string[]` | 标签列表 |
| `created_at` | `string` | 创建时间，RFC 3339 UTC |
| `updated_at` | `string` | 更新时间，RFC 3339 UTC |

## curl

```bash
curl -X POST "http://127.0.0.1:58685/api/v3/projects/9f8c12a4b3e2489c91d8e5f2a1b3c4d5/characters" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "林冲",
    "aliases": ["豹子头", "林教头"],
    "gender": "male",
    "age": "三十出头",
    "role_type": "protagonist",
    "description": "原东京八十万禁军枪棒教头。",
    "tags": ["武将", "梁山"]
  }'
```

## 常见错误

| HTTP | 业务码 | 场景 |
| --- | --- | --- |
| 404 | 40400 | 项目不存在或已移入回收站 |
| 409 | 40900 | 项目内角色名称或别名冲突 |
| 422 | 40000 | 名称为空、字段超限或枚举非法 |
