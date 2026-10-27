# 删除项目

根据 `project_id` 删除剧目/项目，并级联清理关联的小说存储文件、章节与行级切句数据。

| 属性 | 值 |
| --- | --- |
| **接口地址** | `DELETE /api/v1/projects/{project_id}` |
| **所属模块** | 剧目与项目管理（`app/modules/projects/`） |
| **请求格式** | 无请求体 |
| **响应格式** | `application/json` |
| **认证方式** | 无（当前版本未接入鉴权） |
| **幂等性** | 否（项目不存在时返回 `404`） |

## 接口描述

删除指定项目，服务端会依次完成：

1. 从项目表 `.leo-studio/projects.csv` 中移除项目记录。
2. 级联调用 `TextFileStorageService.delete_file`：
   - 从磁盘删除小说物理存储文件；
   - 从小说表 `.leo-studio/novels.csv` 中移除小说元信息；
   - 从章节内容表 `.leo-studio/novel_contents.csv` 中删除该书所有章节；
   - 从章节切句表 `.leo-studio/novel_sentences.csv` 中删除该书所有切句记录。

## 路径参数

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `project_id` | `string` | **是** | 项目唯一标识，32 位十六进制字符串 |

## 响应字段

### `data` 对象（`ProjectDeleteResponse`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `project_id` | `string` | 被删除项目的唯一标识 |
| `name` | `string` | 被删除项目的名称 |
| `deleted` | `boolean` | 是否删除成功，成功恒为 `true` |

## 成功示例

### 请求

```bash
curl -X DELETE "http://127.0.0.1:8000/api/v1/projects/9f8c12a4b3e2489c91d8e5f2a1b3c4d5"
```

### 响应

```json
{
  "code": 0,
  "message": "项目删除成功",
  "data": {
    "project_id": "9f8c12a4b3e2489c91d8e5f2a1b3c4d5",
    "name": "诡秘之主：第一部 愚者",
    "deleted": true
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
