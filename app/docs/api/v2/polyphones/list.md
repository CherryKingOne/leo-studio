# 查询章节多音字标注

查询指定项目中某个章节内全部多音字标注，按句子分组返回。

| 属性 | 值 |
| --- | --- |
| **接口地址** | `GET /api/v2/projects/{project_id}/chapters/{chapter_id}/polyphones` |
| **所属模块** | 多音字标注（`polyphones`） |
| **请求格式** | 无请求体 |
| **响应格式** | `application/json` |
| **认证方式** | 无 |
| **幂等性** | 是 |

## 接口描述

先校验项目存在且未删除，再校验章节属于该项目，返回该章节内所有带多音字标注的句子。

- 只返回至少有一条标注的句子。
- 句子按 `sentence_index` 升序，句子内标注按 `char_index` 升序。
- 若章节没有任何标注，`items` 为空数组，`total` 为 0。

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

### `data` 对象（`ChapterPolyphoneListResponse`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `chapter_id` | `string` | 章节唯一标识 |
| `chapter_title` | `string` | 章节标题 |
| `sentence_count` | `integer` | 含标注的句子数 |
| `total` | `integer` | 标注总数 |
| `items` | `object[]` | 按句子分组的标注列表 |

### `items[]` 元素（`ChapterPolyphoneSentence`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `sentence_id` | `string` | 切句唯一标识 |
| `sentence_index` | `integer` | 句子序号 |
| `content` | `string` | 句子正文 |
| `polyphones` | `object[]` | 该句子的多音字标注，元素为 `SentencePolyphoneInfo` |

`SentencePolyphoneInfo` 字段见[章节详细内容](../chapters/get.md#polyphones-元素sentencepolyphoneinfo)。

## 代码示例

```bash
curl -s "http://127.0.0.1:58685/api/v2/projects/9f8c12a4b3e2489c91d8e5f2a1b3c4d5/chapters/ch_0001/polyphones"
```

## 成功响应示例

```json
{
  "code": 0,
  "message": "success",
  "data": {
    "chapter_id": "ch_0001",
    "chapter_title": "第一章 开端",
    "sentence_count": 1,
    "total": 1,
    "items": [
      {
        "sentence_id": "s_0001",
        "sentence_index": 0,
        "content": "人行道上有一行桌子。",
        "polyphones": [
          {
            "polyphone_id": "p_0001",
            "sentence_id": "s_0001",
            "char_index": 5,
            "character": "行",
            "pinyin": "xing",
            "pinyin_base": "xing",
            "tone": 2,
            "tts_pinyin": "xing2",
            "created_at": "2026-10-27T10:00:00+00:00",
            "updated_at": "2026-10-27T10:00:00+00:00"
          }
        ]
      }
    ]
  },
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

## 失败响应示例

### 404 项目不存在或已删除

```json
{
  "code": 40400,
  "message": "项目不存在或已移入回收站：9f8c12a4b3e2489c91d8e5f2a1b3c4d5",
  "data": null,
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

### 404 章节不存在

```json
{
  "code": 40400,
  "message": "章节不存在：ch_9999",
  "data": null,
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```