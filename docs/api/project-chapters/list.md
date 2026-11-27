# 查询项目章节大纲

| 属性 | 值 |
| --- | --- |
| 接口地址 | `GET /api/v3/projects/{project_id}/chapters` |
| 请求格式 | Path + Query |
| 响应格式 | `application/json` |
| 认证方式 | 无 |

## 请求参数

### Path

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `project_id` | `string` | 是 | 项目唯一标识 |

### Query

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `keyword` | `string` | 否 | 按章节标题包含关键字过滤；空字符串视为不过滤 |

## 返回字段

### `data`（`ChapterOutlineListResponse`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `project_id` | `string` | 项目唯一标识 |
| `total` | `integer` | 过滤后的章节数量 |
| `items` | `object[]` | 章节大纲列表 |

### `items[]`（`ChapterOutlineInfo`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `chapter_id` | `string` | 章节唯一标识 |
| `chapter_index` | `integer` | 章节序号，从 `0` 开始 |
| `chapter_title` | `string` | 章节标题 |
| `word_count` | `integer` | 章节字数 |

章节按 `chapter_index` 升序排列。

## curl

```bash
curl -sG "http://127.0.0.1:58685/api/v3/projects/9f8c12a4b3e2489c91d8e5f2a1b3c4d5/chapters" \
  --data-urlencode "keyword=第一章"
```

## 常见错误

| HTTP | 业务码 | 场景 |
| --- | --- | --- |
| 404 | 40400 | 项目不存在或已移入回收站 |
