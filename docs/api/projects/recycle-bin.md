# 回收站管理

提供回收站项目的分页查询与一键清空回收站能力。

---

## 1. 查询回收站项目列表

分页获取已软删除移入回收站的项目列表，按删除时间 `deleted_at` 倒序排列。

| 属性 | 值 |
| --- | --- |
| **接口地址** | `GET /api/v1/projects/recycle-bin` |
| **所属模块** | 剧目与项目管理（`app/modules/projects/`） |
| **请求格式** | 查询参数（Query Parameters） |
| **响应格式** | `application/json` |
| **认证方式** | 无（当前版本未接入鉴权） |
| **幂等性** | 是 |

### 查询参数

| 参数名 | 类型 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- | --- |
| `page` | `integer` | 否 | `1` | 页码，必须 $\ge 1$ |
| `page_size` | `integer` | 否 | `20` | 每页数量，范围 $1 \sim 100$ |

### 响应字段

#### `data` 对象（`ProjectListResponse`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `total` | `integer` | 回收站中的项目总数 |
| `page` | `integer` | 当前页码 |
| `page_size` | `integer` | 每页数量 |
| `pages` | `integer` | 总页数 |
| `items` | `ProjectListItem[]` | 项目简要信息列表 |

#### `items` 数组元素字段

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `project_id` | `string` | 项目唯一标识 (UUID) |
| `name` | `string` | 项目名称 |
| `file_id` | `string` | 关联文本文件 ID |
| `filename` | `string` | 原始文件名 |
| `status` | `string` | 制作状态 |
| `chapter_count` | `integer` | 章节数 |
| `total_words` | `integer` | 总字数 |
| `created_at` | `string` | 创建时间 (ISO 8601) |
| `updated_at` | `string` | 更新时间 (ISO 8601) |
| `is_deleted` | `boolean` | 是否处于回收站（软删除状态），在回收站中恒为 `true` |
| `deleted_at` | `string \| null` | 软删除时间戳 (ISO 8601) |

### 请求示例

```bash
curl -X GET "http://127.0.0.1:8000/api/v1/projects/recycle-bin?page=1&page_size=20"
```

### 响应示例

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
        "name": "诡秘之主：第一部 愚者",
        "file_id": "3a1b2c4d5e6f7a8b9c0d1e2f3a4b5c6d",
        "filename": "guimi.txt",
        "status": "制作中",
        "chapter_count": 100,
        "total_words": 250000,
        "created_at": "2026-10-28T08:00:00+00:00",
        "updated_at": "2026-10-28T10:00:00+00:00",
        "is_deleted": true,
        "deleted_at": "2026-10-28T10:00:00+00:00"
      }
    ]
  },
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

---

## 2. 清空回收站

永久彻底物理删除回收站内的**所有项目**，并级联清理底层物理小说文件、章节与行级切句数据。

| 属性 | 值 |
| --- | --- |
| **接口地址** | `DELETE /api/v1/projects/recycle-bin` |
| **所属模块** | 剧目与项目管理（`app/modules/projects/`） |
| **请求格式** | 无请求体 |
| **响应格式** | `application/json` |
| **认证方式** | 无（当前版本未接入鉴权） |
| **幂等性** | 是（多次清空返回 `cleared_count: 0`） |

### 响应字段

#### `data` 对象（`ProjectClearRecycleBinResponse`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `cleared_count` | `integer` | 彻底删除的项目数量 |
| `cleared` | `boolean` | 是否清空成功，成功恒为 `true` |

### 请求示例

```bash
curl -X DELETE "http://127.0.0.1:8000/api/v1/projects/recycle-bin"
```

### 响应示例

```json
{
  "code": 0,
  "message": "回收站已清空，共彻底删除 2 个项目",
  "data": {
    "cleared_count": 2,
    "cleared": true
  },
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```
