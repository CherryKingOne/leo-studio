# 下载文本文件

下载指定文本文件的原始二进制内容。

| 属性 | 值 |
| --- | --- |
| **接口地址** | `GET /api/v1/text-files/{file_id}/download` |
| **所属模块** | 文本文件（`src/modules/text_files/`） |
| **请求格式** | 无请求体 |
| **响应格式** | 文件二进制流（非 JSON） |
| **认证方式** | 无 |
| **幂等性** | 是 |

## 接口描述

以附件形式返回文件原始字节。响应头 `Content-Type` 取文件 MIME 类型，
`Content-Disposition` 为 `attachment`，并同时提供 ASCII 回退文件名与
UTF-8 编码文件名（`filename*=UTF-8''...`），以兼容中文文件名。

## 请求参数

### Path

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `file_id` | `string` | **是** | 文件唯一标识 |

### Header

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `X-Request-ID` | `string` | 否 | 自定义请求追踪 ID |

## 响应头

| 名称 | 说明 |
| --- | --- |
| `Content-Type` | 文件 MIME 类型，如 `text/plain` |
| `Content-Disposition` | `attachment; filename="..."; filename*=UTF-8''...` |
| `X-Request-ID` | 请求追踪 ID |

响应体为文件原始字节，不使用 `ApiResponse` 包装。

## 代码示例

### curl

```bash
curl -s -O -J "http://127.0.0.1:58685/api/v1/text-files/9f2c1a8b7d6e5f4a3b2c1d0e9f8a7b6c/download"
```

### Python

```python
import httpx

resp = httpx.get(
    "http://127.0.0.1:58685/api/v1/text-files/9f2c1a8b7d6e5f4a3b2c1d0e9f8a7b6c/download"
)
with open("downloaded.txt", "wb") as fp:
    fp.write(resp.content)
```

## 成功响应示例

`HTTP/1.1 200 OK`

```http
Content-Type: text/plain
Content-Disposition: attachment; filename="example.txt"; filename*=UTF-8''example.txt
X-Request-ID: b3f1c2d4e5a60718293a4b5c6d7e8f90

<文件原始字节>
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

- 该接口是少数不返回统一 JSON 包装的接口之一。
- 若元信息存在但磁盘文件已丢失，返回 `404`。