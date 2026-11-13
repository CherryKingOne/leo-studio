# 后端目录结构（Rust）

Leo Studio 后端已从 Python / FastAPI 完整迁移为 **Rust + Axum**。
前端 `desktop/` 未改动，仍通过 `API_BASE_URL` 访问后端，接口契约保持兼容。

## 目录树

```text
backend/
├── Cargo.toml                     # 包与依赖（axum / tokio / fancy-regex / csv / chrono）
├── Cargo.lock                     # 依赖锁定
├── .env.example                   # 环境变量样例（默认端口 58685）
├── .gitignore                     # 忽略 target、.env、临时文件
├── README.md                      # 后端使用说明
├── STRUCTURE.md                   # 本文件：目录结构与代码风格
├── src/
│   ├── main.rs                    # 进程入口：加载 .env、初始化日志/存储、启动监听
│   ├── lib.rs                     # 库入口，导出各层模块，供集成测试复用
│   ├── app.rs                     # 路由装配、CORS、请求体上限、共享状态
│   ├── config.rs                  # 环境变量配置（LEO_ / LEO_TEXT_FILES_）
│   ├── error.rs                   # AppError、业务错误码、统一错误响应
│   ├── http.rs                    # 通用分页参数与分页计算
│   ├── middleware.rs              # X-Request-ID 请求头与响应体注入
│   ├── response.rs                # 统一响应包装 ApiResponse<T>
│   ├── state.rs                   # AppState：配置 + 线程安全存储
│   ├── modules/                   # 功能模块（按业务边界拆分）
│   │   ├── mod.rs
│   │   ├── system.rs              # /health、/api/v1/system/health、/
│   │   ├── projects/              # 项目、章节、追加上传、多音字标注
│   │   │   ├── mod.rs
│   │   │   ├── router.rs          # 路由与 multipart / JSON 入参解析
│   │   │   ├── service.rs         # 项目生命周期、章节合并、标注校验
│   │   │   ├── schema.rs          # 请求体与响应 DTO
│   │   │   └── model.rs           # 领域模型（ProjectInfo / ChapterContentInfo 等）
│   │   └── text_files/            # 文本文件上传、解析、下载、切句查询
│   │       ├── mod.rs
│   │       ├── router.rs
│   │       ├── service.rs
│   │       ├── schema.rs
│   │       └── model.rs
│   ├── storage/                   # 持久化层
│   │   ├── mod.rs
│   │   ├── csv_table.rs           # 通用 CSV 表：加载、表头迁移、原子落盘
│   │   └── store.rs               # 五张业务表 + 目录管理 + 统一保存
│   └── text/                      # 纯文本处理（无 I/O）
│       ├── mod.rs
│       ├── kaomoji.rs             # 颜文字识别与清洗（结构识别 + 强信号校验）
│       ├── parsers.rs             # 章节切分、文件名解析、拼音/中文数字
│       └── processor.rs           # 行级切句与切句记录构造
├── docs/api/                      # API 契约文档（已更新为 Axum / 58685）
│   ├── README.md
│   ├── getting-started.md
│   ├── configuration.md
│   ├── errors.md
│   ├── projects/
│   └── text-files/
└── tests/
    ├── common/mod.rs              # 测试应用装配、multipart 请求构造
    ├── api.rs                     # 接口集成测试（上传、项目、追加、多音字、回收站）
    └── parsers.rs                 # 颜文字清洗、章节切分等纯函数测试
```

## 分层与职责

```text
HTTP 请求
  │
  ▼
router.rs        解析 Path / Query / Multipart / Json，只做参数提取与状态码映射
  │
  ▼
service.rs       业务规则：校验、章节合并、级联删除、拼音推导
  │
  ├──► text/     纯函数：清洗、章节切分、切句、拼音
  └──► storage/  内存表 + CSV 原子落盘
  │
  ▼
ApiResponse<T>   统一 { code, message, data, request_id }
```

- **router 不写业务规则**：只做协议解析与响应包装。
- **service 不直接拼 HTTP**：返回 `AppResult<T>`，由 `AppError` 统一转换状态码。
- **text 层无 I/O**：便于单元测试与复用。
- **storage 层统一事务边界**：一次业务操作内的多表写入集中在 service，最后调用 `Store::save_*`。

## 数据存储

