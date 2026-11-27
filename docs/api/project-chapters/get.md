# 查询项目章节内容

| 属性 | 值 |
| --- | --- |
| 接口地址 | `GET /api/v3/projects/{project_id}/chapters/{chapter_id}` |
| 请求格式 | Path |
| 响应格式 | `application/json` |
| 认证方式 | 无 |

## 请求参数

### Path

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `project_id` | `string` | 是 | 项目唯一标识 |
| `chapter_id` | `string` | 是 | 章节唯一标识 |

## 返回字段

### `data`（`ChapterContentInfo`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `project_id` | `string` | 项目唯一标识 |
| `file_id` | `string` | 关联文本文件标识 |
| `chapter_id` | `string` | 章节唯一标识 |
| `chapter_index` | `integer` | 章节序号，从 `0` 开始 |
| `chapter_title` | `string` | 章节标题 |
| `word_count` | `integer` | 章节字数 |
| `content` | `string` | 章节完整正文 |
| `sentences` | `object[]` | 切句列表 |

### `sentences[]`（`ChapterSentenceInfo`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `sentence_id` | `string` | 句子唯一标识 |
| `sentence_index` | `integer` | 句子序号 |
| `content` | `string` | 句子内容 |
| `word_count` | `integer` | 句子字数 |
| `polyphones` | `object[]` | 该句子的多音字标注 |

### `sentences[].polyphones[]`（`SentencePolyphoneInfo`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `polyphone_id` | `string` | 标注唯一标识 |
| `sentence_id` | `string` | 句子唯一标识 |
| `char_index` | `integer` | 字符在句子中的偏移量，从 `0` 开始 |
| `character` | `string` | 被标注字符 |
| `pinyin` | `string` | 原始拼音 |
| `pinyin_base` | `string` | 不带声调的拼音 |
| `tone` | `integer` | 声调 |
| `tts_pinyin` | `string` | TTS 使用拼音 |
| `created_at` | `string` | 创建时间 |
| `updated_at` | `string` | 更新时间 |

## curl

```bash
curl -s "http://127.0.0.1:58685/api/v3/projects/9f8c12a4b3e2489c91d8e5f2a1b3c4d5/chapters/c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8"
```

## 常见错误

| HTTP | 业务码 | 场景 |
| --- | --- | --- |
| 404 | 40400 | 项目不存在、章节不存在或章节不属于该项目 |
