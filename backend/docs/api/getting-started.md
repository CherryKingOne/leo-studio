# 快速开始

本文档介绍如何在本地启动 Leo Studio 后端服务并完成第一次文本文件上传。

## 环境要求

| 组件 | 版本要求 | 说明 |
| --- | --- | --- |
| Rust | `>= 1.85`（edition 2024） | 建议使用 `rustup` 安装 stable 工具链 |
| 系统 C 工具链 | `cc` / `build-essential` | 部分依赖需要链接 C 运行时 |

## 构建与启动

```bash
cd backend
cargo build --release

# 默认监听 0.0.0.0:58685，数据目录为项目根目录
cargo run --release --bin leo-studio-backend
# 开发模式
cargo run --bin leo-studio-backend
```

启动成功后可访问：

| 地址 | 说明 |
| --- | --- |
| <http://127.0.0.1:58685/health> | 健康检查 |
| <http://127.0.0.1:58685/api/v1/system/health> | 带版本前缀的健康检查 |
| <http://127.0.0.1:58685/> | 服务与模块索引 |

## 配置

后端所有配置通过环境变量或 `backend/.env` 覆盖，详见 [配置说明](configuration.md)。

常用示例：

```bash
LEO_HOST=127.0.0.1
LEO_PORT=58685
LEO_DATA_DIR=./
LEO_CSV_DIR=./.leo-studio
LEO_UPLOAD_DIR=./storage/uploads/text-files
LEO_MAX_UPLOAD_SIZE_MB=50
LEO_TEXT_FILES_ALLOWED_EXTENSIONS=.txt
```

## 第一次上传

```bash
curl -X POST http://127.0.0.1:58685/api/v1/text-files/upload \
  -F "file=@测试.txt;type=text/plain" \
  -F "description=测试小说"
```

```json
{
  "code": 0,
  "message": "文件上传成功",
  "data": {
    "file_id": "…",
    "filename": "测试.txt",
    "size": 7340,
    "extension": ".txt",
    "category": "plain_text",
    "download_url": "/api/v1/text-files/…/download"
  },
  "request_id": "…"
}
```

## 前端联调

前端 Next.js 应用位于 `desktop/`，通过环境变量指定后端地址：

```bash
# desktop/.env.local
API_BASE_URL=http://127.0.0.1:58685
```

前端启动：

```bash
cd desktop
npm install
npm run dev
```

后端已默认开启 CORS，允许前端跨域访问。

## 数据与测试

- 数据目录默认与项目根目录一致，CSV 表位于 `.leo-studio/`，上传文件位于 `storage/uploads/text-files/`。
- 运行测试：

```bash
cd backend
cargo test
```