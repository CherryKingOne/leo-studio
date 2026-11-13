# 快速开始

本文介绍如何以 **API Server 模式**启动 Leo Studio 后端，并完成第一次 v2 接口调用。

## 1. 启动 API Server

```bash
cd backend
cargo run --release
```

启动后日志输出监听地址，默认：

```text
Leo Studio API 已启动：http://0.0.0.0:58685
```

可通过环境变量覆盖监听地址与端口：

```bash
LEO_HOST=127.0.0.1 LEO_PORT=58685 cargo run --release
```

## 2. 健康检查

```bash
curl http://127.0.0.1:58685/api/v2/health
```

响应：

```json
{
  "status": "ok",
  "app": "Leo Studio API",
  "version": "0.1.0",
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

## 3. 服务索引

```bash
curl http://127.0.0.1:58685/api/v2
```

返回应用名、版本、模块清单与业务前缀。

## 4. 创建第一个项目

v2 不暴露独立的文本文件接口，项目创建时直接以 `multipart/form-data` 上传小说文件：

```bash
curl -X POST "http://127.0.0.1:58685/api/v2/projects" \
  -F "name=示例项目" \
  -F "file=@./example.txt"
```

响应 `data.project_id` 即项目标识，后续章节、多音字等接口均基于该 ID。

## 5. 查询项目列表

```bash
curl "http://127.0.0.1:58685/api/v2/projects?page=1&page_size=20"
```

## 6. 请求追踪

每个请求都会生成 `request_id`：

- 若请求头携带 `X-Request-ID`，则沿用该值；
- 否则由服务端生成 UUID v4。

响应体与响应头 `X-Request-ID` 均会返回该值，便于日志排查。

## 7. 数据目录

默认数据文件位于：

```text
.leo-studio/
├── novels.csv
├── novel_contents.csv
├── novel_sentences.csv
├── novel_polyphones.csv
└── projects.csv
storage/uploads/text-files/
```

可通过 `LEO_DATA_DIR`、`LEO_CSV_DIR`、`LEO_UPLOAD_DIR` 调整。

## 下一步

- [API v2 总览](./README.md)
- [项目接口](./projects/create.md)
- [章节接口](./chapters/list.md)
- [多音字标注接口](./polyphones/list.md)