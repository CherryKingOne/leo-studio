use std::collections::HashMap;
use std::path::PathBuf;

use crate::error::{AppError, AppResult, storage_io};

/// 一条 CSV 记录：列名到字符串值的映射。
pub type Record = HashMap<String, String>;

/// 读取字符串字段，缺失时返回空串。
pub fn str_field<'a>(record: &'a Record, key: &str) -> &'a str {
    record.get(key).map(|value| value.as_str()).unwrap_or("")
}

/// 读取整型字段，缺失或非法时返回 0。
pub fn int_field(record: &Record, key: &str) -> i64 {
    str_field(record, key).trim().parse::<i64>().unwrap_or(0)
}

/// 读取布尔字段，兼容 1/true/yes/on。
pub fn bool_field(record: &Record, key: &str) -> bool {
    matches!(
        str_field(record, key).trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn csv_error(context: &str, err: impl std::fmt::Display) -> AppError {
    AppError::storage(format!("{context}：{err}"))
}

/// 内存 CSV 表：进程内全量加载，变更后原子落盘。
#[derive(Debug, Clone)]
pub struct CsvTable {
    path: PathBuf,
    headers: Vec<String>,
    records: Vec<Record>,
}

impl CsvTable {
    /// 打开（或创建）一张 CSV 表。
    ///
    /// 若文件已存在则读取全部记录；文件表头与默认表头取并集，
    /// 缺失列以空字符串补齐。
    pub fn open(path: PathBuf, default_headers: &[&str]) -> AppResult<Self> {
        let mut headers: Vec<String> = default_headers.iter().map(|h| h.to_string()).collect();
        let mut records = Vec::new();

        if path.exists() {
            let mut reader = csv::ReaderBuilder::new()
                .has_headers(true)
                .flexible(true)
                .from_path(&path)
                .map_err(|err| csv_error("读取 CSV 文件失败", err))?;

            let file_headers: Vec<String> = reader
                .headers()
                .map_err(|err| csv_error("解析 CSV 表头失败", err))?
                .iter()
                .map(|h| h.to_string())
                .collect();

            for header in &file_headers {
                if !headers.iter().any(|item| item == header) {
                    headers.push(header.clone());
                }
            }

            for result in reader.records() {
                let record = result.map_err(|err| csv_error("解析 CSV 记录失败", err))?;
                let mut map = Record::new();
                for (index, header) in file_headers.iter().enumerate() {
                    map.insert(header.clone(), record.get(index).unwrap_or("").to_string());
                }
                for header in &headers {
                    map.entry(header.clone()).or_default();
                }
                records.push(map);
            }
        }

        Ok(Self {
            path,
            headers,
            records,
        })
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    pub fn headers(&self) -> &[String] {
        &self.headers
    }

    pub fn find_all(&self) -> &[Record] {
        &self.records
    }

    pub fn find_by(&self, key: &str, value: &str) -> Option<&Record> {
        self.records
            .iter()
            .find(|record| str_field(record, key) == value)
    }

    pub fn insert(&mut self, record: Record) {
        self.records.push(record);
    }

    pub fn insert_batch(&mut self, records: Vec<Record>) {
        self.records.extend(records);
    }

    pub fn update_by(&mut self, key: &str, value: &str, updates: &[(&str, String)]) {
        for record in self.records.iter_mut() {
            if str_field(record, key) == value {
                for (column, new_value) in updates {
                    record.insert(column.to_string(), new_value.clone());
                }
            }
        }
    }

    pub fn delete_by(&mut self, key: &str, value: &str) {
        self.records
            .retain(|record| str_field(record, key) != value);
    }

    pub fn clear(&mut self) {
        self.records.clear();
    }

    /// 原子落盘：先写临时文件，再替换目标文件。
    pub fn save(&self) -> AppResult<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|err| storage_io("创建 CSV 目录失败", err))?;
        }

        let tmp_path = self.path.with_extension("tmp");
        {
            let mut writer = csv::Writer::from_path(&tmp_path)
                .map_err(|err| csv_error("创建 CSV 临时文件失败", err))?;
            writer
                .write_record(&self.headers)
                .map_err(|err| csv_error("写入 CSV 表头失败", err))?;
            for record in &self.records {
                let row: Vec<String> = self
                    .headers
                    .iter()
                    .map(|header| record.get(header).cloned().unwrap_or_default())
                    .collect();
                writer
                    .write_record(&row)
                    .map_err(|err| csv_error("写入 CSV 记录失败", err))?;
            }
            writer
                .flush()
                .map_err(|err| storage_io("刷新 CSV 缓冲失败", err))?;
        }

        std::fs::rename(&tmp_path, &self.path)
            .map_err(|err| storage_io("替换 CSV 文件失败", err))?;
        Ok(())
    }
}
