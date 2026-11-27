# 删除项目章节

| 属性 | 值 |
| --- | --- |
| 接口地址 | `DELETE /api/v3/projects/{project_id}/chapters/{chapter_id}` |
| 请求格式 | Path |
| 响应格式 | `application/json` |
| 认证方式 | 无 |

## 接口描述

删除指定项目下的章节及其切句、多音字标注和台词片段，并重新计算项目章节数与总字数。该操作不可恢复。

## 请求参数

### Path

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `project_id` | `string` | 是 | 项目唯一标识 |
| `chapter_id` | `string` | 是 | 章节唯一标识 |

## 返回字段

### `data`（`ChapterDeleteInfo`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `project_id` | `string` | 项目唯一标识 |
| `chapter_id` | `string` | 被删除章节标识 |
| `chapter_title` | `string` | 被删除章节标题 |
| `deleted` | `boolean` | 是否已删除，固定为 `true` |
| `chapter_count` | `integer` | 删除后的项目章节总数 |
| `total_words` | `integer` | 删除后的项目总字数 |
| `updated_at` | `string` | 项目更新时间 |

## curl

```bash
curl -X DELETE "http://127.0.0.1:58685/api/v3/projects/9f8c12a4b3e2489c91d8e5f2a1b3c4d5/chapters/c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8"
```

## 常见错误

| HTTP | 业务码 | 场景 |
| --- | --- | --- |
| 404 | 40400 | 项目不存在、章节不存在或章节不属于该项目 |
