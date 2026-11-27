# 查询台词片段详情

| 属性 | 值 |
| --- | --- |
| 接口地址 | `GET /api/v3/projects/{project_id}/chapters/{chapter_id}/dialogues/{dialogue_id}` |
| 请求格式 | Path |
| 响应格式 | `application/json` |
| 认证方式 | 无 |

## 请求参数

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `project_id` | `string` | 是 | 项目唯一标识 |
| `chapter_id` | `string` | 是 | 章节唯一标识 |
| `dialogue_id` | `string` | 是 | 台词片段 ID |

## 返回字段

### `data`（`DialogueInfo`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `project_id` | `string` | 项目 ID |
| `chapter_id` | `string` | 章节 ID |
| `dialogue_id` | `string` | 台词片段 ID |
| `parent_sentence_id` | `string` | 来源句子 ID |
| `sentence_index` | `integer` | 来源句子序号 |
| `segment_index` | `integer` | 片段序号 |
| `segment_type` | `string` | `narration` / `dialogue` |
| `content` | `string` | 片段正文 |
| `word_count` | `integer` | 片段字数 |
| `parse_status` | `string` | 切分状态 |
| `speaker_id` | `string/null` | 说话人角色 ID |
| `speaker_name` | `string/null` | 说话人名称 |
| `binding_source` | `string/null` | 绑定来源 |
| `binding_confidence` | `number/null` | 自动匹配置信度 |
| `speaker_updated_at` | `string/null` | 最近绑定或解绑时间 |

## curl

```bash
curl -s "http://127.0.0.1:58685/api/v3/projects/9f8c12a4b3e2489c91d8e5f2a1b3c4d5/chapters/1f2e3d4c5b6a7890abcdef1234567890/dialogues/dialogue_1"
```

## 常见错误

| HTTP | 业务码 | 场景 |
| --- | --- | --- |
| 404 | 40400 | 项目、章节或台词片段不存在，或片段不属于当前章节 |
