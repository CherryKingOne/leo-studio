# 彻底删除回收站项目

| 属性 | 值 |
| --- | --- |
| 接口地址 | `DELETE /api/v3/recycle-bin/{project_id}` |
| 请求格式 | Path |
| 响应格式 | `application/json` |
| 认证方式 | 无 |

## 接口描述

彻底删除回收站中的项目，并级联删除关联的文本文件、章节、切句与多音字标注。该操作不可恢复。

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
| `hard_deleted` | `boolean` | 是否已彻底删除，固定为 `true` |

## curl

```bash
curl -X DELETE "http://127.0.0.1:58685/api/v3/recycle-bin/9f8c12a4b3e2489c91d8e5f2a1b3c4d5"
```

## 常见错误

| HTTP | 业务码 | 场景 |
| --- | --- | --- |
| 404 | 40400 | 项目不存在 |
| 422 | 40000 | 项目未处于回收站 |
