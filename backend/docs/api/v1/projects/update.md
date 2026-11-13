# 更新项目名称

重命名指定项目。

| 属性 | 值 |
| --- | --- |
| **接口地址** | `PATCH /api/v1/projects/{project_id}`（别名：`PUT /api/v1/projects/{project_id}`） |
| **所属模块** | 项目管理（`src/modules/projects/`） |
| **请求格式** | `application/json` |
| **响应格式** | `application/json` |
| **认证方式** | 无 |
| **幂等性** | 是 |

## 接口描述

修改项目名称并刷新 `updated_at`。名称去除首尾空白后不能为空，长度不能超过 255 个字符。
若项目不存在或在回收站中，返回 `404`。

## 请求参数

### Path

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `project_id` | `string` | **是** | 项目唯一标识 |

### Header

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `Content-Type` | `string` | 是 | `application/json` |
| `X-Request-ID` | `string` | 否 | 自定义请求追踪 ID |

### Body（`ProjectUpdateRequest`）

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `name` | `string` | **是** | 新的项目名称 |

## 响应字段

### `data` 对象（`ProjectInfo`）

字段见[创建项目](./create.md#响应字段)。

## 代码示例

```bash
curl -X PATCH "http://127.0.0.1:58685/api/v1/projects/9f8c12a4b3e2489c91d8e5f2a1b3c4d5" \
  -H "Content-Type: application/json" \
  -d '{"name":"新的项目名称"}'
```

## 成功响应示例

```json
{
  "code": 0,
  "message": "项目更新成功",
  "data": {
    "project_id": "9f8c12a4b3e2489c91d8e5f2a1b3c4d5",
    "name": "新的项目名称",
    "file_id": "a1b2c3d4e5f67890123456789abcdef0",
    "filename": "example.txt",
    "status": "制作中",
    "chapter_count": 12,
    "total_words": 45820,
    "created_at": "2026-10-27T08:55:00+00:00",
    "updated_at": "2026-10-27T09:10:00+00:00",
    "is_deleted": false,
    "deleted_at": null
  },
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

## 失败响应示例

### 422 名称为空

```json
{
  "code": 40000,
  "message": "项目名称不能为空",
  "data": null,
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

### 404 项目不存在

```json
{
  "code": 40400,
  "message": "项目不存在：9f8c12a4b3e2489c91d8e5f2a1b3c4d5",
  "data": null,
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```