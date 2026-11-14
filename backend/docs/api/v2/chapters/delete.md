# 删除章节

删除项目中的指定章节，并同步清理该章节的切句与多音字标注，刷新项目统计。

| 属性 | 值 |
| --- | --- |
| **接口地址** | `DELETE /api/v2/projects/{project_id}/chapters/{chapter_id}` |
| **所属模块** | 章节（`chapters`） |
| **请求格式** | 无请求体 |
| **响应格式** | `application/json` |
| **认证方式** | 无 |
| **幂等性** | 是（重复删除返回 `404`） |

## 接口描述

先校验项目存在且未删除，再校验章节属于该项目。删除章节记录，并级联删除：

- 该章节的全部切句（`novel_sentences.csv`）
- 上述切句关联的多音字标注（`novel_polyphones.csv`）

删除完成后重新统计该项目的章节数与总字数，并同步更新项目与小说记录。

## 请求参数

### Path

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `project_id` | `string` | **是** | 项目唯一标识 |
| `chapter_id` | `string` | **是** | 章节唯一标识 |

### Header

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `X-Request-ID` | `string` | 否 | 自定义请求追踪 ID |

## 响应字段

### `data` 对象（`ChapterDeleteResponse`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `project_id` | `string` | 项目唯一标识 |
| `chapter_id` | `string` | 被删除的章节唯一标识 |
| `chapter_title` | `string` | 被删除的章节标题 |
| `deleted` | `boolean` | 固定为 `true` |
| `chapter_count` | `integer` | 删除后剩余章节数 |
| `total_words` | `integer` | 删除后剩余总字数 |
| `updated_at` | `string` | 项目更新时间（RFC 3339 UTC） |

## 代码示例

```bash
curl -X DELETE "http://127.0.0.1:58685/api/v2/projects/9f8c12a4b3e2489c91d8e5f2a1b3c4d5/chapters/ch_0001"
```

## 成功响应示例

```json
{
  "code": 0,
  "message": "章节删除成功",
  "data": {
    "project_id": "9f8c12a4b3e2489c91d8e5f2a1b3c4d5",
    "chapter_id": "ch_0001",
    "chapter_title": "第一章 开端",
    "deleted": true,
    "chapter_count": 11,
    "total_words": 42620,
    "updated_at": "2026-10-27T10:10:00+00:00"
  },
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

## 失败响应示例

### 404 项目不存在或已删除

```json
{
  "code": 40400,
  "message": "项目不存在或已移入回收站：9f8c12a4b3e2489c91d8e5f2a1b3c4d5",
  "data": null,
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

### 404 章节不存在

```json
{
  "code": 40400,
  "message": "章节不存在：ch_9999",
  "data": null,
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

## 注意事项

- 删除章节不可恢复，请谨慎调用。
- 删除后剩余章节的 `chapter_index` 保持不变，不会自动重排。