# 预览文本文件内容

读取文本文件全文内容并返回，供在线预览使用。

| 属性 | 值 |
| --- | --- |
| **接口地址** | `GET /api/v1/text-files/{file_id}/content` |
| **所属模块** | 文本文件（`src/modules/text_files/`） |
| **请求格式** | 无请求体 |
| **响应格式** | `application/json` |
| **认证方式** | 无 |
| **幂等性** | 是 |

## 接口描述

优先读取磁盘上的原始文件并按指定编码解码；若磁盘文件缺失，则回退为
`novel_contents.csv` 中的章节内容拼接（章节标题 + 正文，以空行分隔）。
返回前统一换行符为 `\n`。

## 请求参数

### Path

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `file_id` | `string` | **是** | 文件唯一标识 |

### Query

| 名称 | 类型 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- | --- |
| `encoding` | `string` | 否 | `utf-8` | 解码编码，支持 `utf-8` / `utf8` / `gb18030` / `gbk` |

### Header

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `X-Request-ID` | `string` | 否 | 自定义请求追踪 ID |

## 响应字段

### `data` 对象（`TextFileContentResponse`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `file_id` | `string` | 文件唯一标识 |
| `filename` | `string` | 文件名 |
| `content_type` | `string` | MIME 类型 |
| `encoding` | `string` | 实际使用的解码编码（回显请求值） |
| `size` | `integer` | 文件大小（字节） |
| `content` | `string` | 文本全文内容 |

## 代码示例

```bash
curl -s "http://127.0.0.1:58685/api/v1/text-files/9f2c1a8b7d6e5f4a3b2c1d0e9f8a7b6c/content?encoding=utf-8"
```

## 成功响应示例

```json
{
  "code": 0,
  "message": "success",
  "data": {
    "file_id": "9f2c1a8b7d6e5f4a3b2c1d0e9f8a7b6c",
    "filename": "example.txt",
    "content_type": "text/plain",
    "encoding": "utf-8",
    "size": 2048,
    "content": "第一章 标题\n正文内容……"
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

### 422 编码不支持

```json
{
  "code": 40000,
  "message": "文件不是合法的 gbk 文本",
  "data": null,
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

## 注意事项

- 当前内置扩展名仅 `.txt`，全部支持预览。
- 若请求 `encoding` 不在支持列表内，会返回 `422` 且提示不是合法的对应编码文本。