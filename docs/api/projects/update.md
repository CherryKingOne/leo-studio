# 更新项目

| 属性 | 值 |
| --- | --- |
| 接口地址 | `PATCH /api/v3/projects/{project_id}` |
| 请求格式 | `application/json` |
| 响应格式 | `application/json` |
| 认证方式 | 无 |

## 请求参数

### Path

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `project_id` | `string` | 是 | 项目唯一标识 |

### Body

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `name` | `string` | 否 | 新项目名称；去除首尾空格后不能为空，最长 255 个字符 |
| `status` | `string` | 否 | 新制作状态；去除首尾空格后不能为空 |

`name` 与 `status` 至少提供一个；都没有提供时返回 `422`。未提供的字段保持不变。

## 返回字段

### `data`（`ProjectInfo`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `project_id` | `string` | 项目唯一标识 |
| `name` | `string` | 项目名称 |
| `file_id` | `string` | 关联文本文件标识 |
| `filename` | `string` | 文件名 |
| `status` | `string` | 制作状态 |
| `chapter_count` | `integer` | 章节总数 |
| `total_words` | `integer` | 总字数 |
| `created_at` | `string` | 创建时间 |
| `updated_at` | `string` | 更新时间 |
| `is_deleted` | `boolean` | 是否在回收站 |
| `deleted_at` | `string/null` | 删除时间 |

## curl

```bash
curl -X PATCH "http://127.0.0.1:58685/api/v3/projects/9f8c12a4b3e2489c91d8e5f2a1b3c4d5" \
  -H "Content-Type: application/json" \
  -d '{"name":"新项目名","status":"已完成"}'
```

## 常见错误

| HTTP | 业务码 | 场景 |
| --- | --- | --- |
| 422 | 40000 | 名称为空、状态为空或未提供任何更新字段 |
| 404 | 40400 | 项目不存在或已移入回收站 |
