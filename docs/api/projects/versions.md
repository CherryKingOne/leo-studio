# 项目历史版本快照与回滚

查询项目所有历史版本快照列表，或一键将项目数据回滚至指定历史版本（后悔药机制）。

## 1. 查询项目历史版本列表

| 属性 | 值 |
| --- | --- |
| **接口地址** | `GET /api/v1/projects/{project_id}/versions` |
| **所属模块** | 剧目与项目管理（`app/modules/projects/`） |
| **请求格式** | 无请求体 |
| **响应格式** | `application/json` |
| **认证方式** | 无（当前版本未接入鉴权） |
| **幂等性** | 是（只读操作） |

### 路径参数

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `project_id` | `string` | **是** | 项目唯一标识，标准 UUID（UUID4，非雪花 ID） |

### 响应字段（`ProjectVersionListResponse`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `project_id` | `string` | 项目唯一标识 (UUID4) |
| `total` | `integer` | 历史版本快照总数（受滑动窗口淘汰机制约束，最多保留最近 3 版） |
| `items` | `ProjectVersionItem[]` | 版本快照列表（按版本编号倒序排列，最新版本排在最前） |

### `items[]` 元素（`ProjectVersionItem`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `version_id` | `string` | 该版本快照的唯一标识 (UUID4) |
| `project_id` | `string` | 所属项目唯一标识 |
| `file_id` | `string` | 关联物理快照文件唯一标识 |
| `version_number` | `integer` | 版本编号（如 `1`, `2`, `3`...） |
| `filename` | `string` | 该版本上传时的原始文件名 |
| `chapter_count` | `integer` | 该版本对应的总章节数 |
| `total_words` | `integer` | 该版本对应的总字数 |
| `created_at` | `string` | 快照生成时间（ISO 8601 UTC 格式） |
| `remark` | `string` | 操作备注（如“初始创建”、“追加章节(+100)”、“回退至版本 V1”） |

---

## 2. 一键回退至指定历史版本

当用户误操作或发现近期上传的章节被脏数据污染时，可通过此接口将全本章节与切句恢复至指定历史版本。

| 属性 | 值 |
| --- | --- |
| **接口地址** | `POST /api/v1/projects/{project_id}/versions/{version_id}/rollback` |
| **所属模块** | 剧目与项目管理（`app/modules/projects/`） |
| **请求格式** | 无请求体 |
| **响应格式** | `application/json` |
| **认证方式** | 无（当前版本未接入鉴权） |
| **幂等性** | 否（数据恢复与重置操作） |

### 路径参数

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `project_id` | `string` | **是** | 项目唯一标识，标准 UUID（UUID4） |
| `version_id` | `string` | **是** | 目标回退版本快照唯一标识，标准 UUID（UUID4） |

### 响应字段（`ProjectRollbackResponse`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `project_id` | `string` | 项目唯一标识 (UUID4) |
| `restored_version_id` | `string` | 恢复的目标版本快照 ID |
| `restored_version_number` | `integer` | 恢复的目标版本编号（如 `1`） |
| `chapter_count` | `integer` | 恢复后当前项目的总章节数 |
| `total_words` | `integer` | 恢复后当前项目的总字数 |
| `updated_at` | `string` | 回滚操作完成时间（ISO 8601 UTC 格式） |

### 成功示例

```bash
curl -X POST "http://127.0.0.1:8000/api/v1/projects/9f8c12a4b3e2489c91d8e5f2a1b3c4d5/versions/4a2b1c0d9e8f7a6b5c4d3e2f1a0b9c8d/rollback"
```

```json
{
  "code": 0,
  "message": "项目已成功回滚至版本 V1",
  "data": {
    "project_id": "9f8c12a4b3e2489c91d8e5f2a1b3c4d5",
    "restored_version_id": "4a2b1c0d9e8f7a6b5c4d3e2f1a0b9c8d",
    "restored_version_number": 1,
    "chapter_count": 100,
    "total_words": 305400,
    "updated_at": "2026-10-27T20:00:00.000000Z"
  },
  "request_id": "req-1a2b3c4d5e6f"
}
```
