use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::atomic::{PendingWrite, commit_all, stage_file};

use leo_core::error::{CoreError, CoreResult};

pub type Record = HashMap<String, String>;

pub fn str_field<'a>(record: &'a Record, key: &str) -> &'a str {
    record.get(key).map(|v| v.as_str()).unwrap_or("")
}

pub fn int_field(record: &Record, key: &str) -> i64 {
    str_field(record, key).trim().parse::<i64>().unwrap_or(0)
}

pub fn bool_field(record: &Record, key: &str) -> bool {
    matches!(
        str_field(record, key).trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

#[derive(Debug, Clone)]
pub struct CsvTable {
    path: PathBuf,
    headers: Vec<String>,
    records: Vec<Record>,
}

impl CsvTable {
    pub fn open(path: PathBuf, default_headers: &[&str]) -> CoreResult<Self> {
        let mut headers: Vec<String> = default_headers.iter().map(|h| h.to_string()).collect();
        let mut records = Vec::new();

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                CoreError::Storage(format!("创建目录 {:?} 失败: {e}", parent))
            })?;
        }

        if path.exists() {
            let mut reader = csv::ReaderBuilder::new()
                .has_headers(true)
                .flexible(true)
                .from_path(&path)
                .map_err(|err| CoreError::Storage(format!("读取 CSV {:?} 失败: {err}", path)))?;

            let file_headers: Vec<String> = reader
                .headers()
                .map_err(|err| CoreError::Storage(format!("解析表头失败: {err}")))?
                .iter()
                .map(|h| h.to_string())
                .collect();

            for header in &file_headers {
                if !headers.iter().any(|item| item == header) {
                    headers.push(header.clone());
                }
            }

            for result in reader.records() {
                let rec = result.map_err(|err| CoreError::Storage(format!("解析记录失败: {err}")))?;
                let mut map = Record::new();
                for (index, header) in file_headers.iter().enumerate() {
                    map.insert(header.clone(), rec.get(index).unwrap_or("").to_string());
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

    pub fn records(&self) -> &[Record] {
        &self.records
    }

    pub fn records_mut(&mut self) -> &mut Vec<Record> {
        &mut self.records
    }

    pub fn replace_records(&mut self, new_records: Vec<Record>) {
        self.records = new_records;
    }

    /// 用新的记录集构造一张同路径、同表头的表（不修改当前表），
    /// 便于先落盘、成功后再替换内存中的表。
    pub fn with_records(&self, records: Vec<Record>) -> Self {
        Self {
            path: self.path.clone(),
            headers: self.headers.clone(),
            records,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 把整张表写成同目录下的临时文件并 fsync，返回待提交的写入（尚未替换目标文件）。
    pub fn stage(&self) -> CoreResult<PendingWrite> {
        stage_file(&self.path, |out| {
            let mut writer = csv::WriterBuilder::new().has_headers(true).from_writer(out);
            writer.write_record(&self.headers).map_err(io::Error::other)?;
            for record in &self.records {
                let row: Vec<&str> = self
                    .headers
                    .iter()
                    .map(|header| record.get(header).map(|v| v.as_str()).unwrap_or(""))
                    .collect();
                writer.write_record(&row).map_err(io::Error::other)?;
            }
            writer.flush()
        })
        .map_err(|err| CoreError::Storage(format!("写入 CSV {:?} 失败: {err}", self.path)))
    }

    /// 原子落盘：临时文件 + fsync + rename + 目录 fsync。
    pub fn save(&self) -> CoreResult<()> {
        save_tables(&[self])
    }
}

/// 多表两阶段落盘：先把每张表都写成临时文件并 fsync，全部成功后才逐个 rename 替换。
/// 任一表写入失败时所有临时文件都会被清理，磁盘上的旧表保持不变。
pub fn save_tables(tables: &[&CsvTable]) -> CoreResult<()> {
    let mut pending = Vec::with_capacity(tables.len());
    for table in tables {
        pending.push(table.stage()?);
    }
    commit_all(pending).map_err(|err| CoreError::Storage(format!("提交 CSV 文件失败: {err}")))
}
