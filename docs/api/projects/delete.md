# 删除项目（移入回收站）

| 属性 | 值 |
| --- | --- |
| 接口地址 | `DELETE /api/v3/projects/{project_id}` |
| 请求格式 | Path |
| 响应格式 | `application/json` |
| 认证方式 | 无 |

## 接口描述

软删除项目，将 `is_deleted` 置为 `true` 并记录 `deleted_at`。项目及其章节数据会保留，可通过恢复接口还原。

## 请求参数

### Path

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `project_id` | `string` | 是 | 项目唯一标识 |

## 返回字段

### `data`

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `project_id` | `string` | 项目唯一标识 |
| `name` | `string` | 项目名称 |
| `deleted` | `boolean` | 是否已移入回收站，固定为 `true` |
| `deleted_at` | `string` | 删除时间，RFC 3339 UTC |

## curl

```bash
curl -X DELETE "http://127.0.0.1:58685/api/v3/projects/9f8c12a4b3e2489c91d8e5f2a1b3c4d5"
```

## 常见错误

| HTTP | 业务码 | 场景 |
| --- | --- | --- |
| 404 | 40400 | 项目不存在或已在回收站 |
