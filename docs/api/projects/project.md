# 查询项目详情

根据 `project_id` 查询单个剧目/项目的详细信息。

| 属性 | 值 |
| --- | --- |
| **接口地址** | `GET /api/v1/projects/{project_id}` |
| **所属模块** | 剧目与项目管理（`app/modules/projects/`） |
| **请求格式** | 无请求体 |
| **响应格式** | `application/json` |
| **认证方式** | 无（当前版本未接入鉴权） |
| **幂等性** | 是（只读操作） |

## 路径参数

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `project_id` | `string` | **是** | 项目唯一标识，32 位十六进制字符串 |

## 响应字段

### `data` 对象（`ProjectResponse`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `project_id` | `string` | 项目唯一标识，标准 UUID（UUID4，非雪花 ID） |
| `name` | `string` | 项目名称 |
| `file_id` | `string` | 关联文本文件唯一标识，标准 UUID（UUID4，非雪花 ID） |
| `filename` | `string` | 原始清洗后文件名 |
| `status` | `string` | 制作状态 |
| `chapter_count` | `integer` | 章节总数 |
| `total_words` | `integer` | 有效总字数 |
| `created_at` | `string` | 创建时间，ISO 8601 格式 |
| `updated_at` | `string` | 更新时间，ISO 8601 格式 |
| `is_deleted` | `boolean` | 是否处于回收站（软删除状态） |
| `deleted_at` | `string \| null` | 软删除时间戳，正常活跃项目中为 `null` |

## 成功示例

### 请求

```bash
curl -X GET "http://127.0.0.1:8000/api/v1/projects/9f8c12a4b3e2489c91d8e5f2a1b3c4d5"
```

### 响应

```json
{
  "code": 0,
  "message": "success",
  "data": {
    "project_id": "9f8c12a4b3e2489c91d8e5f2a1b3c4d5",
    "name": "诡秘之主：第一部 愚者",
    "file_id": "a1b2c3d4e5f67890123456789abcdef0",
    "filename": "诡秘之主.txt",
    "status": "制作中",
    "chapter_count": 12,
    "total_words": 45820,
    "created_at": "2026-10-27T08:55:00+00:00",
    "updated_at": "2026-10-27T08:55:00+00:00"
  },
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

## 失败示例

### 项目不存在或已移入回收站 (404)

```json
{
  "code": 40400,
  "message": "项目不存在或已移入回收站：9f8c12a4b3e2489c91d8e5f2a1b3c4d5",
  "data": null,
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```
