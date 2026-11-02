# 查询项目章节详细内容

查询指定项目中具体某个章节的详细内容，包含清洗后的完整正文文本及按行切分后的句子列表。

| 属性 | 值 |
| --- | --- |
| **接口地址** | `GET /api/v1/projects/{project_id}/chapters/{chapter_id}`（别名：`GET /api/v1/projects/{project_id}/chapters/{chapter_id}/content`） |
| **所属模块** | 剧目与项目管理（`app/modules/projects/`） |
| **请求格式** | 无请求体 |
| **响应格式** | `application/json` |
| **认证方式** | 无（当前版本未接入鉴权） |
| **幂等性** | 是（只读操作） |

## 接口描述

根据 `project_id` 与 `chapter_id` 获取指定章节的详细制作内容。服务端会返回：

1. **章节基本元信息** —— 章节序号 `chapter_index`、章节标题 `chapter_title` 与章节字数 `word_count`。
2. **章节完整正文文本 (`content`)** —— 经过颜文字清洗、分隔线过滤后的完整单章小说文本。
3. **行级切句列表 (`sentences`)** —— 供章节制作工作台（Chapter Studio）剧本流卡片直接使用的单句数据，每个句子带有独立生成的 UUID `sentence_id`、行序号 `sentence_index`、内容与字数。

## 路径参数

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `project_id` | `string` | **是** | 项目唯一标识，标准 UUID（UUID4，非雪花 ID） |
| `chapter_id` | `string` | **是** | 章节唯一标识，标准 UUID（UUID4，非雪花 ID） |

## 响应字段

### `data` 对象（`ChapterContentResponse`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `project_id` | `string` | 项目唯一标识 (UUID4) |
| `file_id` | `string` | 关联文本文件唯一标识 (UUID4) |
| `chapter_id` | `string` | 章节唯一标识 (UUID4) |
| `chapter_index` | `integer` | 章节序号，从 0 开始自增 |
| `chapter_title` | `string` | 章节标题 |
| `word_count` | `integer` | 章节字数 |
| `content` | `string` | 章节完整正文文本 |
| `sentences` | `ChapterSentenceItem[]` | 章节内按行切分的句子列表 |

### `sentences[]` 元素（`ChapterSentenceItem`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `sentence_id` | `string` | 句子唯一标识，标准 UUID（UUID4，非雪花 ID） |
| `sentence_index` | `integer` | 章节内行级索引，从 0 开始自增 |
| `content` | `string` | 句子正文内容 |
| `word_count` | `integer` | 句子字数（非空白字符） |

## 成功示例

### 请求

```bash
curl -X GET "http://127.0.0.1:8000/api/v1/projects/9f8c12a4b3e2489c91d8e5f2a1b3c4d5/chapters/c1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6"
```

```python
import httpx

project_id = "9f8c12a4b3e2489c91d8e5f2a1b3c4d5"
chapter_id = "c1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6"
response = httpx.get(
    f"http://127.0.0.1:8000/api/v1/projects/{project_id}/chapters/{chapter_id}"
)

data = response.json()["data"]
print(f"章节: {data['chapter_title']}")
print(f"句子总数: {len(data['sentences'])}")
```

### 响应

```json
{
  "code": 0,
  "message": "success",
  "data": {
    "project_id": "9f8c12a4b3e2489c91d8e5f2a1b3c4d5",
    "file_id": "a1b2c3d4e5f67890123456789abcdef0",
    "chapter_id": "c1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6",
    "chapter_index": 0,
    "chapter_title": "第1章 绯红之月",
    "word_count": 3428,
    "content": "痛！好痛！头好痛！狂暴的剧痛如同一柄烧红的钢铁长矛，狠狠刺入了脑海深处。\n窗外绯红的月光正如同粘稠的血液般，静静淌在粗糙的木制书桌上。",
    "sentences": [
      {
        "sentence_id": "e3f4a5b6c7d8e9f0a1b2c3d4e5f6a7b8",
        "sentence_index": 0,
        "content": "痛！好痛！头好痛！狂暴的剧痛如同一柄烧红的钢铁长矛，狠狠刺入了脑海深处。",
        "word_count": 38
      },
      {
        "sentence_id": "f4a5b6c7d8e9f0a1b2c3d4e5f6a7b8c9",
        "sentence_index": 1,
        "content": "窗外绯红的月光正如同粘稠的血液般，静静淌在粗糙的木制书桌上。",
        "word_count": 30
      }
    ]
  },
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

## 失败示例

### 章节不存在 (404)

```json
{
  "code": 40400,
  "message": "章节不存在：c1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6",
  "data": null,
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```
