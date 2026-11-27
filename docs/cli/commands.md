# CLI 命令手册

## 数据目录

- `--data-dir`：数据根目录，默认当前目录。
- `--csv-dir`：可选 CSV 目录，默认 `<data-dir>/.leo-studio`。
- v3 管理命令统一使用 CSV Store，与 REST/MCP 共用 Service。
- `import/export/voices/synthesize` 继续使用 AST StorageHub 生产流水线。

## 项目管理

```bash
leo-cli project create --name "示例项目" --file ./example.txt
leo-cli project list --page 1 --page-size 20 --status "制作中"
leo-cli project get <project_id>
leo-cli project update <project_id> --name "新名称" --status "已完成"
leo-cli project delete <project_id> --confirm
```

## 项目章节

```bash
leo-cli project chapters list <project_id> --keyword "第一章"
leo-cli project chapters append <project_id> --file ./new.txt --dry-run
leo-cli project chapters append <project_id> --file ./new.txt --dry-run=false --confirm
leo-cli project chapters get <project_id> <chapter_id>
leo-cli project chapters delete <project_id> <chapter_id> --confirm
```

## 项目台词

```bash
leo-cli project dialogues list <project_id> <chapter_id> \
  --binding-status unbound \
  --segment-type dialogue \
  --page 1 \
  --page-size 20
leo-cli project dialogues get <project_id> <chapter_id> <dialogue_id>
leo-cli project dialogues bind <project_id> <chapter_id> <dialogue_id> \
  --speaker-id <character_id> \
  --binding-source manual
leo-cli project dialogues unbind <project_id> <chapter_id> <dialogue_id>
```

不提供批量绑定或批量解绑命令；批量操作可在脚本或前端中循环调用单条命令。

## 项目角色

```bash
leo-cli project characters create <project_id> \
  --name "林冲" \
  --alias "豹子头" \
  --gender male \
  --role-type protagonist \
  --tag "武将"

leo-cli project characters list <project_id> --status all --keyword "林"
leo-cli project characters get <project_id> <character_id>
leo-cli project characters update <project_id> <character_id> \
  --status archived \
  --clear-age \
  --clear-tags
leo-cli project characters delete <project_id> <character_id> --confirm
```

## 项目多音字

```bash
leo-cli project polyphones chapter-list <project_id> <chapter_id>
leo-cli project polyphones sentence-list <project_id> <chapter_id> <sentence_id>
leo-cli project polyphones save <project_id> <chapter_id> <sentence_id> \
  --annotations-file ./annotations.json
leo-cli project polyphones clear <project_id> <chapter_id> <sentence_id> --confirm
leo-cli project polyphones delete-item <project_id> <chapter_id> <sentence_id> 1 --confirm
```

`annotations.json` 示例：

```json
[
  {
    "char_index": 1,
    "character": "行",
    "pinyin": "xíng",
    "pinyin_base": "xing",
    "tone": 2,
    "tts_pinyin": "xing2"
  }
]
```

## 回收站

```bash
leo-cli recycle-bin list
leo-cli recycle-bin restore <project_id>
leo-cli recycle-bin purge <project_id> --confirm
leo-cli recycle-bin clear --confirm
```

## 旧 AST 生产命令

```bash
leo-cli import ./example.txt --title "示例项目"
leo-cli export <project_id> --format csv -o ./export.csv
leo-cli voices
leo-cli synthesize <project_id> --chapter 0 --voice zh-CN-YunxiNeural
```

这些命令操作 AST 存储，不与 `project` 命令的 CSV Store 混用。
