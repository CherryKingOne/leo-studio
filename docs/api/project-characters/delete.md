# 删除项目角色

| 属性 | 值 |
| --- | --- |
| 接口地址 | `DELETE /api/v3/projects/{project_id}/characters/{character_id}` |
| 请求格式 | Path |
| 响应格式 | `application/json` |
| 认证方式 | 无 |

## 请求参数

### Path

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `project_id` | `string` | 是 | 项目唯一标识 |
| `character_id` | `string` | 是 | 角色唯一标识 |

## 返回字段

### `data`

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `project_id` | `string` | 项目 ID |
| `character_id` | `string` | 被删除角色 ID |
| `name` | `string` | 被删除角色名称 |
| `deleted` | `boolean` | 固定为 `true` |

删除为物理删除，不进角色回收站。

## curl

```bash
curl -X DELETE "http://127.0.0.1:58685/api/v3/projects/9f8c12a4b3e2489c91d8e5f2a1b3c4d5/characters/c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8"
```

## 常见错误

| HTTP | 业务码 | 场景 |
| --- | --- | --- |
| 404 | 40400 | 项目或角色不存在，或角色不属于该项目 |
