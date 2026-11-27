# 创建项目

| 属性 | 值 |
| --- | --- |
| 接口地址 | `POST /api/v3/projects` |
| 请求格式 | `multipart/form-data` |
| 响应格式 | `application/json` |
| 认证方式 | 无 |
| 幂等性 | 否，每次调用生成新的 `project_id` 与 `file_id` |

## 接口描述

上传小说文本并创建项目。服务端会校验文件名、扩展名与文件大小，完成文本解码、清洗、分章、切句，并建立项目台账。初始状态为 `制作中`。

## 请求参数

### Form Data

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `name` | `string` | 是 | 项目名称，去除首尾空格后不能为空 |
| `file` | `binary` | 是 | 小说文件；默认仅允许 `.txt`，默认最大 50 MB |

### Header

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `Content-Type` | `string` | 是 | `multipart/form-data; boundary=...` |
| `X-Request-ID` | `string` | 否 | 自定义请求追踪 ID |

## 返回字段

### `data`（`ProjectInfo`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `project_id` | `string` | 项目唯一标识，UUID v4 32 位无连字符格式 |
| `name` | `string` | 项目名称 |
| `file_id` | `string` | 关联文本文件标识 |
| `filename` | `string` | 清洗后的文件名 |
| `status` | `string` | 制作状态，初始为 `制作中` |
| `chapter_count` | `integer` | 章节总数 |
| `total_words` | `integer` | 总字数 |
| `created_at` | `string` | 创建时间，RFC 3339 UTC |
| `updated_at` | `string` | 更新时间，RFC 3339 UTC |
| `is_deleted` | `boolean` | 是否已移入回收站 |
| `deleted_at` | `string/null` | 删除时间，未删除为 `null` |

## curl

```bash
curl -X POST "http://127.0.0.1:58685/api/v3/projects" \
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
    "created_at": "2026-11-27T08:55:00Z",
    "updated_at": "2026-11-27T08:55:00Z",
    "is_deleted": false,
    "deleted_at": null
  },
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

## 常见错误

| HTTP | 业务码 | 场景 |
| --- | --- | --- |
| 422 | 40000 | 项目名称为空、上传文件为空或内容为空 |
| 413 | 41300 | 文件超过大小限制 |
| 415 | 41500 | 文件扩展名不在允许列表 |
