# Leo Studio

前后端分离的小说创作工作台。

- `desktop/`：Next.js 前端，保持不变。
- `backend/`：Rust + Axum 后端，默认监听 `58685`。
- `docs/solutions/`：历史方案文档。

## 目录结构

```text
.
├── desktop/                 # Next.js 前端（未改动）
├── backend/                 # Rust / Axum 后端
│   ├── src/
│   │   ├── main.rs          # 进程入口
│   │   ├── app.rs           # 路由、CORS、共享状态
│   │   ├── config.rs        # 环境变量配置
│   │   ├── error.rs         # 业务错误与统一错误响应
│   │   ├── http.rs          # 分页
│   │   ├── middleware.rs    # request_id
│   │   ├── response.rs      # 统一响应包装
│   │   ├── state.rs         # AppState
│   │   ├── modules/         # system / text_files / projects
│   │   ├── storage/         # CSV 表与仓储
│   │   └── text/            # 颜文字清洗、章节切分、切句、拼音
│   ├── docs/api/            # 接口文档
│   ├── tests/               # 集成测试与解析器测试
│   ├── Cargo.toml
│   ├── README.md
│   └── STRUCTURE.md         # 详细目录结构与代码风格
├── docs/
└── README.md
```

后端代码已从 Python / FastAPI 完整迁移为 Rust，仓库中不再保留 Python 后端源码。

## 启动

```bash
# 后端
cd backend
cargo run --release
# http://127.0.0.1:58685

# 前端
cd desktop
npm install
npm run dev
```

前端 `desktop/.env.local`：

```bash
API_BASE_URL=http://127.0.0.1:58685
```

## 接口

- 健康检查：`GET /health`
- 文本文件：`POST /api/v1/text-files/upload`
- 项目：`POST /api/v1/projects`
- 项目章节：`GET /api/v1/projects/{project_id}/chapters`
- 多音字标注：`PUT /api/v1/projects/{project_id}/sentences/{sentence_id}/polyphones`

完整接口契约见 [backend/docs/api](backend/docs/api)。

## 测试

```bash
cd backend
cargo test
```

## 代码风格

Rust 2024 edition；业务模块按功能边界拆分；请求路径禁止 `unwrap`；
统一 `AppError` 错误码；时间使用 UTC RFC 3339；详细约定见
[backend/STRUCTURE.md](backend/STRUCTURE.md)。