# 删除单条多音字标注

删除指定章节下指定句子中某个字符位置的多音字标注。

| 属性 | 值 |
| --- | --- |
| **接口地址** | `DELETE /api/v2/projects/{project_id}/chapters/{chapter_id}/sentences/{sentence_id}/polyphones/{char_index}` |
| **所属模块** | 多音字标注（`polyphones`） |
| **请求格式** | 无请求体 |
| **响应格式** | `application/json` |
| **认证方式** | 无 |
| **幂等性** | 是（重复删除返回 `404`） |

## 接口描述

先校验项目、章节存在且句子属于该章节，再按 `char_index` 删除该句子中的单条标注。
若该位置没有标注，返回 `404`。

## 请求参数

### Path

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `project_id` | `string` | **是** | 项目唯一标识 |
| `chapter_id` | `string` | **是** | 章节唯一标识 |
| `sentence_id` | `string` | **是** | 切句唯一标识 |
| `char_index` | `integer` | **是** | 字符下标（从 0 开始） |

### Header

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `X-Request-ID` | `string` | 否 | 自定义请求追踪 ID |

## 响应字段

### `data` 对象（`SentencePolyphoneItemDeleteResponse`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `polyphone_id` | `string` | 被删除的标注唯一标识 |
| `sentence_id` | `string` | 切句唯一标识 |
| `char_index` | `integer` | 被删除标注的字符下标 |
| `deleted` | `boolean` | 固定为 `true` |

## 代码示例

```bash
curl -X DELETE "http://127.0.0.1:58685/api/v2/projects/9f8c12a4b3e2489c91d8e5f2a1b3c4d5/chapters/ch_0001/sentences/s_0001/polyphones/1"
```

## 成功响应示例

```json
{
  "code": 0,
  "message": "多音字标注删除成功",
  "data": {
    "polyphone_id": "p_0001",
    "sentence_id": "s_0001",
    "char_index": 1,
    "deleted": true
  },
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

## 失败响应示例

### 404 标注不存在

```json
{
  "code": 40400,
  "message": "多音字标注不存在：sentence_id=s_0001, char_index=1",
  "data": null,
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

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