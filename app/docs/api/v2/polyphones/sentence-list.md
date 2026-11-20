# 查询句子多音字标注

查询指定章节中某个句子的全部多音字标注。

| 属性 | 值 |
| --- | --- |
| **接口地址** | `GET /api/v2/projects/{project_id}/chapters/{chapter_id}/sentences/{sentence_id}/polyphones` |
| **所属模块** | 多音字标注（`polyphones`） |
| **请求格式** | 无请求体 |
| **响应格式** | `application/json` |
| **认证方式** | 无 |
| **幂等性** | 是 |

## 接口描述

先校验项目、章节存在，再校验句子属于该章节，返回该句子的全部标注。
若句子存在但无标注，`items` 为空数组，`total` 为 0。

## 请求参数

### Path

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `project_id` | `string` | **是** | 项目唯一标识 |
| `chapter_id` | `string` | **是** | 章节唯一标识 |
| `sentence_id` | `string` | **是** | 切句唯一标识 |

### Header

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `X-Request-ID` | `string` | 否 | 自定义请求追踪 ID |

## 响应字段

### `data` 对象（`SentencePolyphoneListResponse`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `sentence_id` | `string` | 切句唯一标识 |
| `total` | `integer` | 标注总数 |
| `items` | `object[]` | 标注列表，元素为 `SentencePolyphoneInfo` |

`SentencePolyphoneInfo` 字段见[章节详细内容](../chapters/get.md#polyphones-元素sentencepolyphoneinfo)。

## 代码示例

```bash
curl -s "http://127.0.0.1:58685/api/v2/projects/9f8c12a4b3e2489c91d8e5f2a1b3c4d5/chapters/ch_0001/sentences/s_0001/polyphones"
```

## 成功响应示例

```json
{
  "code": 0,
  "message": "success",
  "data": {
    "sentence_id": "s_0001",
    "total": 1,
    "items": [
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
  },
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

## 失败响应示例

### 404 句子不存在或不属于该章节

```json
{
  "code": 40400,
  "message": "句子不存在或不属于该章节：s_9999",
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