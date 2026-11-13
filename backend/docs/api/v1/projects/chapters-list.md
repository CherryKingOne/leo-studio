# 查询项目章节大纲

查询指定项目的章节大纲列表（不含正文），按章节序号升序返回。

| 属性 | 值 |
| --- | --- |
| **接口地址** | `GET /api/v1/projects/{project_id}/chapters` |
| **所属模块** | 项目管理（`src/modules/projects/`） |
| **请求格式** | 无请求体 |
| **响应格式** | `application/json` |
| **认证方式** | 无 |
| **幂等性** | 是 |

## 接口描述

先校验项目存在且未被删除，再返回该项目关联小说的所有章节元数据。

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

### `data` 对象（`ChapterOutlineListResponse`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `project_id` | `string` | 项目唯一标识 |
| `total` | `integer` | 章节总数 |
| `items` | `object[]` | 章节大纲列表，元素为 `ChapterOutlineInfo` |

### `items[]` 元素（`ChapterOutlineInfo`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `chapter_id` | `string` | 章节唯一标识 |
| `chapter_index` | `integer` | 章节序号（从 0 开始） |
| `chapter_title` | `string` | 章节标题 |
| `word_count` | `integer` | 章节字数 |

## 代码示例

```bash
curl -s "http://127.0.0.1:58685/api/v1/projects/9f8c12a4b3e2489c91d8e5f2a1b3c4d5/chapters"
```

## 成功响应示例

```json
{
  "code": 0,
  "message": "success",
  "data": {
    "project_id": "9f8c12a4b3e2489c91d8e5f2a1b3c4d5",
    "total": 2,
    "items": [
      {
        "chapter_id": "ch_0001",
        "chapter_index": 0,
        "chapter_title": "第一章 开端",
        "word_count": 3200
      },
      {
        "chapter_id": "ch_0002",
        "chapter_index": 1,
        "chapter_title": "第二章 相遇",
        "word_count": 2800
      }
    ]
  },
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

## 失败响应示例

### 404 项目不存在

```json
{
  "code": 40400,
  "message": "项目不存在或已移入回收站：9f8c12a4b3e2489c91d8e5f2a1b3c4d5",
  "data": null,
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```