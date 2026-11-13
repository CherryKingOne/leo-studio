# 删除项目（移入回收站）

软删除项目，将项目移入回收站，保留小说与切句数据以便恢复。

| 属性 | 值 |
| --- | --- |
| **接口地址** | `DELETE /api/v1/projects/{project_id}` |
| **所属模块** | 项目管理（`src/modules/projects/`） |
| **请求格式** | 无请求体 |
| **响应格式** | `application/json` |
| **认证方式** | 无 |
| **幂等性** | 是（重复删除返回 `404`） |

## 接口描述

将项目标记为 `is_deleted = true` 并记录 `deleted_at`，不删除关联的小说文件、
章节与切句数据。项目随后不再出现在[项目列表](./list.md)与详情接口中，
但可通过[恢复接口](./restore.md)还原。

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

### `data` 对象（`ProjectDeleteResponse`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `project_id` | `string` | 项目唯一标识 |
| `name` | `string` | 项目名称 |
| `deleted` | `boolean` | 固定为 `true` |
| `deleted_at` | `string \| null` | 删除时间（RFC 3339 UTC） |

## 代码示例

```bash
curl -X DELETE "http://127.0.0.1:58685/api/v1/projects/9f8c12a4b3e2489c91d8e5f2a1b3c4d5"
```

## 成功响应示例

```json
{
  "code": 0,
  "message": "项目已成功移入回收站",
  "data": {
    "project_id": "9f8c12a4b3e2489c91d8e5f2a1b3c4d5",
    "name": "示例项目",
    "deleted": true,
    "deleted_at": "2026-10-27T09:20:00+00:00"
  },
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

## 失败响应示例

### 404 项目不存在

```json
{
  "code": 40400,
  "message": "项目不存在：9f8c12a4b3e2489c91d8e5f2a1b3c4d5",
  "data": null,
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```