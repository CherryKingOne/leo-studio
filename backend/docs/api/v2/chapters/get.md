# 查询章节详细内容

查询指定项目某个章节的正文、切句及其多音字标注。

| 属性 | 值 |
| --- | --- |
| **接口地址** | `GET /api/v2/projects/{project_id}/chapters/{chapter_id}` |
| **所属模块** | 章节（`chapters`） |
| **请求格式** | 无请求体 |
| **响应格式** | `application/json` |
| **认证方式** | 无 |
| **幂等性** | 是 |

## 接口描述

先校验项目存在且未删除，再校验章节属于该项目，返回章节正文、按序号升序排列的
切句列表，以及每句按 `char_index` 升序排列的多音字标注。

## 请求参数

### Path

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `project_id` | `string` | **是** | 项目唯一标识 |
| `chapter_id` | `string` | **是** | 章节唯一标识 |

### Header

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `X-Request-ID` | `string` | 否 | 自定义请求追踪 ID |

## 响应字段

### `data` 对象（`ChapterContentInfo`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `project_id` | `string` | 项目唯一标识 |
| `file_id` | `string` | 关联小说文件标识 |
| `chapter_id` | `string` | 章节唯一标识 |
| `chapter_index` | `integer` | 章节序号 |
| `chapter_title` | `string` | 章节标题 |
| `word_count` | `integer` | 章节字数 |
| `content` | `string` | 章节正文 |
| `sentences` | `object[]` | 切句列表，元素为 `ChapterSentenceInfo` |

### `sentences[]` 元素（`ChapterSentenceInfo`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `sentence_id` | `string` | 切句唯一标识 |
| `sentence_index` | `integer` | 章节内句子序号 |
| `content` | `string` | 句子内容 |
| `word_count` | `integer` | 句子字数 |
| `polyphones` | `object[]` | 多音字标注列表，元素为 `SentencePolyphoneInfo` |

### `polyphones[]` 元素（`SentencePolyphoneInfo`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `polyphone_id` | `string` | 标注唯一标识 |
| `sentence_id` | `string` | 所属切句标识 |
| `char_index` | `integer` | 字符在句子中的下标 |
| `character` | `string` | 标注字符 |
| `pinyin` | `string` | 拼音 |
| `pinyin_base` | `string` | 基础拼音（无声调） |
| `tone` | `integer` | 声调 |
| `tts_pinyin` | `string` | TTS 用拼音 |
| `created_at` | `string` | 创建时间（RFC 3339 UTC） |
| `updated_at` | `string` | 更新时间（RFC 3339 UTC） |

## 代码示例

```bash
curl -s "http://127.0.0.1:58685/api/v2/projects/9f8c12a4b3e2489c91d8e5f2a1b3c4d5/chapters/ch_0001"
```

## 成功响应示例

```json
{
  "code": 0,
  "message": "success",
  "data": {
    "project_id": "9f8c12a4b3e2489c91d8e5f2a1b3c4d5",
    "file_id": "a1b2c3d4e5f67890123456789abcdef0",
    "chapter_id": "ch_0001",
    "chapter_index": 0,
    "chapter_title": "第一章 开端",
    "word_count": 3200,
    "content": "第一章 开端\n他缓缓抬起头……",
    "sentences": [
      {
        "sentence_id": "s_0001",
        "sentence_index": 0,
        "content": "他缓缓抬起头。",
        "word_count": 7,
        "polyphones": []
      }
    ]
  },
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

## 失败响应示例

### 404 章节不存在

```json
{
  "code": 40400,
  "message": "章节不存在：ch_9999",
  "data": null,
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```