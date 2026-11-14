# 配置说明

Leo Studio 后端服务的全部配置项均可通过**环境变量**或 `backend/.env` 文件覆盖。
配置入口位于 `backend/src/config.rs`。

## 配置分层

| 层级 | 环境变量前缀 | 作用范围 |
| --- | --- | --- |
| 全局 | `LEO_` | 应用级通用配置 |
| 文本文件模块 | `LEO_TEXT_FILES_` | 单个功能模块的专属配置 |

模块级配置优先于全局配置。例如 `LEO_TEXT_FILES_MAX_UPLOAD_SIZE_MB`
会覆盖 `LEO_MAX_UPLOAD_SIZE_MB`。

## 全局配置

| 配置项 | 环境变量 | 类型 | 默认值 | 说明 |
| --- | --- | --- | --- | --- |
| `app_name` | `LEO_APP_NAME` | `string` | `Leo Studio API` | 服务名称 |
| `app_version` | `LEO_APP_VERSION` | `string` | `0.1.0` | 服务版本号 |
| `debug` | `LEO_DEBUG` | `boolean` | `true` | 调试模式 |
| `cors_origins` | `LEO_CORS_ORIGINS` | `list` | `*` | 允许跨域的来源，逗号分隔 |
| `max_upload_size_mb` | `LEO_MAX_UPLOAD_SIZE_MB` | `integer` | `50` | 全局上传大小上限（MB） |
| `host` | `LEO_HOST` | `string` | `0.0.0.0` | 监听地址 |
| `port` | `LEO_PORT` | `integer` | `58685` | 监听端口 |
| `data_dir` | `LEO_DATA_DIR` | `path` | `.` | 数据根目录，相对路径基于进程工作目录解析 |

## 文本文件模块配置

配置前缀 `LEO_TEXT_FILES_`：

| 配置项 | 环境变量 | 类型 | 默认值 | 说明 |
| --- | --- | --- | --- | --- |
| `allowed_extensions` | `LEO_TEXT_FILES_ALLOWED_EXTENSIONS` | `list` | `.txt` | 允许上传的扩展名白名单，逗号分隔 |
| `max_upload_size_mb` | `LEO_TEXT_FILES_MAX_UPLOAD_SIZE_MB` | `integer \| null` | 沿用全局 | 模块级大小上限（MB） |
| `upload_dir` | `LEO_UPLOAD_DIR` | `path` | `<data_dir>/storage/uploads/text-files` | 上传文件目录 |
| `csv_dir` | `LEO_CSV_DIR` | `path` | `<data_dir>/.leo-studio` | CSV 数据目录 |
| `novels_csv_filename` | `LEO_TEXT_FILES_NOVELS_CSV_FILENAME` | `string` | `novels.csv` | 小说元数据表 |
| `novel_contents_csv_filename` | `LEO_TEXT_FILES_NOVEL_CONTENTS_CSV_FILENAME` | `string` | `novel_contents.csv` | 章节内容表 |
| `novel_sentences_csv_filename` | `LEO_TEXT_FILES_NOVEL_SENTENCES_CSV_FILENAME` | `string` | `novel_sentences.csv` | 切句表 |
| `novel_dialogues_csv_filename` | `LEO_TEXT_FILES_NOVEL_DIALOGUES_CSV_FILENAME` | `string` | `novel_dialogues.csv` | 台词切分表 |
| `novel_polyphones_csv_filename` | `LEO_TEXT_FILES_NOVEL_POLYPHONES_CSV_FILENAME` | `string` | `novel_polyphones.csv` | 多音字标注表 |
| `max_filename_length` | `LEO_TEXT_FILES_MAX_FILENAME_LENGTH` | `integer` | `255` | 文件名最大长度 |

## 使用示例

```bash
LEO_PORT=58685 \
LEO_DATA_DIR=./data \
LEO_CSV_DIR=./data/csv \
LEO_UPLOAD_DIR=./data/uploads \
LEO_TEXT_FILES_ALLOWED_EXTENSIONS=.txt \
cargo run --release --bin leo-studio-backend
# 开发模式：cargo run --bin leo-studio-backend
```

## 存储说明

- CSV 表在进程内常驻内存，每次变更后以临时文件 + 原子替换方式落盘。
- 首次启动会自动创建缺失的目录与表头；旧表头会自动补齐新增列。
- 上传目录与 CSV 目录可指向任意可写路径，不要求与源码同目录。