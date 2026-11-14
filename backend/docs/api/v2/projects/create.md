# 创建项目

以 `multipart/form-data` 创建项目并上传小说文本，服务端自动完成清洗、分章、切句与项目台账建立。

| 属性 | 值 |
| --- | --- |
| **接口地址** | `POST /api/v2/projects` |
| **所属模块** | 项目（`projects`） |
| **请求格式** | `multipart/form-data` |
| **响应格式** | `application/json` |
| **认证方式** | 无 |
| **幂等性** | 否，每次调用生成新的 `project_id` 与 `file_id` |

## 接口描述

v2 不单独暴露文本文件接口，文件上传作为项目创建的组成部分：

1. **参数校验** —— `name` 非空、`file` 非空。
2. **文本处理（内部能力）** —— 调用内部文本文件服务完成文件名清洗、扩展名校验、
   大小校验、内容解码、颜文字清洗、章节切分与切句。
3. **项目台账** —— 生成 `project_id`，状态置为 `制作中`，统计章节数与总字数。

## 请求参数

### Header

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `Content-Type` | `string` | 是 | `multipart/form-data; boundary=...` |
| `X-Request-ID` | `string` | 否 | 自定义请求追踪 ID |

### Form Data

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `name` | `string` | **是** | 项目名称 |
| `file` | `binary` | **是** | 小说文本文件（默认仅 `.txt`） |

## 响应字段

### `data` 对象（`ProjectInfo`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `project_id` | `string` | 项目唯一标识（UUID4，32 位） |
| `name` | `string` | 项目名称 |
| `file_id` | `string` | 关联文本文件标识 |
| `filename` | `string` | 清洗后的文件名 |
| `status` | `string` | 制作状态，初始为 `制作中` |
| `chapter_count` | `integer` | 章节总数 |
| `total_words` | `integer` | 有效总字数 |
| `created_at` | `string` | 创建时间（RFC 3339 UTC） |
| `updated_at` | `string` | 更新时间（RFC 3339 UTC） |
| `is_deleted` | `boolean` | 是否在回收站 |
| `deleted_at` | `string \| null` | 删除时间，未删除为 `null` |

## 代码示例

```bash
curl -X POST "http://127.0.0.1:58685/api/v2/projects" \
  -F "name=示例项目" \
  -F "file=@./example.txt"
```

## 成功响应示例

```json
{
  "code": 0,
  "message": "项目创建成功",
  "data": {
    "project_id": "9f8c12a4b3e2489c91d8e5f2a1b3c4d5",
    "name": "示例项目",
    "file_id": "a1b2c3d4e5f67890123456789abcdef0",
    "filename": "example.txt",
    "status": "制作中",
    "chapter_count": 12,
    "total_words": 45820,
    "created_at": "2026-10-27T08:55:00+00:00",
    "updated_at": "2026-10-27T08:55:00+00:00",
    "is_deleted": false,
    "deleted_at": null
  },
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

## 失败响应示例

### 422 项目名称为空

```json
{
  "code": 40000,
  "message": "项目名称不能为空",
  "data": null,
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

### 415 不支持的文件类型

```json
{
  "code": 41500,
  "message": "不支持的文件类型 .exe，允许的类型：.txt",
  "data": null,
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```