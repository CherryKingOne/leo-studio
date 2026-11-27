# 查询项目列表

| 属性 | 值 |
| --- | --- |
| 接口地址 | `GET /api/v3/projects` |
| 请求格式 | Query |
| 响应格式 | `application/json` |
| 认证方式 | 无 |

## 请求参数

### Query

| 名称 | 类型 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- | --- |
| `page` | `integer` | 否 | `1` | 页码，必须大于 0 |
| `page_size` | `integer` | 否 | `20` | 每页数量，范围 `1~100` |
| `status` | `string` | 否 | — | 按制作状态精确过滤，例如 `制作中` |
| `keyword` | `string` | 否 | — | 按项目名称包含关键字过滤 |
| `sort` | `string` | 否 | `created_at` | 排序字段：`created_at`、`updated_at`、`name`；其他值回退到 `created_at` |
| `order` | `string` | 否 | `desc` | 排序方向；仅 `asc`（不区分大小写）为升序，其他值为降序 |

## 返回字段

### `data`（`ProjectListResponse`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `total` | `integer` | 过滤后的总记录数 |
| `page` | `integer` | 当前页码 |
| `page_size` | `integer` | 每页数量 |
| `pages` | `integer` | 总页数，无数据时为 `0` |
| `items` | `object[]` | 项目列表 |

### `items[]`（`ProjectInfo`）

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
| `is_deleted` | `boolean` | 是否在回收站；本接口固定为 `false` |
| `deleted_at` | `string/null` | 删除时间；本接口固定为 `null` |

## curl

```bash
curl -sG "http://127.0.0.1:58685/api/v3/projects" \
  --data-urlencode "page=1" \
  --data-urlencode "page_size=20" \
  --data-urlencode "status=制作中" \
  --data-urlencode "keyword=示例" \
  --data-urlencode "sort=updated_at" \
  --data-urlencode "order=desc"
```

## 常见错误

| HTTP | 业务码 | 场景 |
| --- | --- | --- |
| 422 | 40000 | `page < 1` 或 `page_size` 不在 `1~100` |
