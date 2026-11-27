# 绑定台词说话人

| 属性 | 值 |
| --- | --- |
| 接口地址 | `PUT /api/v3/projects/{project_id}/chapters/{chapter_id}/dialogues/{dialogue_id}/speaker` |
| 请求格式 | `application/json` |
| 响应格式 | `application/json` |
| 认证方式 | 无 |

## 请求参数

### Path

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `project_id` | `string` | 是 | 项目唯一标识 |
| `chapter_id` | `string` | 是 | 章节唯一标识 |
| `dialogue_id` | `string` | 是 | 台词片段 ID |

### Body

| 字段 | 类型 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- | --- |
| `speaker_id` | `string` | 是 | — | 要绑定的当前项目角色 ID |
| `binding_source` | `string` | 否 | `manual` | `manual` / `auto` / `imported` |
| `binding_confidence` | `number` | 否 | — | 自动匹配置信度，范围 `0.0~1.0` |

解绑请使用 DELETE 接口，不向本接口传 `null`。

## 返回字段

### `data`（`DialogueInfo`）

返回绑定后的完整台词片段，关键字段如下：

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `dialogue_id` | `string` | 台词片段 ID |
| `content` | `string` | 片段正文 |
| `segment_type` | `string` | `narration` / `dialogue` |
| `speaker_id` | `string` | 说话人角色 ID |
| `speaker_name` | `string` | 说话人名称 |
| `binding_source` | `string` | 绑定来源 |
| `binding_confidence` | `number/null` | 自动匹配置信度 |
| `speaker_updated_at` | `string` | 绑定更新时间 |

完整字段见 [台词详情](get.md)。

## curl

```bash
curl -X PUT "http://127.0.0.1:58685/api/v3/projects/9f8c12a4b3e2489c91d8e5f2a1b3c4d5/chapters/1f2e3d4c5b6a7890abcdef1234567890/dialogues/dialogue_1/speaker" \
  -H "Content-Type: application/json" \
  -d '{"speaker_id":"c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8","binding_source":"manual"}'
```

## 常见错误

| HTTP | 业务码 | 场景 |
| --- | --- | --- |
| 404 | 40400 | 项目、章节、片段或角色不存在，或角色不属于当前项目 |
| 422 | 40000 | 角色 ID 为空、绑定来源或置信度非法 |
