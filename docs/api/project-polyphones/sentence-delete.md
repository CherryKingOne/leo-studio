# 清空项目句子多音字标注

| 属性 | 值 |
| --- | --- |
| 接口地址 | `DELETE /api/v3/projects/{project_id}/chapters/{chapter_id}/sentences/{sentence_id}/polyphones` |
| 请求格式 | Path |
| 响应格式 | `application/json` |
| 认证方式 | 无 |

## 请求参数

### Path

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `project_id` | `string` | 是 | 项目唯一标识 |
| `chapter_id` | `string` | 是 | 章节唯一标识 |
| `sentence_id` | `string` | 是 | 句子唯一标识 |

## 返回字段

### `data`（`SentencePolyphoneDeleteResponse`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `sentence_id` | `string` | 句子唯一标识 |
| `deleted_count` | `integer` | 本次删除的标注数量；没有标注时为 `0` |
| `deleted` | `boolean` | 是否已完成清空，固定为 `true` |

## curl

```bash
curl -X DELETE "http://127.0.0.1:58685/api/v3/projects/9f8c12a4b3e2489c91d8e5f2a1b3c4d5/chapters/c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8/sentences/d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9/polyphones"
```

## 常见错误

| HTTP | 业务码 | 场景 |
| --- | --- | --- |
| 404 | 40400 | 项目、章节或句子不存在，或句子不属于该章节 |
