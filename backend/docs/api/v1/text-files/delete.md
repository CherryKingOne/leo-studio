# 删除文本文件

删除指定文本文件及其关联的章节、切句与多音字标注数据。

| 属性 | 值 |
| --- | --- |
| **接口地址** | `DELETE /api/v1/text-files/{file_id}` |
| **所属模块** | 文本文件（`src/modules/text_files/`） |
| **请求格式** | 无请求体 |
| **响应格式** | `application/json` |
| **认证方式** | 无 |
| **幂等性** | 是（重复删除返回 `404`） |

## 接口描述

删除磁盘文件并级联清理：

1. 删除上传目录中的文件（文件不存在时忽略）。
2. 删除 `novels.csv` 中的元信息。
3. 删除 `novel_contents.csv` 中该小说的章节。
4. 删除 `novel_sentences.csv` 中该小说的切句。
5. 删除 `novel_polyphones.csv` 中相关句子的多音字标注。

## 请求参数

### Path

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `file_id` | `string` | **是** | 文件唯一标识 |

### Header

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `X-Request-ID` | `string` | 否 | 自定义请求追踪 ID |

## 响应字段

### `data` 对象（`TextFileDeleteResponse`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `file_id` | `string` | 被删除文件的唯一标识 |
| `filename` | `string` | 被删除文件的文件名 |
| `deleted` | `boolean` | 固定为 `true` |

## 代码示例

```bash
curl -X DELETE "http://127.0.0.1:58685/api/v1/text-files/9f2c1a8b7d6e5f4a3b2c1d0e9f8a7b6c"
```

## 成功响应示例

```json
{
  "code": 0,
  "message": "文件删除成功",
  "data": {
    "file_id": "9f2c1a8b7d6e5f4a3b2c1d0e9f8a7b6c",
    "filename": "example.txt",
    "deleted": true
  },
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

## 失败响应示例

### 404 文件不存在

```json
{
  "code": 40400,
  "message": "文件不存在：9f2c1a8b7d6e5f4a3b2c1d0e9f8a7b6c",
  "data": null,
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

## 注意事项

- 删除为物理删除，不可恢复。
- 通过项目硬删除或清空回收站触发的文件删除同样走此逻辑。