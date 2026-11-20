# 查询文本切句列表

查询指定文本文件的切句结果；也可通过 `chapter_id` 查询某一章的切句。

| 属性 | 值 |
| --- | --- |
| **接口地址** | `GET /api/v1/text-files/{file_id}/sentences` |
| **所属模块** | 文本文件（`src/modules/text_files/`） |
| **请求格式** | 无请求体 |
| **响应格式** | `application/json` |
| **认证方式** | 无 |
| **幂等性** | 是 |

## 接口描述

- 未传 `chapter_id`：返回该文件（小说）下的全部切句。
- 传入 `chapter_id`：返回该章节下的切句（`novel_id` 仍为路径中的 `file_id`）。

请求前会先校验 `file_id` 对应文件是否存在。

## 请求参数

### Path

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `file_id` | `string` | **是** | 文件（小说）唯一标识 |

### Query

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `chapter_id` | `string` | 否 | 章节唯一标识；传入后仅返回该章节切句 |

### Header

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `X-Request-ID` | `string` | 否 | 自定义请求追踪 ID |

## 响应字段

### `data` 对象（`TextFileSentenceListResponse`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `novel_id` | `string` | 小说（文件）唯一标识，等于路径 `file_id` |
| `total` | `integer` | 切句总数 |
| `items` | `object[]` | 切句列表，元素为 `TextFileSentenceItem` |

### `items[]` 元素（`TextFileSentenceItem`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `sentence_id` | `string` | 切句唯一标识 |
| `novel_id` | `string` | 所属小说（文件）标识 |
| `chapter_id` | `string` | 所属章节标识 |
| `sentence_index` | `integer` | 章节内句子序号（从 0 开始） |
| `content` | `string` | 句子内容 |
| `word_count` | `integer` | 句子字数 |

## 代码示例

```bash
# 查询整本小说切句
curl -s "http://127.0.0.1:58685/api/v1/text-files/9f2c1a8b7d6e5f4a3b2c1d0e9f8a7b6c/sentences"

# 查询某一章切句
curl -s "http://127.0.0.1:58685/api/v1/text-files/9f2c1a8b7d6e5f4a3b2c1d0e9f8a7b6c/sentences?chapter_id=ch_0001"
```

## 成功响应示例

```json
{
  "code": 0,
  "message": "success",
  "data": {
    "novel_id": "9f2c1a8b7d6e5f4a3b2c1d0e9f8a7b6c",
    "total": 2,
    "items": [
      {
        "sentence_id": "s_0001",
        "novel_id": "9f2c1a8b7d6e5f4a3b2c1d0e9f8a7b6c",
        "chapter_id": "ch_0001",
        "sentence_index": 0,
        "content": "他缓缓抬起头。",
        "word_count": 7
      },
      {
        "sentence_id": "s_0002",
        "novel_id": "9f2c1a8b7d6e5f4a3b2c1d0e9f8a7b6c",
        "chapter_id": "ch_0001",
        "sentence_index": 1,
        "content": "窗外下着雨。",
        "word_count": 6
      }
    ]
  },
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

## 失败响应示例

### 404 文件不存在

```json
{
  "code": 40400,
  "message": "文件不存在：9f2c1a8b7d6e5f4a3b2c1d0e9f8a7b6c",
  "data": null,
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```