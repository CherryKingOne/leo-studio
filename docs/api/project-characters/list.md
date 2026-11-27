# 查询项目角色列表

| 属性 | 值 |
| --- | --- |
| 接口地址 | `GET /api/v3/projects/{project_id}/characters` |
| 请求格式 | Path + Query |
| 响应格式 | `application/json` |
| 认证方式 | 无 |

## 请求参数

### Path

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `project_id` | `string` | 是 | 项目唯一标识 |

### Query

| 名称 | 类型 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- | --- |
| `page` | `integer` | 否 | `1` | 页码，必须大于 0 |
| `page_size` | `integer` | 否 | `20` | 每页数量，1~100 |
| `keyword` | `string` | 否 | — | 匹配名称、别名或简介 |
| `gender` | `string` | 否 | — | `male` / `female` / `unknown` / `other` |
| `role_type` | `string` | 否 | — | 角色类型过滤 |
| `status` | `string` | 否 | `active` | `active` / `archived` / `all` |
| `tag` | `string` | 否 | — | 按单个标签过滤 |
| `sort` | `string` | 否 | `created_at` | `created_at` / `updated_at` / `name` |
| `order` | `string` | 否 | `desc` | `asc` / `desc` |

## 返回字段

### `data`（`CharacterListResponse`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `project_id` | `string` | 项目 ID |
| `total` | `integer` | 过滤后的角色总数 |
| `page` | `integer` | 当前页码 |
| `page_size` | `integer` | 每页数量 |
| `pages` | `integer` | 总页数 |
| `items` | `object[]` | 角色列表 |

### `items[]`（`CharacterInfo`）

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
curl -sG "http://127.0.0.1:58685/api/v3/projects/9f8c12a4b3e2489c91d8e5f2a1b3c4d5/characters" \
  --data-urlencode "page=1" \
  --data-urlencode "page_size=20" \
  --data-urlencode "status=all" \
  --data-urlencode "sort=name" \
  --data-urlencode "order=asc"
```

## 常见错误

| HTTP | 业务码 | 场景 |
| --- | --- | --- |
| 404 | 40400 | 项目不存在或已移入回收站 |
| 422 | 40000 | 分页或过滤枚举非法 |
