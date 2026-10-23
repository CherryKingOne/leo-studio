# 预览文本文件内容

根据 `file_id` 读取文本文件内容，并以 JSON 形式返回解码后的文本。

| 属性 | 值 |
| --- | --- |
| **接口地址** | `GET /api/v1/text-files/{file_id}/content` |
| **所属模块** | 文本文件管理（`app/modules/text_files/`） |
| **请求格式** | 无请求体 |
| **响应格式** | `application/json` |
| **认证方式** | 无（当前版本未接入鉴权） |
| **幂等性** | 是（只读操作） |

## 接口描述

这是文本文件模块特有的**在线预览**接口：服务端读取文件内容并按指定字符集解码，
以 JSON 字符串返回，前端可直接渲染到编辑器或代码高亮组件中。

与[下载接口](./download.md)的区别：

| 场景 | 推荐接口 |
| --- | --- |
| 需要原始字节流（保存到本地、交给其它程序处理） | [下载接口](./download.md) |
| 需要在页面上直接展示文本内容 | 本接口 |

> **注意**：本接口仅支持纯文本类型 `.txt` 与 `.md`。
> `.doc` `.docx` `.epub` 为二进制文档格式，调用本接口会返回 `422`，
> 请改用[下载接口](./download.md)获取原始文件。

## 路径参数

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `file_id` | `string` | **是** | 文件唯一标识，32 位十六进制字符串 |

## 请求参数

### Query

| 名称 | 类型 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- | --- |
| `encoding` | `string` | 否 | `utf-8` | 解码使用的字符集，如 `utf-8`、`gbk`、`latin-1` |

## 响应字段

### `data` 对象（`TextFileContentResponse`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `file_id` | `string` | 文件唯一标识 |
| `filename` | `string` | 清洗后的原始文件名 |
| `content_type` | `string` | MIME 类型，按扩展名推导 |
| `encoding` | `string` | 实际使用的解码字符集 |
| `size` | `integer` | 文件大小，单位字节 |
| `content` | `string` | 解码后的文本内容 |

## 成功示例

### 请求

```bash
curl -X GET "http://127.0.0.1:8000/api/v1/text-files/9f2c1a8b7d6e5f4a3b2c1d0e9f8a7b6c/content"
```

```python
import httpx

file_id = "9f2c1a8b7d6e5f4a3b2c1d0e9f8a7b6c"
response = httpx.get(f"http://127.0.0.1:8000/api/v1/text-files/{file_id}/content")

data = response.json()["data"]
print(data["encoding"], data["size"])
print(data["content"])
```

### 响应

`HTTP/1.1 200 OK`

```json
{
  "code": 0,
  "message": "success",
  "data": {
    "file_id": "9f2c1a8b7d6e5f4a3b2c1d0e9f8a7b6c",
    "filename": "notes.md",
    "content_type": "text/markdown",
    "encoding": "utf-8",
    "size": 17,
    "content": "# 项目笔记\n\nHello Leo Studio"
  },
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

## 错误响应

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

### `422` 字符集不匹配或解码失败

文件内容无法按指定字符集解码时返回。常见于使用 GBK 编码的文件却以 UTF-8 解码，
此时可尝试 `?encoding=gbk` 重新请求。

```json
{
  "code": 40000,
  "message": "文件不是合法的 utf-8 文本：'gbk' codec can't decode byte 0xb2 in position 0: illegal multibyte sequence",
  "data": null,
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

### `422` 二进制文档不支持预览

`.doc` `.docx` `.epub` 不是可直接解码的纯文本格式，调用本接口会被拒绝，
请改用[下载接口](./download.md)。

```json
{
  "code": 40000,
  "message": ".epub 为二进制文档格式，不支持文本内容预览，请使用下载接口",
  "data": null,
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

## 注意事项

- **整体读入内存**：本接口会把整个文件读入内存后解码，
  超大文件建议改用[下载接口](./download.md)流式处理。
- **换行符**：Python 的 `read_text` 会做通用换行转换，
  `CRLF` 会被规范化为 `\n`，返回内容与磁盘字节不完全一致。
- **字符集**：默认按 UTF-8 解码；服务端不做编码嗅探，
  非 UTF-8 文件需显式指定 `encoding` 参数。

## 相关接口

- [上传单个文本文件](./upload.md) —— 获取 `file_id` 的来源
- [下载文本文件](./download.md) —— 以二进制流下载
- [查询文本文件详情](./file.md) —— 获取文件的完整元信息