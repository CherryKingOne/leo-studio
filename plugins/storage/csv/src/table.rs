use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

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

    /// 原子落盘：先写入同目录下的临时文件，然后通过操作系统级 rename 原子替换。
    pub fn save(&self) -> CoreResult<()> {
        let parent = self.path.parent().unwrap_or_else(|| Path::new("."));
        let temp_path = parent.join(format!(".tmp_{}", uuid::Uuid::new_v4().simple()));

        let file = fs::File::create(&temp_path).map_err(|err| {
            CoreError::Storage(format!("创建临时 CSV 文件失败: {err}"))
        })?;

        let mut writer = csv::WriterBuilder::new()
            .has_headers(true)
            .from_writer(file);

        writer.write_record(&self.headers).map_err(|err| {
            let _ = fs::remove_file(&temp_path);
            CoreError::Storage(format!("写入表头失败: {err}"))
        })?;

        for record in &self.records {
            let row: Vec<&str> = self
                .headers
                .iter()
                .map(|header| record.get(header).map(|v| v.as_str()).unwrap_or(""))
                .collect();
            writer.write_record(&row).map_err(|err| {
                let _ = fs::remove_file(&temp_path);
                CoreError::Storage(format!("写入记录失败: {err}"))
            })?;
        }

        writer.flush().map_err(|err| {
            let _ = fs::remove_file(&temp_path);
            CoreError::Storage(format!("刷新缓冲区失败: {err}"))
        })?;

        drop(writer);

        fs::rename(&temp_path, &self.path).map_err(|err| {
            let _ = fs::remove_file(&temp_path);
            CoreError::Storage(format!("原子重命名 CSV 失败: {err}"))
        })?;

        Ok(())
    }
}
