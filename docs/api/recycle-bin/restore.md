# 恢复回收站项目

| 属性 | 值 |
| --- | --- |
| 接口地址 | `POST /api/v3/recycle-bin/{project_id}/restore` |
| 请求格式 | 无请求体 |
| 响应格式 | `application/json` |
| 认证方式 | 无 |

## 接口描述

将回收站中的项目恢复为正常状态，清除 `deleted_at`，并更新 `updated_at`。

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
| `restored` | `boolean` | 是否已恢复，固定为 `true` |

## curl

```bash
curl -X POST "http://127.0.0.1:58685/api/v3/recycle-bin/9f8c12a4b3e2489c91d8e5f2a1b3c4d5/restore"
```

## 常见错误

| HTTP | 业务码 | 场景 |
| --- | --- | --- |
| 404 | 40400 | 项目不存在 |
| 422 | 40000 | 项目未处于回收站 |
