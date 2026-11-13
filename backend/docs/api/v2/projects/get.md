# 查询项目详情

根据 `project_id` 查询单个未删除项目的详细信息。

| 属性 | 值 |
| --- | --- |
| **接口地址** | `GET /api/v2/projects/{project_id}` |
| **所属模块** | 项目（`projects`） |
| **请求格式** | 无请求体 |
| **响应格式** | `application/json` |
| **认证方式** | 无 |
| **幂等性** | 是 |

## 接口描述

返回项目元信息。若项目不存在或已在回收站中（`is_deleted = true`），返回 `404`。

## 请求参数

### Path

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `project_id` | `string` | **是** | 项目唯一标识 |

### Header

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `X-Request-ID` | `string` | 否 | 自定义请求追踪 ID |

## 响应字段

### `data` 对象（`ProjectInfo`）

字段见[创建项目](./create.md#响应字段)。

## 代码示例

```bash
curl -s "http://127.0.0.1:58685/api/v2/projects/9f8c12a4b3e2489c91d8e5f2a1b3c4d5"
```

## 成功响应示例

```json
{
  "code": 0,
  "message": "success",
  "data": {
    "project_id": "9f8c12a4b3e2489c91d8e5f2a1b3c4d5",
    "name": "示例项目",
    "file_id": "a1b2c3d4e5f67890123456789abcdef0",
    "filename": "example.txt",
    "status": "制作中",
    "chapter_count": 12,
    "total_words": 45820,
    "created_at": "2026-10-27T08:55:00+00:00",
    "updated_at": "2026-10-27T08:55:00+00:00",
    "is_deleted": false,
    "deleted_at": null
  },
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

## 失败响应示例

### 404 项目不存在或已删除

```json
{
  "code": 40400,
  "message": "项目不存在或已移入回收站：9f8c12a4b3e2489c91d8e5f2a1b3c4d5",
  "data": null,
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```