# 追加上传章节

向已有项目追加上传小说章节文件，支持冲突检测与预检（dry run）。

| 属性 | 值 |
| --- | --- |
| **接口地址** | `POST /api/v2/projects/{project_id}/chapters` |
| **所属模块** | 章节（`chapters`） |
| **请求格式** | `multipart/form-data` |
| **响应格式** | `application/json` |
| **认证方式** | 无 |
| **幂等性** | 否（非 dry run 时会新增章节） |

## 接口描述

将上传文件切分为章节并与项目已有章节按标题键合并：

1. **新增章节**：标题不存在时新增章节并切句；若标题包含章节序号
   （如 `第1章`、`第2回`、`Chapter 3`），会按序号插入到正确位置，
   无序号则追加到末尾。插入后剩余章节的 `chapter_index` 会自动重排为连续序号。
   实际追加（`dry_run=false`）时，也会按章节序号整理已有章节，修正历史错位数据。
2. **跳过章节**：标题已存在且 `overwrite_existing = false` 时跳过。
3. **覆盖章节**：标题已存在且 `overwrite_existing = true` 时覆盖正文、
   重新切句，并清除该章节原有切句与多音字标注。
4. **冲突检测**：同标题但内容或字数不一致时记录到 `conflicts`。

`dry_run = true` 时仅做预检，不写入任何数据，返回预计新增/跳过/冲突数量，
临时文件会被删除。预检返回的 `total_chapters` 与 `total_words` 为预计值；
查询章节列表不会看到本次上传的章节。

确认预检结果后，省略 `dry_run` 或传 `dry_run=false` 才会实际写入章节。

## 请求参数

### Path

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `project_id` | `string` | **是** | 项目唯一标识 |

### Header

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `Content-Type` | `string` | 是 | `multipart/form-data; boundary=...` |
| `X-Request-ID` | `string` | 否 | 自定义请求追踪 ID |

### Form Data

| 名称 | 类型 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- | --- |
| `file` | `binary` | **是** | — | 章节文件（默认仅 `.txt`） |
| `overwrite_existing` | `boolean` | 否 | `false` | 是否覆盖同名章节（`1/true/yes/on` 为真） |
| `dry_run` | `boolean` | 否 | `false` | 是否仅预检不落盘 |

## 响应字段

### `data` 对象（`ProjectChapterAppendInfo`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `project_id` | `string` | 项目唯一标识 |
| `file_id` | `string` | 关联小说文件标识 |
| `dry_run` | `boolean` | 是否仅预检；`true` 表示未写入任何数据 |
| `added_chapters` | `integer` | 新增章节数 |
| `skipped_chapters` | `integer` | 跳过/覆盖章节数 |
| `total_chapters` | `integer` | 处理后章节总数；`dry_run=true` 时为预计值 |
| `total_words` | `integer` | 处理后总字数；`dry_run=true` 时为预计值 |
| `has_conflicts` | `boolean` | 是否存在冲突 |
| `conflicts` | `object[]` | 冲突列表，元素为 `ChapterConflictInfo` |
| `chapters` | `object[]` | 本次处理的章节列表，元素为 `ChapterAppendItem` |
| `updated_at` | `string` | 项目更新时间（RFC 3339 UTC） |

### `chapters[]` 元素（`ChapterAppendItem`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `chapter_id` | `string \| null` | 章节唯一标识；预检新增时为 `null` |
| `chapter_index` | `integer \| null` | 章节序号；预检新增时为 `null` |
| `chapter_title` | `string` | 章节标题 |
| `action` | `string` | `added` / `skipped` / `overwritten` |
| `word_count` | `integer` | 本次上传章节的字数 |

### `conflicts[]` 元素（`ChapterConflictInfo`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `chapter_id` | `string` | 已有章节标识 |
| `chapter_index` | `integer` | 章节序号 |
| `chapter_title` | `string` | 章节标题 |
| `existing_word_count` | `integer` | 已有字数 |
| `incoming_word_count` | `integer` | 上传章节字数 |
| `word_diff` | `integer` | 字数差（新 - 旧） |

## 代码示例

```bash
# 预检
curl -X POST "http://127.0.0.1:58685/api/v2/projects/9f8c12a4b3e2489c91d8e5f2a1b3c4d5/chapters" \
  -F "file=@./new_chapters.txt" \
  -F "dry_run=true"

# 实际追加并覆盖同名章节
curl -X POST "http://127.0.0.1:58685/api/v2/projects/9f8c12a4b3e2489c91d8e5f2a1b3c4d5/chapters" \
  -F "file=@./new_chapters.txt" \
  -F "overwrite_existing=true"
```

## 成功响应示例

```json
{
  "code": 0,
  "message": "章节追加成功，新增 2 章，跳过 1 章",
  "data": {
    "project_id": "9f8c12a4b3e2489c91d8e5f2a1b3c4d5",
    "file_id": "a1b2c3d4e5f67890123456789abcdef0",
    "dry_run": false,
    "added_chapters": 2,
    "skipped_chapters": 1,
    "total_chapters": 14,
    "total_words": 52000,
    "has_conflicts": false,
    "conflicts": [],
    "chapters": [
      {
        "chapter_id": "c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8",
        "chapter_index": 12,
        "chapter_title": "第12章 新章节",
        "action": "added",
        "word_count": 2100
      },
      {
        "chapter_id": "d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9",
        "chapter_index": 13,
        "chapter_title": "第13章 另一新章节",
        "action": "added",
        "word_count": 1800
      },
      {
        "chapter_id": "e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0",
        "chapter_index": 1,
        "chapter_title": "第2章 已存在章节",
        "action": "skipped",
        "word_count": 1500
      }
    ],
    "updated_at": "2026-10-27T10:00:00+00:00"
  },
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

### 预检响应示例（`dry_run=true`，未写入）

```json
{
  "code": 0,
  "message": "预检完成（未写入）：预计新增 1 章，跳过 0 章，冲突 0 处",
  "data": {
    "project_id": "9f8c12a4b3e2489c91d8e5f2a1b3c4d5",
    "file_id": "a1b2c3d4e5f67890123456789abcdef0",
    "dry_run": true,
    "added_chapters": 1,
    "skipped_chapters": 0,
    "total_chapters": 1108,
    "total_words": 2448491,
    "has_conflicts": false,
    "conflicts": [],
    "chapters": [
      {
        "chapter_id": null,
        "chapter_index": null,
        "chapter_title": "第1109章 新章节",
        "action": "added",
        "word_count": 2136
      }
    ],
    "updated_at": "2026-11-14T05:10:53.959938Z"
  },
  "request_id": "d6e553161c8443628fd0819634d5afd3"
}
```

## 失败响应示例

### 404 项目不存在

```json
{
  "code": 40400,
  "message": "项目不存在或已移入回收站：9f8c12a4b3e2489c91d8e5f2a1b3c4d5",
  "data": null,
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

### 422 文件内容为空

```json
{
  "code": 40000,
  "message": "上传文件内容为空",
  "data": null,
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```