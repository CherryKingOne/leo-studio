# 恢复项目

将回收站中的项目恢复到正常列表。

| 属性 | 值 |
| --- | --- |
| **接口地址** | `POST /api/v2/projects/{project_id}/restore` |
| **所属模块** | 回收站（`recycle-bin`） |
| **请求格式** | 无请求体 |
| **响应格式** | `application/json` |
| **认证方式** | 无 |
| **幂等性** | 是（重复恢复返回 `422`） |

## 接口描述

将 `is_deleted` 置为 `false` 并清空 `deleted_at`。若项目不存在返回 `404`；
若项目本身不在回收站中，返回 `422`。

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

### `data` 对象（`ProjectRestoreResponse`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `project_id` | `string` | 项目唯一标识 |
| `name` | `string` | 项目名称 |
| `restored` | `boolean` | 固定为 `true` |

## 代码示例

```bash
curl -X POST "http://127.0.0.1:58685/api/v2/projects/9f8c12a4b3e2489c91d8e5f2a1b3c4d5/restore"
```

## 成功响应示例

```json
{
  "code": 0,
  "message": "项目已成功恢复",
  "data": {
    "project_id": "9f8c12a4b3e2489c91d8e5f2a1b3c4d5",
    "name": "示例项目",
    "restored": true
  },
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

## 失败响应示例

### 422 项目不在回收站中

```json
{
  "code": 40000,
  "message": "项目未处于回收站中：9f8c12a4b3e2489c91d8e5f2a1b3c4d5",
  "data": null,
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```