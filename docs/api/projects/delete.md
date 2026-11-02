# 软删除项目（移入回收站）

根据 `project_id` 将剧目/项目移入回收站。软删除会保留关联的小说文件、章节大纲与切句数据，支持随时恢复。

| 属性 | 值 |
| --- | --- |
| **接口地址** | `DELETE /api/v1/projects/{project_id}` |
| **所属模块** | 剧目与项目管理（`app/modules/projects/`） |
| **请求格式** | 无请求体 |
| **响应格式** | `application/json` |
| **认证方式** | 无（当前版本未接入鉴权） |
| **幂等性** | 否（项目不存在或已在回收站中时返回 `404`） |

## 接口描述

软删除指定项目，服务端会完成：

1. 将项目在 `.leo-studio/projects.csv` 中的 `is_deleted` 字段标记为 `true`，并更新 `deleted_at` 为当前时间戳。
2. **不删除**磁盘小说物理存储文件，**不删除**分章与切句数据。
3. 移入回收站后，常规项目列表（`GET /api/v1/projects`）与详情接口（`GET /api/v1/projects/{project_id}`）将不再返回该项目。
4. 可通过回收站列表接口（`GET /api/v1/projects/recycle-bin`）查看，并随时通过恢复接口还原。若需要彻底物理销毁，可调用彻底删除接口。

## 路径参数

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `project_id` | `string` | **是** | 项目唯一标识，32 位十六进制字符串 |

## 响应字段

### `data` 对象（`ProjectDeleteResponse`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `project_id` | `string` | 被移入回收站项目的唯一标识 |
| `name` | `string` | 项目名称 |
| `deleted` | `boolean` | 是否移入回收站成功，成功恒为 `true` |
| `deleted_at` | `string \| null` | 移入回收站的时间（ISO 8601 格式） |

## 成功示例

### 请求

```bash
curl -X DELETE "http://127.0.0.1:8000/api/v1/projects/9f8c12a4b3e2489c91d8e5f2a1b3c4d5"
```

### 响应

```json
{
  "code": 0,
  "message": "项目已成功移入回收站",
  "data": {
    "project_id": "9f8c12a4b3e2489c91d8e5f2a1b3c4d5",
    "name": "诡秘之主：第一部 愚者",
    "deleted": true,
    "deleted_at": "2026-10-28T10:00:00.000000+00:00"
  },
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

## 失败示例

### 项目不存在或已在回收站中 (404)

```json
{
  "code": 40400,
  "message": "项目不存在或已移入回收站：9f8c12a4b3e2489c91d8e5f2a1b3c4d5",
  "data": null,
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```
