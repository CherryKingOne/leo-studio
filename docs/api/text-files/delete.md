# 删除文本文件

根据 `file_id` 删除已上传的文本文件。

| 属性 | 值 |
| --- | --- |
| **接口地址** | `DELETE /api/v1/text-files/{file_id}` |
| **所属模块** | 文本文件管理（`app/modules/text_files/`） |
| **请求格式** | 无请求体 |
| **响应格式** | `application/json` |
| **认证方式** | 无（当前版本未接入鉴权） |
| **幂等性** | 否（文件不存在时返回 `404`） |

## 接口描述

删除指定文件，包含两个动作：

1. 从磁盘删除实际存储的文件（`{file_id}{ext}`）。
2. 从元数据索引（`.text-files-index.json`）中移除对应记录。

删除成功后，该 `file_id` 再次查询将返回 `404`，文件内容不可恢复。

## 路径参数

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `file_id` | `string` | **是** | 文件唯一标识，32 位十六进制字符串 |

## 响应字段

### `data` 对象（`TextFileDeleteResponse`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `file_id` | `string` | 被删除文件的唯一标识 |
| `filename` | `string` | 被删除文件的原始文件名 |
| `deleted` | `boolean` | 是否删除成功，成功恒为 `true` |

## 成功示例

### 请求

```bash
curl -X DELETE "http://127.0.0.1:8000/api/v1/text-files/9f2c1a8b7d6e5f4a3b2c1d0e9f8a7b6c"
```

```python
import httpx

file_id = "9f2c1a8b7d6e5f4a3b2c1d0e9f8a7b6c"
response = httpx.delete(f"http://127.0.0.1:8000/api/v1/text-files/{file_id}")

if response.status_code == 200:
    print("已删除:", response.json()["data"]["filename"])
else:
    print("删除失败:", response.json()["message"])
```

### 响应

`HTTP/1.1 200 OK`

```json
{
  "code": 0,
  "message": "文件删除成功",
  "data": {
    "file_id": "9f2c1a8b7d6e5f4a3b2c1d0e9f8a7b6c",
    "filename": "notes.md",
    "deleted": true
  },
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

## 错误响应

### `404` 文件不存在

`file_id` 不在索引中，文件此前已被删除或从未存在。

```json
{
  "code": 40400,
  "message": "文件不存在：9f2c1a8b7d6e5f4a3b2c1d0e9f8a7b6c",
  "data": null,
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

## 注意事项

- **不可恢复**：删除操作直接移除磁盘文件，没有回收站或软删除机制。
- **非幂等**：对同一个 `file_id` 连续调用两次，第二次返回 `404`。
  如需幂等语义，客户端应将 `404` 也视为成功。
- **索引与磁盘**：若磁盘文件被外部删除但索引仍在，删除接口仍会返回 `200`，
  并同时清理索引记录。
- **并发**：当前版本未对同一文件的并发删除加锁；
  多个请求同时删除同一文件时，其中一个会返回 `404`。
- **模块范围**：只影响文本文件模块的数据，其它功能模块的存储不受影响。

## 相关接口

- [上传单个文本文件](./upload.md) —— 重新上传文件
- [文本文件列表](./list.md) —— 批量管理已上传文件
