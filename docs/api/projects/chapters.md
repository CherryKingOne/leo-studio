# 查询项目章节大纲列表

查询指定项目下小说文件的章节大纲列表（目录索引），按章节序号升序排列。

| 属性 | 值 |
| --- | --- |
| **接口地址** | `GET /api/v1/projects/{project_id}/chapters` |
| **所属模块** | 剧目与项目管理（`app/modules/projects/`） |
| **请求格式** | 无请求体 |
| **响应格式** | `application/json` |
| **认证方式** | 无（当前版本未接入鉴权） |
| **幂等性** | 是（只读操作） |

## 接口描述

根据 `project_id` 获取项目中已上传小说的完整章节大纲（目录树）。包含每个章节的唯一标识、章节序号、章节标题、章节字数以及前段文字摘要。

本接口返回轻量级大纲数据，不携带数十万字的完整正文，非常适合：
- 剧目看板页（Project Hub）的**章节生产流水线**导航渲染；
- 章节目录索引、选章下拉菜单的大纲定位。

若需要获取某单个章节的完整正文与切句流，请使用[章节内容详情接口](./chapter-content.md)。

## 路径参数

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `project_id` | `string` | **是** | 项目唯一标识，标准 UUID（UUID4，非雪花 ID） |

## 响应字段

### `data` 对象（`ChapterOutlineListResponse`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `project_id` | `string` | 项目唯一标识 (UUID4) |
| `total` | `integer` | 章节总数 |
| `items` | `ChapterOutlineItem[]` | 章节大纲列表（按章节序号升序排列） |

### `items[]` 元素（`ChapterOutlineItem`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `chapter_id` | `string` | 章节唯一标识，标准 UUID（UUID4，非雪花 ID） |
| `chapter_index` | `integer` | 章节序号，从 0 开始自增 |
| `chapter_title` | `string` | 章节标题（如 `第1章 绯红之月`） |
| `word_count` | `integer` | 章节有效字数 |

## 成功示例

### 请求

```bash
curl -X GET "http://127.0.0.1:8000/api/v1/projects/9f8c12a4b3e2489c91d8e5f2a1b3c4d5/chapters"
```

```python
import httpx

project_id = "9f8c12a4b3e2489c91d8e5f2a1b3c4d5"
response = httpx.get(f"http://127.0.0.1:8000/api/v1/projects/{project_id}/chapters")

data = response.json()["data"]
print(f"总章节数: {data['total']}")
for ch in data["items"]:
    print(f"[{ch['chapter_index']}] {ch['chapter_title']} ({ch['word_count']} 字)")
```

### 响应

```json
{
  "code": 0,
  "message": "success",
  "data": {
    "project_id": "9f8c12a4b3e2489c91d8e5f2a1b3c4d5",
    "total": 2,
    "items": [
      {
        "chapter_id": "c1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6",
        "chapter_index": 0,
        "chapter_title": "第1章 绯红之月",
        "word_count": 3428
      },
      {
        "chapter_id": "d2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7",
        "chapter_index": 1,
        "chapter_title": "第2章 宿命之枪",
        "word_count": 3810
      }
    ]
  },
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

## 失败示例

### 项目不存在 (404)

```json
{
  "code": 40400,
  "message": "项目不存在：9f8c12a4b3e2489c91d8e5f2a1b3c4d5",
  "data": null,
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```
