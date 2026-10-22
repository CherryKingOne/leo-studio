# 下载文本文件

根据 `file_id` 下载文件内容。

| 属性 | 值 |
| --- | --- |
| **接口地址** | `GET /api/v1/text-files/{file_id}/download` |
| **所属模块** | 文本文件管理（`app/modules/text_files/`） |
| **请求格式** | 无请求体 |
| **响应格式** | 二进制流（`Content-Type` 与上传时一致） |
| **认证方式** | 无（当前版本未接入鉴权） |
| **幂等性** | 是（只读操作） |

## 接口描述

读取文件内容并以二进制流返回。与其他业务接口不同，本接口**不使用统一 JSON 包装**，
直接返回文件原始字节。

响应头 `Content-Disposition` 使用 `attachment` 模式并携带原始文件名，
浏览器会提示下载而非直接内联渲染。

若只想在线查看文本内容而非下载文件，请使用[内容预览接口](./get-text-file-content.md)。

## 路径参数

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `file_id` | `string` | **是** | 文件唯一标识，32 位十六进制字符串 |

## 响应说明

### Header

| 名称 | 类型 | 说明 |
| --- | --- | --- |
| `Content-Type` | `string` | 与上传时提交的 MIME 类型一致，默认 `text/plain` |
| `Content-Length` | `integer` | 文件大小，单位字节 |
| `Content-Disposition` | `string` | 形如 `attachment; filename="notes.md"`（非 ASCII 文件名使用 RFC 5987 编码） |
| `X-Request-ID` | `string` | 请求追踪 ID |
| `ETag` | `string` | 内容标识（由服务端基于文件内容与修改时间计算） |

### Body

文件原始字节流，无 JSON 包装、无 `code` / `message` 字段。

## 成功示例

### 请求

```bash
curl -X GET "http://127.0.0.1:8000/api/v1/text-files/9f2c1a8b7d6e5f4a3b2c1d0e9f8a7b6c/download" \
  -o notes.md
```

```python
import httpx

file_id = "9f2c1a8b7d6e5f4a3b2c1d0e9f8a7b6c"

# 直接保存到本地文件
with httpx.stream("GET", f"http://127.0.0.1:8000/api/v1/text-files/{file_id}/download") as r:
    r.raise_for_status()
    with open("notes.md", "wb") as fp:
        for chunk in r.iter_bytes():
            fp.write(chunk)

# 或直接读取到内存
response = httpx.get(f"http://127.0.0.1:8000/api/v1/text-files/{file_id}/download")
content = response.content
print(response.headers["content-type"], len(content))
```

### 响应

`HTTP/1.1 200 OK`

```
Content-Type: text/markdown
Content-Length: 2048
Content-Disposition: attachment; filename="notes.md"
ETag: "a47f2317729d7c48b0b0fd766f7d99b9"
X-Request-ID: b3f1c2d4e5a60718293a4b5c6d7e8f90

# 项目笔记 ...（文件原始字节）
```

## 错误响应

错误场景仍返回统一的 JSON 包装结构。

### `404` 文件不存在

`file_id` 不在索引中，或磁盘文件已被外部删除。

```json
{
  "code": 40400,
  "message": "文件不存在：9f2c1a8b7d6e5f4a3b2c1d0e9f8a7b6c",
  "data": null,
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

> **注意**：即使 `Content-Type` 声明为二进制，成功与失败响应的
> `Content-Type` 都会是 `application/json`（错误场景），
> 客户端应根据 HTTP 状态码判断是否解析 JSON。

## 注意事项

- **强制下载**：始终使用 `attachment` 模式，即使文件是 Markdown 或 JSON 也不会内联展示。
- **中文文件名**：原始文件名为中文时，`Content-Disposition` 使用 RFC 5987 编码，
  形如 `attachment; filename*=utf-8''%E4%B8%AD%E6%96%87%E6%8A%A5%E5%91%8A.md`，
  现代浏览器均可正确解析并还原原始文件名。
- **性能**：文件通过 `FileResponse` 以流式方式发送，不会将整个文件读入内存。
- **Range 请求**：当前版本未实现分片下载（`Range` / `206 Partial Content`），
  大文件下载场景需自行处理断点续传需求。

## 相关接口

- [上传单个文本文件](./upload-text-file.md) —— 获取 `file_id` 的来源
- [查询文本文件详情](./get-text-file.md) —— 获取文件的完整元信息
- [预览文本文件内容](./get-text-file-content.md) —— 以 JSON 返回文本内容
