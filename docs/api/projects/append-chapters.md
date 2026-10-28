# 向项目追加上传章节文件

向已有项目增量上传小说文件，自动清洗分章并安全追加新章节。

| 属性 | 值 |
| --- | --- |
| **接口地址** | `POST /api/v1/projects/{project_id}/chapters/upload` |
| **别名路径** | `POST /api/v1/projects/{project_id}/chapters` |
| **所属模块** | 剧目与项目管理（`app/modules/projects/`） |
| **请求格式** | `multipart/form-data` |
| **响应格式** | `application/json` |
| **认证方式** | 无（当前版本未接入鉴权） |
| **幂等性** | 否（追加/变更操作） |

## 接口描述

针对连载小说或多批次交付文稿场景，为已有项目增量上传新小说文件：
- **安全防污染机制**：默认 `overwrite_existing=false`，自动比对项目已有章节。如遇新文件包含旧章节（如已有 1～100 章，新上传了包含 1～200 章的爬虫文稿），**默认自动跳过旧章节，仅提取并追加新章节（101～200）**，防止低质量爬虫数据洗白破坏已有人工校对的高清正文。
- **物理快照留痕**：每次上传独立落盘，不物理覆盖旧文件。
- **自动顺延序号**：新增章节序号自动承接已有章节的最大序号，确保章节序列连续。
- **自动同步统计**：完成追加后自动更新项目的总章节数、总字数及更新时间。

## 路径参数

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `project_id` | `string` | **是** | 项目唯一标识，标准 UUID（UUID4，非雪花 ID） |

## 表单请求参数（`multipart/form-data`）

| 参数名 | 类型 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- | --- |
| `file` | `UploadFile` | **是** | - | 增量小说文稿（支持 `.txt`, `.md` 等文本文件） |
| `overwrite_existing` | `boolean` | 否 | `false` | 是否覆盖已有同序号/同名章节。默认 `false`（安全防污染模式），仅追加新章节 |
| `dry_run` | `boolean` | 否 | `false` | 是否仅做差异预检。为 `true` 时比对冲突并返回预期变动，不实际修改项目数据与数据库 |

## 响应字段

### `data` 对象（`ProjectChapterAppendResponse`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `project_id` | `string` | 项目唯一标识 (UUID4) |
| `file_id` | `string` | 关联小说文本文件 ID (UUID4) |
| `added_chapters` | `integer` | 本次新增追加的章节数量 |
| `skipped_chapters` | `integer` | 本次跳过（或保留）的已存在章节数量 |
| `total_chapters` | `integer` | 项目最新总章节数 |
| `total_words` | `integer` | 项目最新总字数 |
| `has_conflicts` | `boolean` | 是否检测到新旧章节内容或字数差异冲突 |
| `conflicts` | `ChapterConflictItem[]` | 冲突章节差异列表（包含章节标题、原字数、新字数、差值） |
| `updated_at` | `string` | 项目最新更新时间（ISO 8601 UTC 格式） |


### `conflicts[]` 冲突元素（`ChapterConflictItem`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `chapter_id` | `string` | 冲突已有章节 ID (UUID4) |
| `chapter_index` | `integer` | 章节序号 |
| `chapter_title` | `string` | 章节标题 |
| `existing_word_count` | `integer` | 项目中现有章节字数 |
| `incoming_word_count` | `integer` | 新上传文稿中对应章节字数 |
| `word_diff` | `integer` | 字数差额（新字数 - 旧字数） |

## 成功示例

### 请求

```bash
curl -X POST "http://127.0.0.1:8000/api/v1/projects/9f8c12a4b3e2489c91d8e5f2a1b3c4d5/chapters/upload" \
  -F "file=@101-200.txt" \
  -F "overwrite_existing=false" \
  -F "dry_run=false"
```

```python
import httpx

project_id = "9f8c12a4b3e2489c91d8e5f2a1b3c4d5"
with open("101-200.txt", "rb") as f:
    response = httpx.post(
        f"http://127.0.0.1:8000/api/v1/projects/{project_id}/chapters/upload",
        files={"file": ("101-200.txt", f, "text/plain")},
        data={"overwrite_existing": "false", "dry_run": "false"},
    )
print(response.json())
```

### 响应

```json
{
  "code": 0,
  "message": "章节追加成功，新增 100 章，跳过 100 章",
  "data": {
    "project_id": "9f8c12a4b3e2489c91d8e5f2a1b3c4d5",
    "file_id": "3a8b2c1d0e4f5a6b7c8d9e0f1a2b3c4d",
    "added_chapters": 100,
    "skipped_chapters": 100,
    "total_chapters": 200,
    "total_words": 612800,
    "has_conflicts": false,
    "conflicts": [],
    "updated_at": "2026-10-27T19:30:00.000000Z"
  },

  "request_id": "req-9c8b7a6d5e4f3a2b"
}
```


## 异常与错误码

| HTTP 状态码 | 业务错误码 | 错误场景 | 解决方案 |
| :--- | :--- | :--- | :--- |
| `404 Not Found` | `40400` | 指定 `project_id` 的项目不存在 | 检查项目 ID 是否正确 |
| `422 Unprocessable` | `40000` | 上传文件为空或文件名不合法 | 检查上传文件及文件名 |
| `413 Payload Too Large` | `41300` | 上传文件大小超出限制 | 检查系统配置的最大单文件大小 |
| `415 Unsupported Type` | `41500` | 上传了不受支持的文件格式 | 仅支持 `.txt`、`.md` 等文本文件 |
