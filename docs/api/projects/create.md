# 创建项目并上传小说

以 `multipart/form-data` 方式创建新剧目/项目并上传小说文本文件。服务端自动完成小说清洗、分章、切句与项目台账建立。

| 属性 | 值 |
| --- | --- |
| **接口地址** | `POST /api/v1/projects`（别名：`POST /api/v1/projects/upload`） |
| **所属模块** | 剧目与项目管理（`app/modules/projects/`） |
| **请求格式** | `multipart/form-data` |
| **响应格式** | `application/json` |
| **认证方式** | 无（当前版本未接入鉴权） |
| **幂等性** | 否，每次调用生成新的 `project_id` 与 `file_id` |

## 接口描述

创建新的有声书/广播剧制作项目。本接口将项目元数据与小说正文解析紧密关联，服务端依次完成：

1. **项目入参校验** —— 校验项目名称 `name`（或兼容别名 `title` / `project_name`）与上传文件 `file` 是否有效。
2. **复用文本模块处理** —— 直接调用 `TextFileStorageService.save_file` 完成：
   - 文件名安全清洗与扩展名校验（仅允许合法纯文本/电子书）；
   - 1 MB 分块流式落盘与大小超限防护；
   - 颜文字（Kaomoji）与无意义占位行深度清洗；
   - 智能识别章节标题切分章节，并在章节内按行切句；
   - 分别持久化至 `.leo-studio/novels.csv`、`novel_contents.csv` 与 `novel_sentences.csv`。
3. **项目台账落盘** —— 生成 32 位唯一 `project_id`，设置初始状态为 `制作中`，并将项目元信息写入 `.leo-studio/projects.csv`。

## 请求参数

### Header

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `Content-Type` | `string` | 是 | 固定为 `multipart/form-data; boundary=...` |
| `X-Request-ID` | `string` | 否 | 自定义请求追踪 ID，未传则由服务端生成 |

### Form Data

| 名称 | 类型 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- | --- |
| **`name`** | `string` | **是** | — | 项目名称 |
| **`file`** | `binary` | **是** | — | 上传文件（`.txt` 文本文件） |

> **注意**：请求入参严格限定为**项目名称 (`name`)** 与**上传文件 (`file`)**，不接受任何其他表单参数。

## 响应字段

### `data` 对象（`ProjectResponse`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `project_id` | `string` | 项目唯一标识，标准 UUID（UUID4，非雪花 ID） |
| `name` | `string` | 项目名称 |
| `file_id` | `string` | 关联的文本文件唯一标识，标准 UUID（UUID4，非雪花 ID） |
| `filename` | `string` | 原始清洗后文件名 |
| `status` | `string` | 制作状态（初始为 `制作中`，导出后为 `母带就绪`） |
| `chapter_count` | `integer` | 自动识别切分出的章节总数 |
| `total_words` | `integer` | 小说有效总字数（非空白字符） |
| `created_at` | `string` | 项目创建时间，ISO 8601 格式（UTC） |
| `updated_at` | `string` | 最后更新时间，ISO 8601 格式（UTC） |

## 成功示例

### 请求

```bash
curl -X POST "http://127.0.0.1:8000/api/v1/projects" \
  -F "name=诡秘之主：第一部 愚者" \
  -F "file=@./诡秘之主.txt"
```

```python
import httpx

with open("诡秘之主.txt", "rb") as fp:
    response = httpx.post(
        "http://127.0.0.1:8000/api/v1/projects",
        data={"name": "诡秘之主：第一部 愚者"},
        files={"file": ("诡秘之主.txt", fp, "text/plain")},
    )

result = response.json()
print("创建项目成功，ID:", result["data"]["project_id"])
```

### 响应

```json
{
  "code": 0,
  "message": "项目创建成功",
  "data": {
    "project_id": "9f8c12a4b3e2489c91d8e5f2a1b3c4d5",
    "name": "诡秘之主：第一部 愚者",
    "file_id": "a1b2c3d4e5f67890123456789abcdef0",
    "filename": "诡秘之主.txt",
    "status": "制作中",
    "chapter_count": 12,
    "total_words": 45820,
    "created_at": "2026-10-27T08:55:00+00:00",
    "updated_at": "2026-10-27T08:55:00+00:00"
  },
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

## 失败示例

### 缺少项目名称

```json
{
  "code": 40000,
  "message": "项目名称不能为空",
  "data": null,
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

### 上传空文件

```json
{
  "code": 40000,
  "message": "上传文件内容为空",
  "data": null,
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```
