# 查询回收站列表

| 属性 | 值 |
| --- | --- |
| 接口地址 | `GET /api/v3/recycle-bin` |
| 请求格式 | Query |
| 响应格式 | `application/json` |
| 认证方式 | 无 |

## 请求参数

### Query

| 名称 | 类型 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- | --- |
| `page` | `integer` | 否 | `1` | 页码，必须大于 0 |
| `page_size` | `integer` | 否 | `20` | 每页数量，范围 `1~100` |

## 返回字段

### `data`（`ProjectListResponse`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `total` | `integer` | 回收站项目总数 |
| `page` | `integer` | 当前页码 |
| `page_size` | `integer` | 每页数量 |
| `pages` | `integer` | 总页数，无数据时为 `0` |
| `items` | `object[]` | 回收站项目列表 |

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
| `is_deleted` | `boolean` | 固定为 `true` |
| `deleted_at` | `string/null` | 移入回收站的时间 |

列表按 `deleted_at` 倒序排列；`deleted_at` 为空时回退到 `updated_at`。

## curl

```bash
curl -sG "http://127.0.0.1:58685/api/v3/recycle-bin" \
  --data-urlencode "page=1" \
  --data-urlencode "page_size=20"
```

## 常见错误

| HTTP | 业务码 | 场景 |
| --- | --- | --- |
| 422 | 40000 | `page < 1` 或 `page_size` 不在 `1~100` |
