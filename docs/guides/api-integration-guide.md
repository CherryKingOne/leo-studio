# 🌐 RESTful API 开发者集成指南 (HTTP API Reference)

Leo Studio 提供现代化、全异步的高性能 HTTP API，基于 Rust Axum 框架构建。无论你是开发前端 Web 工作台（React / Vue / Svelte），还是在企业内部系统中通过 Python / Java / Go / Node.js 远程集成，均可通过标准 RESTful 协议与 Leo Studio 通信。

---

## 1. 服务地址与基础约定

- **默认服务基础 URL**：`http://127.0.0.1:58685`
- **协议格式**：标准 HTTP / 1.1 与 HTTP / 2，请求头通常包含 `Content-Type: application/json`
- **字符编码**：强制 UTF-8 编码
- **跨域支持**：默认只允许本机来源（`http://localhost:*`、`http://127.0.0.1:*`、`http://[::1]:*`，如 `localhost:5173` / `localhost:3000`）；其它来源需通过 `LEO_CORS_ORIGINS` 加入白名单
- **鉴权**：默认不鉴权且仅监听本机；配置 `LEO_API_TOKEN` 后需携带 `Authorization: Bearer <token>`

---

## 2. API 端点详解

### 2.1 系统与健康检查

#### ① 健康检查
```http
GET /health
```
**响应示例 (`200 OK`)**：
```json
{
  "app": "leo-studio",
  "status": "ok",
  "version": "0.2.0"
}
```

---

### 2.2 插件与声线发现

#### ① 查询当前系统已激活插件列表
```http
GET /api/v2/plugins
```
**响应示例 (`200 OK`)**：
```json
{
  "code": "OK",
  "data": [
    {
      "id": "leo-plugin-importer-txt",
      "name": "TXT 纯文本小说导入插件",
      "version": "0.1.0",
      "description": "支持 .txt 格式纯文本小说解析，自动执行清洗分章并生成 Script AST。",
      "category": "importer",
      "capabilities": ["importer:txt"]
    },
    {
      "id": "leo-plugin-tts-edge",
      "name": "微软 Edge-TTS 语音合成",
      "version": "0.1.0",
      "description": "基于微软 Edge 神经语音的高品质免费 TTS 插件。",
      "category": "tts",
      "capabilities": ["tts:synthesize", "tts:list_voices"]
    }
  ]
}
```

#### ② 查询当前可用的 TTS 角色声线列表
```http
GET /api/v2/voices
```
**响应示例 (`200 OK`)**：
```json
{
  "code": "OK",
  "data": [
    {
      "id": "zh-CN-YunxiNeural",
      "name": "云希",
      "gender": "male",
      "locale": "zh-CN",
      "description": "活泼青年，适合玄幻修仙爽文男主"
    },
    {
      "id": "zh-CN-XiaoxiaoNeural",
      "name": "晓晓",
      "gender": "female",
      "locale": "zh-CN",
      "description": "清脆温婉，适合言情玄幻女主"
    }
  ]
}
```

---

### 2.3 剧本导入与导出

#### ① 导入文件并自动生成剧本 (`POST /api/v2/import`)
通过 `multipart/form-data` 上传原始小说文件（支持 `.txt`）。服务端按文件扩展名挑选已挂载的 `Importer` 插件（按注册顺序取第一个声明支持该扩展名的插件），没有插件支持该扩展名时返回 `415`。

```bash
curl -X POST http://127.0.0.1:58685/api/v2/import \
  -F "file=@测试.txt"
```

导入结果必须经已挂载的 `StorageProvider` 插件成功落盘才返回 `200`；未挂载存储插件或保存失败时返回 `500`（业务码 `50000`），不会再出现“接口成功但数据没保存”的情况。

**响应示例 (`200 OK`)**：
```json
{
  "code": "OK",
  "data": {
    "id": "8fa2514d-176f-47dc-9f06-e0f3175317ad",
    "title": "测试",
    "chapter_count": 5
  }
}
```

