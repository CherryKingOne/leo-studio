use std::fs;
use std::path::Path;

use leo_core::ast::{ChapterNode, ParagraphNode, ScriptDocument, SegmentKind, SegmentNode, SentenceNode};
use leo_core::plugin::StorageProvider;
use leo_plugin_storage_csv::atomic::write_atomic;
use leo_plugin_storage_csv::{CsvStorageService, CsvTable, Record, save_tables};
use tempfile::tempdir;

fn temp_files(dir: &Path) -> Vec<String> {
    fs::read_dir(dir)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .filter(|name| name.ends_with(".tmp"))
        .collect()
}

fn table_with(path: &Path, value: &str) -> CsvTable {
    let mut table = CsvTable::open(path.to_path_buf(), &["id", "value"]).unwrap();
    let mut record = Record::new();
    record.insert("id".into(), "1".into());
    record.insert("value".into(), value.into());
    table.replace_records(vec![record]);
    table
}

#[test]
fn write_atomic_replaces_file_and_leaves_no_temp_files() {
    let dir = tempdir().unwrap();
    let target = dir.path().join("data.csv");
    write_atomic(&target, |out| std::io::Write::write_all(out, b"v1")).unwrap();
    write_atomic(&target, |out| std::io::Write::write_all(out, b"v2")).unwrap();
    assert_eq!(fs::read_to_string(&target).unwrap(), "v2");
    assert!(temp_files(dir.path()).is_empty());

    // 写入过程出错：目标文件保持旧内容，临时文件被清理
    let err = write_atomic(&target, |out| {
        std::io::Write::write_all(out, b"half")?;
        Err(std::io::Error::other("模拟磁盘已满"))
    });
    assert!(err.is_err());
    assert_eq!(fs::read_to_string(&target).unwrap(), "v2");
    assert!(temp_files(dir.path()).is_empty());
}

#[test]
fn multi_table_save_changes_nothing_when_one_table_fails() {
    let dir = tempdir().unwrap();
    let first_path = dir.path().join("first.csv");
    table_with(&first_path, "old").save().unwrap();
    let before = fs::read_to_string(&first_path).unwrap();

    // 第二张表打开后，把它的父目录换成一个普通文件，写临时文件必然失败
    let blocker = dir.path().join("blocker");
    let first = table_with(&first_path, "new");
    let second = table_with(&blocker.join("second.csv"), "new");
    fs::remove_dir_all(&blocker).unwrap();
    fs::write(&blocker, b"not a dir").unwrap();

    assert!(save_tables(&[&first, &second]).is_err());
    assert_eq!(
        fs::read_to_string(&first_path).unwrap(),
        before,
        "任一表写入失败时，已成功写好临时文件的表也不能被替换"
    );
    assert!(temp_files(dir.path()).is_empty(), "失败后不应残留临时文件");

    // 全部成功时两张表都更新
    let second = table_with(&dir.path().join("second.csv"), "new");
    save_tables(&[&first, &second]).unwrap();
    assert!(fs::read_to_string(&first_path).unwrap().contains("new"));
    assert!(fs::read_to_string(dir.path().join("second.csv")).unwrap().contains("new"));
    assert!(temp_files(dir.path()).is_empty());
}

fn script(id: &str, title: &str) -> ScriptDocument {
    let mut script = ScriptDocument::new(id.into(), title.into(), "作者".into(), "仙侠".into());
    let mut chapter = ChapterNode::new(format!("{id}_c1"), 1, "ch_1".into(), "第一章".into());
    let mut paragraph = ParagraphNode {
        id: format!("{id}_p1"),
        index: 0,
        sentences: Vec::new(),
    };
    let mut sentence = SentenceNode::new(format!("{id}_s1"), 0, "他说。".into());
    sentence
        .segments
        .push(SegmentNode::new(format!("{id}_g1"), 0, SegmentKind::Narration, "他说。".into()));
    paragraph.sentences.push(sentence);
    chapter.paragraphs.push(paragraph);
    script.chapters.push(chapter);
    script.recalculate_totals();
    script
}

#[cfg(unix)]
#[tokio::test]
async fn failed_save_script_keeps_memory_and_disk_unchanged() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempdir().unwrap();
    let storage = CsvStorageService::new(dir.path().to_path_buf()).unwrap();
    storage.save_script(&script("book", "旧书名")).await.unwrap();

    let table_dir = dir.path().join("scripts");
    let novels_before = fs::read_to_string(table_dir.join("novels.csv")).unwrap();
    fs::set_permissions(&table_dir, fs::Permissions::from_mode(0o555)).unwrap();
    // root 不受目录权限限制，这种环境下无法模拟写入失败
    let probe = table_dir.join("probe");
    if fs::write(&probe, b"x").is_ok() {
        let _ = fs::remove_file(&probe);
        fs::set_permissions(&table_dir, fs::Permissions::from_mode(0o755)).unwrap();
        eprintln!("当前用户可写只读目录（可能是 root），跳过该用例");
        return;
    }

    let result = storage.save_script(&script("book", "新书名")).await;
    fs::set_permissions(&table_dir, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(result.is_err(), "目录不可写时保存必须失败");

    // 内存未被提前修改：读回的仍是磁盘上的旧版本
    let loaded = storage.load_script("book").await.unwrap().unwrap();
    assert_eq!(loaded.title, "旧书名");
    assert_eq!(fs::read_to_string(table_dir.join("novels.csv")).unwrap(), novels_before);
    assert!(temp_files(&table_dir).is_empty());

    // 恢复后可以正常保存
    storage.save_script(&script("book", "新书名")).await.unwrap();
    assert_eq!(storage.load_script("book").await.unwrap().unwrap().title, "新书名");
}
