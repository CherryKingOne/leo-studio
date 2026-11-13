# 保存句子多音字标注

以全量覆盖方式保存指定句子的多音字标注。

| 属性 | 值 |
| --- | --- |
| **接口地址** | `PUT /api/v1/projects/{project_id}/sentences/{sentence_id}/polyphones` |
| **所属模块** | 项目管理（`src/modules/projects/`） |
| **请求格式** | `application/json` |
| **响应格式** | `application/json` |
| **认证方式** | 无 |
| **幂等性** | 是（全量覆盖） |

## 接口描述

先删除该句子已有的全部标注，再按请求写入新标注。服务端会校验：

- `pinyin` 非空。
- `character` 为单个字符。
- `char_index` 在句子长度范围内且不重复。
- `char_index` 位置的实际字符与 `character` 一致。

`pinyin_base`、`tone`、`tts_pinyin` 未提供时，会根据 `pinyin` 自动推导。

## 请求参数

### Path

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `project_id` | `string` | **是** | 项目唯一标识 |
| `sentence_id` | `string` | **是** | 切句唯一标识 |

### Header

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `Content-Type` | `string` | 是 | `application/json` |
| `X-Request-ID` | `string` | 否 | 自定义请求追踪 ID |

### Body（`SentencePolyphoneSaveRequest`）

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `annotations` | `object[]` | 是 | 标注数组，元素为 `PolyphoneAnnotation` |

### `annotations[]` 元素（`PolyphoneAnnotation`）

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `char_index` | `integer` | 是 | 字符下标（从 0 开始） |
| `character` | `string` | 是 | 标注字符（单个字符） |
| `pinyin` | `string` | 是 | 拼音 |
| `pinyin_base` | `string` | 否 | 基础拼音，缺省自动推导 |
| `tone` | `integer` | 否 | 声调，缺省自动推导 |
| `tts_pinyin` | `string` | 否 | TTS 拼音，缺省自动推导 |

## 响应字段

### `data` 对象（`SentencePolyphoneListResponse`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `sentence_id` | `string` | 切句唯一标识 |
| `total` | `integer` | 保存后的标注总数 |
| `items` | `object[]` | 保存后的标注列表 |

`SentencePolyphoneInfo` 字段见[章节详细内容](./chapter-content.md#polyphones-元素sentencepolyphoneinfo)。

## 代码示例

```bash
curl -X PUT "http://127.0.0.1:58685/api/v1/projects/9f8c12a4b3e2489c91d8e5f2a1b3c4d5/sentences/s_0001/polyphones" \
  -H "Content-Type: application/json" \
  -d '{
    "annotations": [
      {"char_index": 1, "character": "行", "pinyin": "xing", "tone": 2}
    ]
  }'
```

## 成功响应示例

```json
{
  "code": 0,
  "message": "多音字标注保存成功",
  "data": {
    "sentence_id": "s_0001",
    "total": 1,
    "items": [
      {
        "polyphone_id": "p_0001",
        "sentence_id": "s_0001",
        "char_index": 1,
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

### 422 字符不匹配

```json
{
  "code": 40000,
  "message": "char_index=1 位置的实际字符为 '走'，与标注字符 '行' 不匹配",
  "data": null,
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

### 404 句子不存在

```json
{
  "code": 40400,
  "message": "句子不存在：s_9999",
  "data": null,
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```