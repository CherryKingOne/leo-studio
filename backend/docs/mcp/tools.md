# MCP 工具契约

- 协议版本：`2026-07-28`
- 命名：统一 `leo_` 前缀，参数 `snake_case`
- 返回：所有成功结果同时包含 `content[0].text`（中文摘要或 JSON 文本）与 `structuredContent`（与 `outputSchema` 一致）
- 错误：业务/参数错误返回 `isError=true` 的工具结果，`structuredContent` 为 `{ "code": "INVALID_PARAMETER", "message": "..." }`；未知工具返回协议错误 `-32602`

## 通用参数

| 参数 | 说明 |
| --- | --- |
| `page` | 页码，默认 1 |
| `page_size` | 每页数量，默认 20，上限 100 |
| `project_id` | 剧目 ID |
| `chapter_id` | 章节 ID |
| `sentence_id` | 句子 ID |
| `max_chars` | 正文字符上限，默认 20,000；超出会截断并返回 `truncated: true` |
| `confirm` | 破坏性工具必须为 `true` |

## 只读工具（默认开启）

| 工具 | 参数 | 返回 |
| --- | --- | --- |
| `leo_health` | — | `status / server / app / version / protocol_versions / write_tools / destructive_tools` |
| `leo_list_projects` | `page, page_size, status, keyword, sort, order` | `{ total, page, page_size, pages, items[] }` |
| `leo_get_project` | `project_id` | `ProjectInfo` |
| `leo_list_chapters` | `project_id, keyword?, limit?` | `{ project_id, total, items[] }` |
| `leo_get_chapter` | `project_id, chapter_id, include_sentences?, max_chars?` | `ChapterContentInfo + truncated` |
| `leo_search_chapters` | `project_id, keyword, limit?` | `{ project_id, total, items[] }`，命中项含 `excerpt` |
| `leo_list_recycle_bin` | `page, page_size` | `{ total, page, page_size, pages, items[] }` |
| `leo_get_sentence_polyphones` | `project_id, sentence_id` | `{ sentence_id, total, items[] }` |
| `leo_list_chapter_polyphones` | `project_id, chapter_id` | `ChapterPolyphoneList` |
| `leo_get_stats` | — | `{ projects, chapters, words, recycle_bin }` |

## 写入工具（默认启用，`LEO_MCP_WRITE_TOOLS=false` 可关闭）

| 工具 | 参数 | 说明 |
| --- | --- | --- |
| `leo_create_project` | `name`, `text` | 新建剧目，自动清洗与分章 |
| `leo_append_chapters` | `project_id`, `text`, `overwrite_existing=false`, `dry_run=true`, `confirm=false` | `dry_run=false` 时必须 `confirm=true`；默认不覆盖同名章节 |
| `leo_rename_project` | `project_id, name` | 重命名 |
| `leo_update_project` | `project_id, name?, status?` | 更新名称/状态 |
| `leo_update_chapter` | `project_id, chapter_id, title?, content?, confirm?` | 改标题/正文；改正文需 `confirm=true`，会重建切句并清空该章多音字 |
| `leo_save_sentence_polyphones` | `project_id, sentence_id, annotations[]` | 全量覆盖保存；`annotations[]` 元素为 `{ char_index, character, pinyin, pinyin_base?, tone?, tts_pinyin? }` |
| `leo_clear_sentence_polyphones` | `project_id, sentence_id` | 清空某句标注 |
| `leo_restore_project` | `project_id` | 从回收站恢复 |

`leo_append_chapters` 返回 `ProjectChapterAppendInfo`：

```json
{
  "project_id": "…",
  "dry_run": true,
  "added_chapters": 0,
  "skipped_chapters": 2,
  "total_chapters": 2,
  "has_conflicts": false,
  "conflicts": [],
  "chapters": [{ "chapter_title": "第一章", "action": "skipped" }]
}
```

## 破坏性工具（默认启用 + `confirm=true`；`LEO_MCP_DESTRUCTIVE_TOOLS=false` 可关闭）

| 工具 | 参数 | 说明 |
| --- | --- | --- |
| `leo_delete_chapter` | `project_id, chapter_id, confirm` | 删除章节、切句与标注，刷新统计 |
| `leo_move_to_trash` | `project_id, confirm` | 软删除，可从回收站恢复 |
| `leo_purge_project` | `project_id, confirm` | 物理删除剧目、文件与章节，不可恢复 |
| `leo_empty_recycle_bin` | `confirm` | 清空回收站，不可恢复 |

## 工具注解

| 工具 | `readOnlyHint` | `destructiveHint` | `idempotentHint` |
| --- | --- | --- | --- |
| 只读 | `true` | `false` | `true` |
| 创建/追更 | `false` | `false` | `false` |
| 重命名/更新/标注 | `false` | `false` | `true` |
| 删除/清空 | `false` | `true` | `true` |

## 典型调用序列

```text
leo_health
leo_list_projects { keyword: "都市" }
leo_get_project { project_id }
leo_list_chapters { project_id }
leo_get_chapter { project_id, chapter_id, max_chars: 8000 }
leo_search_chapters { project_id, keyword: "银行" }
leo_append_chapters { project_id, text, dry_run: true }
leo_append_chapters { project_id, text, dry_run: false, confirm: true }
```

## 错误映射

| 场景 | 结果 |
| --- | --- |
| 参数缺失/类型错误 | 工具执行错误，`code=INVALID_PARAMETER` |
| 剧目/章节不存在 | 工具执行错误，`code=NOT_FOUND` |
| 需要确认但未传 `confirm` | 工具执行错误，`code=INVALID_PARAMETER` |
| 未知工具名 | 协议错误 `-32602` |
| 不支持的协议版本 | `-32022`，`data.supported=["2026-07-28"]` |
| 缺少协议头/头不一致 | `-32020` |