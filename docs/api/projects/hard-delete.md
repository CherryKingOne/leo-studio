# 彻底删除项目（物理硬删除）

根据 `project_id` 彻底物理删除项目，并级联清除关联的小说物理文件、章节内容与行级切句数据。操作不可逆。

| 属性 | 值 |
| --- | --- |
| **接口地址** | `DELETE /api/v1/projects/{project_id}/hard` |
| **所属模块** | 剧目与项目管理（`app/modules/projects/`） |
| **请求格式** | 无请求体 |
| **响应格式** | `application/json` |
| **认证方式** | 无（当前版本未接入鉴权） |
| **幂等性** | 否（项目不存在时返回 `404`） |

## 接口描述

彻底物理删除指定项目，服务端会按序完成级联清理：

1. 从项目表 `.leo-studio/projects.csv` 中彻底抹除该项目记录；
2. 级联调用 `TextFileStorageService.delete_file`：
   - 从磁盘彻底删除小说物理上传文件；
   - 从小说元信息表 `.leo-studio/novels.csv` 中移除小说元信息；
   - 从章节内容表 `.leo-studio/novel_contents.csv` 中删除该书所有章节内容；
   - 从章节切句表 `.leo-studio/novel_sentences.csv` 中删除该书所有切句记录。

> [!WARNING]
> 本操作为不可逆的物理硬删除。一旦执行，项目及其关联的小说文件、章节大纲、切句台账将无法找回。

## 路径参数

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `project_id` | `string` | **是** | 项目唯一标识，32 位十六进制字符串 |

## 响应字段

### `data` 对象（`ProjectHardDeleteResponse`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `project_id` | `string` | 被彻底删除的项目唯一标识 |
| `name` | `string` | 项目名称 |
| `hard_deleted` | `boolean` | 是否彻底删除成功，成功恒为 `true` |

## 成功示例

### 请求

```bash
curl -X DELETE "http://127.0.0.1:8000/api/v1/projects/9f8c12a4b3e2489c91d8e5f2a1b3c4d5/hard"
```

### 响应

```json
{
  "code": 0,
  "message": "项目已彻底物理删除",
  "data": {
    "project_id": "9f8c12a4b3e2489c91d8e5f2a1b3c4d5",
    "name": "诡秘之主：第一部 愚者",
    "hard_deleted": true
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