| 表 | 文件 | 作用 |
| --- | --- | --- |
| `novels` | `.leo-studio/novels.csv` | 小说元信息 |
| `contents` | `.leo-studio/novel_contents.csv` | 章节标题、正文、字数 |
| `sentences` | `.leo-studio/novel_sentences.csv` | 章节切句 |
| `polyphones` | `.leo-studio/novel_polyphones.csv` | 句子多音字标注 |
| `projects` | `.leo-studio/projects.csv` | 项目元信息与回收站标记 |

- 上传文件位于 `storage/uploads/text-files/`。
- CSV 采用「进程内全量 + 变更后临时文件原子替换」，兼容旧版 Python 生成的 UTF-8 CSV。
- 表头缺失新增列时自动补齐。

## API 一览

| 方法 | 路径 | 说明 |
| --- | --- | --- |
| GET | `/health`、`/api/v1/system/health` | 健康检查 |
| GET | `/` | 服务与模块索引 |
| POST | `/api/v1/text-files/upload` | 上传文本文件 |
| GET | `/api/v1/text-files` | 文件分页列表 |
| GET | `/api/v1/text-files/{file_id}` | 文件详情 |
| GET | `/api/v1/text-files/{file_id}/content` | 文本内容预览 |
| GET | `/api/v1/text-files/{file_id}/download` | 下载原文件 |
| DELETE | `/api/v1/text-files/{file_id}` | 删除文件 |
| GET | `/api/v1/text-files/{file_id}/sentences` | 查询切句 |
| POST | `/api/v1/projects`、`/api/v1/projects/upload` | 创建项目 |
| GET | `/api/v1/projects` | 项目分页列表 |
| GET | `/api/v1/projects/recycle-bin` | 回收站列表 |
| DELETE | `/api/v1/projects/recycle-bin` | 清空回收站 |
| GET | `/api/v1/projects/{project_id}` | 项目详情 |
| PATCH / PUT | `/api/v1/projects/{project_id}` | 重命名 |
| DELETE | `/api/v1/projects/{project_id}` | 软删除 |
| POST | `/api/v1/projects/{project_id}/restore` | 恢复 |
| DELETE | `/api/v1/projects/{project_id}/hard` | 物理删除 |
| GET | `/api/v1/projects/{project_id}/chapters` | 章节大纲 |
| GET | `/api/v1/projects/{project_id}/chapters/{chapter_id}` | 章节内容（含切句与多音字） |
| POST | `/api/v1/projects/{project_id}/chapters/upload` | 追加上传章节 |
| GET / PUT / DELETE | `/api/v1/projects/{project_id}/sentences/{sentence_id}/polyphones` | 多音字标注查询 / 覆盖保存 / 清空 |

## 代码风格

- **Rust 2024 edition**，`cargo fmt` + `cargo clippy` 约束格式与常见错误。
- 命名使用 `snake_case` / `CamelCase`，模块按业务能力拆分，不出现以 `_` 开头的函数或类型。
- 业务错误统一使用 `AppError`，禁止在请求路径上 `unwrap`；解析失败、校验失败均返回带业务码的结构化响应。
- 时间统一为 UTC，序列化为 RFC 3339；UUID 使用 UUID v4 的 32 位无连字符字符串，与前端约定一致。
- 文本清洗规则与旧版 Python 保持行为一致（颜文字、分隔线、省略号、章节标题、中文数字归一化）。
- 集成测试使用 `tower::ServiceExt::oneshot` 直接驱动 Router，临时目录隔离数据，不依赖外部服务。

## 代码规模

| 区域 | 文件 | 行数 |
| --- | --- | --- |
| 入口与装配 | `main.rs` / `app.rs` / `lib.rs` | 约 100 |
| 基础设施 | `config` / `error` / `http` / `middleware` / `response` / `state` | 约 300 |
| projects 模块 | `router` / `service` / `schema` / `model` | 约 1430 |
| text_files 模块 | `router` / `service` / `schema` / `model` | 约 930 |
| 存储层 | `csv_table` / `store` | 约 420 |
| 文本处理 | `kaomoji` / `parsers` / `processor` | 约 1170 |
| 测试 | `api.rs` / `parsers.rs` / `common` | 约 750 |

## 端口与启动

默认端口 **58685**，可通过 `LEO_PORT` 覆盖。

```bash
cd backend
cargo run --release
# 监听 http://0.0.0.0:58685
```

前端 `desktop/.env.local`：

```bash
API_BASE_URL=http://127.0.0.1:58685
```