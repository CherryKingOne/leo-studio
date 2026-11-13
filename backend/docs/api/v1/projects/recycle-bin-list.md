# 查询回收站项目列表

分页查询回收站中已被软删除的项目，按删除时间倒序返回。

| 属性 | 值 |
| --- | --- |
| **接口地址** | `GET /api/v1/projects/recycle-bin` |
| **所属模块** | 项目管理（`src/modules/projects/`） |
| **请求格式** | 无请求体 |
| **响应格式** | `application/json` |
| **认证方式** | 无 |
| **幂等性** | 是 |

## 接口描述

返回 `is_deleted = true` 的项目，按 `deleted_at` 倒序排列（缺失时回退 `updated_at`），并分页。

## 请求参数

### Query

| 名称 | 类型 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- | --- |
| `page` | `integer` | 否 | `1` | 页码，必须大于 0 |
| `page_size` | `integer` | 否 | `20` | 每页数量，范围 1~100 |

### Header

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `X-Request-ID` | `string` | 否 | 自定义请求追踪 ID |

## 响应字段

### `data` 对象（`ProjectListResponse`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `total` | `integer` | 总记录数 |
| `page` | `integer` | 当前页码 |
| `page_size` | `integer` | 每页数量 |
| `pages` | `integer` | 总页数 |
| `items` | `object[]` | 回收站项目列表，元素为 `ProjectInfo` |

`ProjectInfo` 字段见[创建项目](./create.md#响应字段)。

## 代码示例

```bash
curl -s "http://127.0.0.1:58685/api/v1/projects/recycle-bin?page=1&page_size=20"
```

## 成功响应示例

```json
{
  "code": 0,
  "message": "success",
  "data": {
    "total": 1,
    "page": 1,
    "page_size": 20,
    "pages": 1,
    "items": [
      {
        "project_id": "9f8c12a4b3e2489c91d8e5f2a1b3c4d5",
        "name": "示例项目",
        "file_id": "a1b2c3d4e5f67890123456789abcdef0",
        "filename": "example.txt",
        "status": "制作中",
        "chapter_count": 12,
        "total_words": 45820,
        "created_at": "2026-10-27T08:55:00+00:00",
        "updated_at": "2026-10-27T09:20:00+00:00",
        "is_deleted": true,
        "deleted_at": "2026-10-27T09:20:00+00:00"
      }
    ]
  },
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```