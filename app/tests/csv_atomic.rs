use std::fs;
use std::path::Path;

use leo_app::storage::csv_table::{CsvTable, Record, save_tables};

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
    table.clear();
    let mut record = Record::new();
    record.insert("id".into(), "1".into());
    record.insert("value".into(), value.into());
    table.insert(record);
    table
}

#[test]
fn rest_store_tables_save_atomically_together() {
    let dir = tempfile::tempdir().unwrap();
    let novels = dir.path().join("novels.csv");
    table_with(&novels, "old").save().unwrap();
    let before = fs::read_to_string(&novels).unwrap();

    // 第二张表无法写入：两阶段提交下第一张表也不能被替换
    let blocker = dir.path().join("blocker");
    fs::write(&blocker, b"file").unwrap();
    let first = table_with(&novels, "new");
    let second = table_with(&blocker.join("contents.csv"), "new");
    assert!(save_tables(&[&first, &second]).is_err());
    assert_eq!(fs::read_to_string(&novels).unwrap(), before);
    assert!(temp_files(dir.path()).is_empty());

    // 旧实现使用固定的 novels.tmp 临时文件名，这里确认既不会留下也不会冲突
    fs::write(dir.path().join("novels.tmp"), b"stale").unwrap();
    let second = table_with(&dir.path().join("contents.csv"), "new");
    save_tables(&[&first, &second]).unwrap();
    assert!(fs::read_to_string(&novels).unwrap().contains("new"));
    assert!(fs::read_to_string(dir.path().join("contents.csv")).unwrap().contains("new"));
    assert_eq!(fs::read(dir.path().join("novels.tmp")).unwrap(), b"stale");
    assert_eq!(temp_files(dir.path()), vec!["novels.tmp".to_string()]);
}
