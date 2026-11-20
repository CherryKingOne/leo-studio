//! 原子落盘工具：临时文件 + fsync + rename，并支持多个文件“两阶段”提交。
//!
//! 单个文件：先在目标文件同目录写入唯一命名的临时文件并 `fsync`，再 `rename` 覆盖目标，
//! 最后 `fsync` 所在目录，保证掉电后要么是旧内容、要么是完整的新内容，不会出现空文件或半截文件。
//!
//! 多个文件：先把所有文件都写成临时文件（[`stage_file`]），全部成功后才依次 `rename`（[`commit_all`]）。
//! 任何一个文件在写入阶段失败，所有临时文件都会被删除，磁盘上的旧数据一个都不会被改动。
//! `rename` 阶段本身不是跨文件原子的（POSIX 没有多文件事务），但只剩下几次元数据操作的极短窗口。

use std::fs::{self, File, OpenOptions};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

/// 已写入并 fsync、尚未替换目标文件的临时文件。未提交就被丢弃时会自动删除临时文件。
#[derive(Debug)]
pub struct PendingWrite {
    temp: PathBuf,
    target: PathBuf,
    committed: bool,
}

impl PendingWrite {
    pub fn target(&self) -> &Path {
        &self.target
    }

    pub fn temp_path(&self) -> &Path {
        &self.temp
    }
}

impl Drop for PendingWrite {
    fn drop(&mut self) {
        if !self.committed {
            let _ = fs::remove_file(&self.temp);
        }
    }
}

/// 把内容写入 `target` 同目录下的唯一临时文件并 fsync，返回待提交的写入。
pub fn stage_file(
    target: &Path,
    write: impl FnOnce(&mut BufWriter<File>) -> io::Result<()>,
) -> io::Result<PendingWrite> {
    let parent = parent_dir(target);
    fs::create_dir_all(parent)?;
    let file_name = target
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("data");
    let temp = parent.join(format!(".{file_name}.{}.tmp", uuid::Uuid::new_v4().simple()));

    let file = OpenOptions::new().write(true).create_new(true).open(&temp)?;
    // 先构造 PendingWrite：后续任何一步失败，Drop 都会清理临时文件
    let pending = PendingWrite {
        temp,
        target: target.to_path_buf(),
        committed: false,
    };
    let mut writer = BufWriter::new(file);
    write(&mut writer)?;
    writer.flush()?;
    let file = writer.into_inner().map_err(|err| err.into_error())?;
    file.sync_all()?;
    Ok(pending)
}

/// 依次把临时文件 rename 为目标文件，然后 fsync 涉及的目录。
pub fn commit_all(pending: Vec<PendingWrite>) -> io::Result<()> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    for mut write in pending {
        fs::rename(&write.temp, &write.target)?;
        write.committed = true;
        let dir = parent_dir(&write.target).to_path_buf();
        if !dirs.contains(&dir) {
            dirs.push(dir);
        }
    }
    for dir in dirs {
        sync_dir(&dir)?;
    }
    Ok(())
}

/// 单文件原子写入：stage + commit。
pub fn write_atomic(
    target: &Path,
    write: impl FnOnce(&mut BufWriter<File>) -> io::Result<()>,
) -> io::Result<()> {
    commit_all(vec![stage_file(target, write)?])
}

fn parent_dir(path: &Path) -> &Path {
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    }
}

/// fsync 目录，使 rename 本身也持久化。非 Unix 平台上目录不能被打开同步，直接忽略。
fn sync_dir(dir: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        File::open(dir)?.sync_all()
    }
    #[cfg(not(unix))]
    {
        let _ = dir;
        Ok(())
    }
}
