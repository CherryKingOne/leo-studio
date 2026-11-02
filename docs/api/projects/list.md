# 查询项目列表

分页查询剧目/项目列表，按创建时间倒序排列。

| 属性 | 值 |
| --- | --- |
| **接口地址** | `GET /api/v1/projects` |
| **所属模块** | 剧目与项目管理（`app/modules/projects/`） |
| **请求格式** | 无请求体 |
| **响应格式** | `application/json` |
| **认证方式** | 无（当前版本未接入鉴权） |
| **幂等性** | 是（只读操作） |

## 接口描述

返回当前系统已创建的所有活跃有声书剧目列表，支持按页码和每页数量分页。结果默认按**创建时间倒序**（最新创建的项目排在最前）。

> [!NOTE]
> 该接口自动过滤掉已移入回收站的项目（`is_deleted == false`）。如需查询回收站中的项目，请访问 [查询回收站项目列表](./recycle-bin.md)。

## 请求参数

### Query

| 名称 | 类型 | 必填 | 默认值 | 约束 | 说明 |
| --- | --- | --- | --- | --- | --- |
| `page` | `integer` | 否 | `1` | `>= 1` | 页码，从 1 开始 |
| `page_size` | `integer` | 否 | `20` | `1 <= n <= 100` | 每页数量，最大 100 |

### Header

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `X-Request-ID` | `string` | 否 | 自定义请求追踪 ID，未传则由服务端生成 |

## 响应字段

### `data` 对象（`ProjectListResponse`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `total` | `integer` | 项目总数 |
| `page` | `integer` | 当前页码 |
| `page_size` | `integer` | 每页数量 |
| `pages` | `integer` | 总页数 |
| `items` | `ProjectListItem[]` | 当前页项目列表 |

### `items[]` 元素（`ProjectListItem`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `project_id` | `string` | 项目唯一标识，标准 UUID（UUID4，非雪花 ID） |
| `name` | `string` | 项目名称 |
| `file_id` | `string` | 关联文本文件唯一标识，标准 UUID（UUID4，非雪花 ID） |
| `filename` | `string` | 原始文件名 |
| `status` | `string` | 制作状态 |
| `chapter_count` | `integer` | 章节总数 |
| `total_words` | `integer` | 有效总字数 |
| `created_at` | `string` | 创建时间，ISO 8601 格式 |
| `updated_at` | `string` | 更新时间，ISO 8601 格式 |
| `is_deleted` | `boolean` | 是否处于回收站（软删除状态），活跃列表中恒为 `false` |
| `deleted_at` | `string \| null` | 软删除时间戳，活跃项目中为 `null` |

## 成功示例

### 请求

```bash
curl -X GET "http://127.0.0.1:8000/api/v1/projects?page=1&page_size=10"
```

### 响应

```json
{
  "code": 0,
  "message": "success",
  "data": {
    "total": 1,
    "page": 1,
    "page_size": 10,
    "pages": 1,
    "items": [
      {
        "project_id": "9f8c12a4b3e2489c91d8e5f2a1b3c4d5",
        "name": "诡秘之主：第一部 愚者",
        "file_id": "a1b2c3d4e5f67890123456789abcdef0",
        "filename": "诡秘之主.txt",
        "status": "制作中",
        "chapter_count": 12,
        "total_words": 45820,
        "created_at": "2026-10-27T08:55:00+00:00",
        "updated_at": "2026-10-27T08:55:00+00:00"
      }
    ]
  },
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```
