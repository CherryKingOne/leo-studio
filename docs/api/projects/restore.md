# 从回收站恢复项目

根据 `project_id` 将回收站中的项目恢复为正常活跃状态。

| 属性 | 值 |
| --- | --- |
| **接口地址** | `POST /api/v1/projects/{project_id}/restore` |
| **所属模块** | 剧目与项目管理（`app/modules/projects/`） |
| **请求格式** | 无请求体 |
| **响应格式** | `application/json` |
| **认证方式** | 无（当前版本未接入鉴权） |
| **幂等性** | 否（非回收站项目重复调用返回 `422`） |

## 接口描述

从回收站恢复指定项目，服务端会完成：

1. 检查指定项目是否存在且当前处于已软删除状态（`is_deleted == true`）；
2. 重置 `is_deleted = false`，清空 `deleted_at` 字段；
3. 更新 `updated_at` 为当前时刻；
4. 恢复后，该项目立即可在正常项目列表与详情接口中查询，所有章节与行级切句数据完好如初。

## 路径参数

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `project_id` | `string` | **是** | 项目唯一标识，32 位十六进制字符串 |

## 响应字段

### `data` 对象（`ProjectRestoreResponse`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `project_id` | `string` | 被恢复的项目唯一标识 |
| `name` | `string` | 项目名称 |
| `restored` | `boolean` | 是否恢复成功，成功恒为 `true` |

## 成功示例

### 请求

```bash
curl -X POST "http://127.0.0.1:8000/api/v1/projects/9f8c12a4b3e2489c91d8e5f2a1b3c4d5/restore"
```

### 响应

```json
{
  "code": 0,
  "message": "项目已成功恢复",
  "data": {
    "project_id": "9f8c12a4b3e2489c91d8e5f2a1b3c4d5",
    "name": "诡秘之主：第一部 愚者",
    "restored": true
  },
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

## 失败示例

### 项目不存在 (404)

```json
{
  "code": 40400,
  "message": "项目不存在：9f8c12a4b3e2489c91d8e5f2a1b3c4d5",
  "data": null,
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

### 项目未处于回收站中 (422)

```json
{
  "code": 40000,
  "message": "项目未处于回收站中：9f8c12a4b3e2489c91d8e5f2a1b3c4d5",
  "data": null,
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```