#### ② 导出剧本为台账或 AST (`GET /api/v2/export/{project_id}`)
- `format=csv`：导出为 Excel / 制作软件通用的配音台账 CSV
- `format=json`（默认）：导出为微内核完整的结构化 AST JSON
- 其它值：按名称查找已挂载的 `Exporter` 插件（名称即插件的 `format_name()`）；未注册的格式返回 `422` 并列出可用格式（此前会静默回退为 JSON）

```bash
# 导出 CSV 配音制作台账
curl -O -J "http://127.0.0.1:58685/api/v2/export/8fa2514d-176f-47dc-9f06-e0f3175317ad?format=csv"

# 导出 JSON 剧本文档
curl -O -J "http://127.0.0.1:58685/api/v2/export/8fa2514d-176f-47dc-9f06-e0f3175317ad?format=json"
```

---

### 2.4 剧目与章节管理

#### ① 分页查询剧目列表
```http
GET /api/v2/projects?page=1&page_size=20&keyword=合欢宗
```

#### ② 获取单个剧目详情
```http
GET /api/v2/projects/{project_id}
```

#### ③ 获取剧目章节大纲列表
```http
GET /api/v2/projects/{project_id}/chapters
```

#### ④ 获取章节正文与台词切片详情
```http
GET /api/v2/projects/{project_id}/chapters/{chapter_id}?include_sentences=true
```
**响应包含**：章节标题、自然段落、切分后的分句、每句包含的台词/旁白片段（`kind: dialogue | narration`）及音频引用状态。

---

### 2.5 章节语音合成与音频流获取

#### ① 触发章节批量语音合成
向微内核下发合成任务，系统将利用当前挂载的 `TTSProvider` 对本章中所有未合成的切片并发合成。

```http
POST /api/v2/projects/{project_id}/chapters/{chapter_id}/synthesize
Content-Type: application/json

{
  "voice": "zh-CN-YunxiNeural",
  "speed": 1.0,
  "pitch": 0.0
}
```

**缓存规则**：切片 ID 形如 `clip_{片段ID}_{参数指纹}`，指纹由 TTS 引擎、`voice`、`speed`、`pitch` 与片段文本共同计算。未传 `force: true` 时，只有这几项完全相同的已有切片才会被复用；换声线、调语速/音调或修改文本都会重新合成。旧版本生成的 `clip_{片段ID}.mp3` 不会再被命中。

**时长**：`duration_ms` 与 `total_duration_ms` 由 MP3 帧头实际计算（复用的缓存切片同样从音频本身计算并计入 `total_duration_ms`），不再是 1500ms 占位值。

**响应示例 (`200 OK`)**：
```json
{
  "code": 0,
  "data": {
    "chapter_id": "c1",
    "total_segments": 26,
    "newly_synthesized": 20,
    "skipped_count": 6,
    "total_duration_ms": 78200,
    "clips": [
      {
        "clip_id": "clip_s1_0_9f1c2a7b3d4e5f60",
        "segment_id": "s1_0",
        "text": "言罢，他一拳轰出。",
        "kind": "narration",
        "voice": "zh-CN-YunxiNeural",
        "duration_ms": 1824,
        "audio_url": "/api/v2/audio/clip_s1_0_9f1c2a7b3d4e5f60"
      }
    ]
  }
}
```

#### ② 试听或下载生成的音频切片
```http
GET /api/v2/audio/{clip_id}
```
- **响应头**：`Content-Type: audio/mpeg`
- **响应体**：纯 MP3 二进制音频字节流，支持流式边下边播或保存。

---

## 3. 前端集成范例 (JavaScript / TypeScript)

```typescript
// 示例：使用 fetch 调用 Leo Studio API
const BASE_URL = 'http://127.0.0.1:58685';

// 1. 查询可用声线
async function getVoices() {
  const res = await fetch(`${BASE_URL}/api/v2/voices`);
  const json = await res.json();
  return json.data;
}

// 2. 一键合成第一章
async function synthesizeChapter(projectId: string, chapterId: string, voice: string) {
  const res = await fetch(`${BASE_URL}/api/v2/projects/${projectId}/chapters/${chapterId}/synthesize`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ voice, speed: 1.0, pitch: 0.0 }),
  });
  return await res.json();
}
```
