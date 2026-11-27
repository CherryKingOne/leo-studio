# 解绑台词说话人

| 属性 | 值 |
| --- | --- |
| 接口地址 | `DELETE /api/v3/projects/{project_id}/chapters/{chapter_id}/dialogues/{dialogue_id}/speaker` |
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

### `data`

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `dialogue_id` | `string` | 台词片段 ID |
| `unbound` | `boolean` | 固定为 `true` |

重复解绑返回成功。接口幂等，不提供批量解绑；前端全选后可逐条调用本接口。

## curl

```bash
curl -X DELETE "http://127.0.0.1:58685/api/v3/projects/9f8c12a4b3e2489c91d8e5f2a1b3c4d5/chapters/1f2e3d4c5b6a7890abcdef1234567890/dialogues/dialogue_1/speaker"
```

## 常见错误

| HTTP | 业务码 | 场景 |
| --- | --- | --- |
| 404 | 40400 | 项目、章节或台词片段不存在，或片段不属于当前章节 |
