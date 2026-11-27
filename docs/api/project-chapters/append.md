# 追加上传项目章节

| 属性 | 值 |
| --- | --- |
| 接口地址 | `POST /api/v3/projects/{project_id}/chapters` |
| 请求格式 | `multipart/form-data` |
| 响应格式 | `application/json` |
| 认证方式 | 无 |

## 接口描述

向指定项目追加小说章节。服务端按章节标题识别新增、跳过与覆盖：

- 新标题：新增章节并切句；标题带章节序号时按序号插入，随后重排 `chapter_index`。
- 同标题且 `overwrite_existing=false`：跳过。
- 同标题且 `overwrite_existing=true`：覆盖正文、重建切句，并清除该章节原有多音字标注。
- 同标题但正文或字数不同：记录到 `conflicts`。
- `dry_run=true`：只预检，不写入项目数据。

## 请求参数

### Path

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `project_id` | `string` | 是 | 项目唯一标识 |

### Form Data

| 名称 | 类型 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- | --- |
| `file` | `binary` | 是 | — | 章节文件，默认仅允许 `.txt` |
| `overwrite_existing` | `boolean` | 否 | `false` | 是否覆盖同名章节；`1/true/yes/on` 视为真 |
| `dry_run` | `boolean` | 否 | `false` | 是否仅预检不写入；`1/true/yes/on` 视为真 |

## 返回字段

### `data`（`ProjectChapterAppendInfo`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `project_id` | `string` | 项目唯一标识 |
| `file_id` | `string` | 关联小说文件标识 |
| `dry_run` | `boolean` | 是否仅预检 |
| `added_chapters` | `integer` | 新增章节数 |
| `skipped_chapters` | `integer` | 跳过或覆盖处理的章节数 |
| `total_chapters` | `integer` | 处理后章节总数；预检时为预计值 |
| `total_words` | `integer` | 处理后总字数；预检时为预计值 |
| `has_conflicts` | `boolean` | 是否存在内容冲突 |
| `conflicts` | `object[]` | 冲突列表 |
| `chapters` | `object[]` | 本次处理章节列表 |
| `updated_at` | `string` | 项目更新时间；预检时返回原项目更新时间 |

### `chapters[]`（`ChapterAppendItem`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `chapter_id` | `string/null` | 章节标识；预检新增时为 `null` |
| `chapter_index` | `integer/null` | 章节序号；预检新增时为 `null` |
| `chapter_title` | `string` | 章节标题 |
| `action` | `string` | `added`、`skipped` 或 `overwritten` |
| `word_count` | `integer` | 本次上传章节字数；跳过时可能返回旧章节字数 |

### `conflicts[]`（`ChapterConflictInfo`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `chapter_id` | `string` | 已有章节标识 |
| `chapter_index` | `integer` | 已有章节序号 |
| `chapter_title` | `string` | 章节标题 |
| `existing_word_count` | `integer` | 已有章节字数 |
| `incoming_word_count` | `integer` | 上传章节字数 |
| `word_diff` | `integer` | 字数差，新字数减旧字数 |

## curl

```bash
# 预检，不写入
curl -X POST "http://127.0.0.1:58685/api/v3/projects/9f8c12a4b3e2489c91d8e5f2a1b3c4d5/chapters" \
  -F "file=@./new_chapters.txt" \
  -F "dry_run=true"

# 实际追加，并覆盖同名章节
curl -X POST "http://127.0.0.1:58685/api/v3/projects/9f8c12a4b3e2489c91d8e5f2a1b3c4d5/chapters" \
  -F "file=@./new_chapters.txt" \
  -F "overwrite_existing=true"
```

## 常见错误

| HTTP | 业务码 | 场景 |
| --- | --- | --- |
| 404 | 40400 | 项目不存在或已移入回收站 |
| 422 | 40000 | 文件为空或内容为空 |
| 413 | 41300 | 文件超过大小限制 |
| 415 | 41500 | 文件扩展名不被允许 |
