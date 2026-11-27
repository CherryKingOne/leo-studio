# 保存项目句子多音字标注

| 属性 | 值 |
| --- | --- |
| 接口地址 | `PUT /api/v3/projects/{project_id}/chapters/{chapter_id}/sentences/{sentence_id}/polyphones` |
| 请求格式 | `application/json` |
| 响应格式 | `application/json` |
| 认证方式 | 无 |

## 接口描述

全量覆盖保存指定句子的多音字标注。提交成功后，该句子原有标注会被全部替换；传入空数组可清空标注。

## 请求参数

### Path

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `project_id` | `string` | 是 | 项目唯一标识 |
| `chapter_id` | `string` | 是 | 章节唯一标识 |
| `sentence_id` | `string` | 是 | 句子唯一标识 |

### Body

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `annotations` | `object[]` | 否 | 标注列表；缺省或为空表示清空 |

### `annotations[]`

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `char_index` | `integer` | 是 | 字符偏移量，不能越界或重复 |
| `character` | `string` | 是 | 单个字符，且与正文对应位置一致 |
| `pinyin` | `string` | 是 | 原始拼音，不能为空 |
| `pinyin_base` | `string` | 否 | 不带声调的拼音 |
| `tone` | `integer` | 否 | 声调 |
| `tts_pinyin` | `string` | 否 | TTS 使用拼音 |

## 返回字段

### `data`（`SentencePolyphoneListResponse`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `sentence_id` | `string` | 句子唯一标识 |
| `total` | `integer` | 保存后的标注总条数 |
| `items` | `object[]` | 保存后的标注列表 |

`items[]` 字段同[查询项目句子多音字标注](sentence-list.md#返回字段)。

## curl

```bash
curl -X PUT "http://127.0.0.1:58685/api/v3/projects/9f8c12a4b3e2489c91d8e5f2a1b3c4d5/chapters/c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8/sentences/d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9/polyphones" \
  -H "Content-Type: application/json" \
  -d '{
    "annotations": [
      {"char_index": 1, "character": "行", "pinyin": "xíng"}
    ]
  }'
```

## 常见错误

| HTTP | 业务码 | 场景 |
| --- | --- | --- |
| 404 | 40400 | 项目、章节或句子不存在，或句子不属于该章节 |
| 422 | 40000 | 拼音、字符、偏移量或重复标注校验失败 |
