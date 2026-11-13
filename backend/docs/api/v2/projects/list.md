# 查询项目列表

分页查询未删除的项目，支持状态过滤、关键字搜索与排序。

| 属性 | 值 |
| --- | --- |
| **接口地址** | `GET /api/v2/projects` |
| **所属模块** | 项目（`projects`） |
| **请求格式** | 无请求体 |
| **响应格式** | `application/json` |
| **认证方式** | 无 |
| **幂等性** | 是 |

## 接口描述

返回 `is_deleted = false` 的项目。默认按 `created_at` 倒序排列。
回收站项目请使用[回收站列表](../recycle-bin/list.md)。

## 请求参数

### Query

| 名称 | 类型 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- | --- |
| `page` | `integer` | 否 | `1` | 页码，必须大于 0 |
| `page_size` | `integer` | 否 | `20` | 每页数量，范围 1~100 |
| `status` | `string` | 否 | — | 按制作状态过滤，如 `制作中` |
| `keyword` | `string` | 否 | — | 按项目名称模糊搜索 |
| `sort` | `string` | 否 | `created_at` | 排序字段：`created_at` / `updated_at` / `name` |
| `order` | `string` | 否 | `desc` | 排序方向：`asc` / `desc` |

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
| `pages` | `integer` | 总页数（无数据为 0） |
| `items` | `object[]` | 项目列表，元素为 `ProjectInfo` |

`ProjectInfo` 字段见[创建项目](./create.md#响应字段)。

## 代码示例

```bash
curl -s "http://127.0.0.1:58685/api/v2/projects?page=1&page_size=20&status=%E5%88%B6%E4%BD%9C%E4%B8%AD&keyword=%E7%A4%BA%E4%BE%8B&sort=updated_at&order=desc"
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
        "updated_at": "2026-10-27T08:55:00+00:00",
        "is_deleted": false,
        "deleted_at": null
      }
    ]
  },
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

## 失败响应示例

### 422 分页参数非法

```json
{
  "code": 40000,
  "message": "每页数量必须在 1 到 100 之间",
  "data": null,
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```