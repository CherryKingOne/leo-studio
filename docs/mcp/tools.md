# MCP 工具目录

本文档对应当前 v3 MCP 实现。所有工具都提供明确的输入 Schema、输出 Schema 和 annotations。

## 工具类型

| 类型 | 说明 |
| --- | --- |
| `read_only` | 只读查询，可自动调用 |
| `write` | 创建或更新数据 |
| `destructive` | 删除或清空数据，必须 `confirm=true`，且服务端开启破坏性工具 |

## 只读工具

| 工具 | 主要参数 | 返回 |
| --- | --- | --- |
| `leo_health` | 无 | 服务健康、版本、工具开关 |
| `leo_list_projects` | `page`, `page_size`, `status?`, `keyword?`, `sort?`, `order?` | `ProjectListResponse` |
| `leo_get_project` | `project_id` | `ProjectInfo` |
| `leo_list_chapters` | `project_id`, `keyword?`, `limit?` | `ChapterOutlineListResponse` |
| `leo_get_chapter` | `project_id`, `chapter_id`, `include_sentences?`, `max_chars?` | `ChapterContentInfo + truncated` |
| `leo_search_chapters` | `project_id`, `keyword`, `limit?` | 章节搜索命中列表 |
| `leo_list_recycle_bin` | `page?`, `page_size?` | `ProjectListResponse` |
| `leo_list_chapter_polyphones` | `project_id`, `chapter_id` | `ChapterPolyphoneList` |
| `leo_get_sentence_polyphones` | `project_id`, `chapter_id`, `sentence_id` | `SentencePolyphoneListResponse` |
| `leo_get_stats` | 无 | `ProjectStats` |
| `leo_list_characters` | `project_id`, 分页与过滤参数 | `CharacterListResponse` |
| `leo_get_character` | `project_id`, `character_id` | `CharacterInfo` |

## 写入工具

| 工具 | 主要参数 | 返回 |
| --- | --- | --- |
| `leo_create_project` | `name`, `text` | `ProjectInfo` |
| `leo_update_project` | `project_id`, `name?`, `status?` | `ProjectInfo` |
| `leo_append_chapters` | `project_id`, `text`, `overwrite_existing?`, `dry_run?`, `confirm?` | `ProjectChapterAppendInfo` |
| `leo_update_chapter` | `project_id`, `chapter_id`, `title?`, `content?`, `confirm?` | `ChapterContentInfo` |
| `leo_create_character` | `project_id`, `name`, 角色可选字段 | `CharacterInfo` |
| `leo_update_character` | `project_id`, `character_id`, 角色可选字段 | `CharacterInfo` |
| `leo_save_sentence_polyphones` | `project_id`, `chapter_id`, `sentence_id`, `annotations` | `SentencePolyphoneListResponse` |
| `leo_restore_project` | `project_id` | `ProjectRestoreResponse` |

## 破坏性工具

| 工具 | 主要参数 | 返回 |
| --- | --- | --- |
| `leo_delete_chapter` | `project_id`, `chapter_id`, `confirm` | `ChapterDeleteInfo` |
| `leo_move_to_trash` | `project_id`, `confirm` | `ProjectDeleteResponse` |
| `leo_purge_project` | `project_id`, `confirm` | `ProjectHardDeleteResponse` |
| `leo_empty_recycle_bin` | `confirm` | `ProjectClearRecycleBinResponse` |
| `leo_clear_sentence_polyphones` | `project_id`, `chapter_id`, `sentence_id`, `confirm` | `SentencePolyphoneDeleteResponse` |
| `leo_delete_sentence_polyphone` | `project_id`, `chapter_id`, `sentence_id`, `char_index`, `confirm` | `SentencePolyphoneItemDeleteInfo` |
| `leo_delete_character` | `project_id`, `character_id`, `confirm` | `CharacterDeleteResponse` |

## 角色字段类型

| 字段 | 类型 | 可选值 |
| --- | --- | --- |
| `name` | `string` | 必填 |
| `aliases` | `string[]` | 最多 20 个 |
| `gender` | `string` | `male` / `female` / `unknown` / `other` |
| `age` | `string/null` | `null` 表示清空 |
| `role_type` | `string` | `protagonist` / `supporting` / `antagonist` / `narrator` / `other` |
| `status` | `string` | `active` / `archived` |
| `description` | `string` | 最多 2000 字 |
| `tags` | `string[]` | 最多 20 个 |

## 资源

- `leo://projects`
- `leo://projects/{project_id}`
- `leo://projects/{project_id}/chapters`
- `leo://projects/{project_id}/chapters/{chapter_id}`
- `leo://projects/{project_id}/characters`
- `leo://projects/{project_id}/characters/{character_id}`

## 提示词

- `leo_project_overview`
- `leo_import_novel`
- `leo_character_overview`
- `leo_chapter_digest`
