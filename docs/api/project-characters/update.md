# 更新项目角色

| 属性 | 值 |
| --- | --- |
| 接口地址 | `PATCH /api/v3/projects/{project_id}/characters/{character_id}` |
| 请求格式 | `application/json` |
| 响应格式 | `application/json` |
| 认证方式 | 无 |

## 请求参数

### Path

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `project_id` | `string` | 是 | 项目唯一标识 |
| `character_id` | `string` | 是 | 角色唯一标识 |

### Body

所有字段可选，至少提供一个：

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `name` | `string` | 新角色名称 |
| `aliases` | `string[]` | 新别名列表，`[]` 可清空 |
| `gender` | `string` | 新性别 |
| `age` | `string/null` | 新年龄描述，`null` 可清空 |
| `role_type` | `string` | 新角色类型 |
| `status` | `string` | 新状态 |
| `description` | `string` | 新简介，`""` 可清空 |
| `tags` | `string[]` | 新标签列表，`[]` 可清空 |

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
| `created_at` | `string` | 创建时间 |
| `updated_at` | `string` | 更新时间 |

## curl

```bash
curl -X PATCH "http://127.0.0.1:58685/api/v3/projects/9f8c12a4b3e2489c91d8e5f2a1b3c4d5/characters/c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8" \
  -H "Content-Type: application/json" \
  -d '{"status":"archived","age":null,"tags":[]}'
```

## 常见错误

| HTTP | 业务码 | 场景 |
| --- | --- | --- |
| 404 | 40400 | 项目或角色不存在，或角色不属于该项目 |
| 409 | 40900 | 角色名称或别名冲突 |
| 422 | 40000 | 未提供更新字段、字段超限或枚举非法 |
