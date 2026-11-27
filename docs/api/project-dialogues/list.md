# 查询项目章节台词列表

| 属性 | 值 |
| --- | --- |
| 接口地址 | `GET /api/v3/projects/{project_id}/chapters/{chapter_id}/dialogues` |
| 请求格式 | Query |
| 响应格式 | `application/json` |
| 认证方式 | 无 |

## 请求参数

### Path

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `project_id` | `string` | 是 | 项目唯一标识 |
| `chapter_id` | `string` | 是 | 章节唯一标识 |

### Query

| 参数 | 类型 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- | --- |
| `page` | `integer` | 否 | `1` | 页码 |
| `page_size` | `integer` | 否 | `20` | 每页数量，范围 `1~100` |
| `segment_type` | `string` | 否 | — | `narration` / `dialogue` |
| `binding_status` | `string` | 否 | `all` | `all` / `bound` / `unbound` |
| `speaker_id` | `string` | 否 | — | 按说话人角色过滤 |
| `keyword` | `string` | 否 | — | 按片段正文关键字过滤 |

结果固定按来源句子序号、片段序号升序排列。

## 返回字段

### `data`（`DialogueListResponse`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `project_id` | `string` | 项目 ID |
| `chapter_id` | `string` | 章节 ID |
| `total` | `integer` | 过滤后的总记录数 |
| `page` | `integer` | 当前页 |
| `page_size` | `integer` | 每页数量 |
| `pages` | `integer` | 总页数 |
| `items` | `DialogueInfo[]` | 台词片段列表 |

### `items[]`（`DialogueInfo`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `project_id` | `string` | 项目 ID |
| `chapter_id` | `string` | 章节 ID |
| `dialogue_id` | `string` | 台词片段 ID |
| `parent_sentence_id` | `string` | 来源句子 ID |
| `sentence_index` | `integer` | 来源句子序号 |
| `segment_index` | `integer` | 片段在句子内的序号 |
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
curl -s "http://127.0.0.1:58685/api/v3/projects/9f8c12a4b3e2489c91d8e5f2a1b3c4d5/chapters/1f2e3d4c5b6a7890abcdef1234567890/dialogues?binding_status=unbound&segment_type=dialogue&page=1&page_size=20"
```

## 常见错误

| HTTP | 业务码 | 场景 |
| --- | --- | --- |
| 404 | 40400 | 项目或章节不存在，或章节不属于该项目 |
| 422 | 40000 | 分页、片段类型或绑定状态非法 |
