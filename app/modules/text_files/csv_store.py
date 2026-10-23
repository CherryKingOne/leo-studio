"""CSV 数据存储驱动：使用 CSV 文件进行持久化存储。

提供与关系型数据库表类似的 insert / find_by / find_all / update_by / delete_by 操作，
负责加锁并发保护与文件大小安全限制。
"""

from __future__ import annotations

import csv
import sys
import threading
from collections.abc import Callable
from pathlib import Path
from typing import Any


def configure_field_size_limit() -> None:
    """调大 CSV 单字段解析大小限制，以容纳大型长篇小说全文。"""
    max_limit = sys.maxsize
    while True:
        try:
            csv.field_size_limit(max_limit)
            break
        except OverflowError:
            max_limit = int(max_limit / 10)


configure_field_size_limit()


class CsvTable:
    """CSV 表格持久化管理器。

    用法示例::

        table = CsvTable(
            path=".leo-studio/novels.csv",
            columns=["id", "file_id", "filename", "size", "uploaded_at"],
        )
        table.insert({"id": "abc", "filename": "test.txt", ...})
        row = table.find_by("id", "abc")
        rows = table.find_all()
        table.delete_by("id", "abc")
    """

    def __init__(self, path: Path | str, columns: list[str]) -> None:
        self.path = Path(path)
        self.columns = columns
        self.lock = threading.Lock()
        self.ensure_file()

    def serialize_record(self, record: dict[str, Any]) -> dict[str, str]:
        """将记录字段值统一序列化为字符串。"""
        serialized: dict[str, str] = {}
        for col in self.columns:
            val = record.get(col)
            if val is None:
                serialized[col] = ""
            else:
                serialized[col] = str(val)
        return serialized

    def ensure_file_unlocked(self) -> None:
        """确保文件及目录存在，若文件为空则写入表头（内部调用需先持锁）。"""
        if self.path.exists() and self.path.stat().st_size > 0:
            return
        self.path.parent.mkdir(parents=True, exist_ok=True)
        with self.path.open("w", encoding="utf-8", newline="") as f:
            writer = csv.DictWriter(f, fieldnames=self.columns, extrasaction="ignore")
            writer.writeheader()

    def ensure_file(self) -> None:
        """带锁确保文件及表头存在。"""
        with self.lock:
            self.ensure_file_unlocked()

    def read_rows_unlocked(self) -> list[dict[str, str]]:
        """无锁读取所有行（内部调用需先持锁）。"""
        if not self.path.exists():
            return []
        with self.path.open("r", encoding="utf-8", newline="") as f:
            reader = csv.DictReader(f)
            return [dict(row) for row in reader]

    def read_rows(self) -> list[dict[str, str]]:
        """加锁读取全部行记录。"""
        with self.lock:
            return self.read_rows_unlocked()

    def write_rows_unlocked(self, rows: list[dict[str, Any]]) -> None:
        """无锁覆盖写入全部行记录（内部调用需先持锁）。"""
        self.path.parent.mkdir(parents=True, exist_ok=True)
        with self.path.open("w", encoding="utf-8", newline="") as f:
            writer = csv.DictWriter(f, fieldnames=self.columns, extrasaction="ignore")
            writer.writeheader()
            serialized_rows = [self.serialize_record(row) for row in rows]
            writer.writerows(serialized_rows)

    def write_rows(self, rows: list[dict[str, Any]]) -> None:
        """加锁覆盖写入全部行记录。"""
        with self.lock:
            self.write_rows_unlocked(rows)

    def insert(self, record: dict[str, Any]) -> dict[str, str]:
        """向 CSV 表尾部追加单条记录。"""
        with self.lock:
            self.ensure_file_unlocked()
            serialized = self.serialize_record(record)
            with self.path.open("a", encoding="utf-8", newline="") as f:
                writer = csv.DictWriter(f, fieldnames=self.columns, extrasaction="ignore")
                writer.writerow(serialized)
            return serialized

    def insert_batch(self, records: list[dict[str, Any]]) -> list[dict[str, str]]:
        """向 CSV 表尾部批量追加多条记录。"""
        if not records:
            return []
        with self.lock:
            self.ensure_file_unlocked()
            serialized_records = [self.serialize_record(r) for r in records]
            with self.path.open("a", encoding="utf-8", newline="") as f:
                writer = csv.DictWriter(f, fieldnames=self.columns, extrasaction="ignore")
                writer.writerows(serialized_records)
            return serialized_records

    def find_by(self, column: str, value: str) -> dict[str, str] | None:
        """根据某一列的值查找第一条匹配的记录。"""
        rows = self.read_rows()
        for row in rows:
            if row.get(column) == value:
                return row
        return None

    def find_all(
        self, where: Callable[[dict[str, str]], bool] | None = None
    ) -> list[dict[str, str]]:
        """获取所有记录，支持传入过滤条件函数。"""
        rows = self.read_rows()
        if where is None:
            return rows
        return [row for row in rows if where(row)]

    def update_by(self, column: str, value: str, updates: dict[str, Any]) -> bool:
        """按匹配条件更新记录，返回是否发生了更新。"""
        with self.lock:
            if not self.path.exists():
                return False
            rows = self.read_rows_unlocked()
            updated = False
            for row in rows:
                if row.get(column) == value:
                    for k, v in updates.items():
                        if k in self.columns:
                            row[k] = "" if v is None else str(v)
                    updated = True
            if updated:
                self.write_rows_unlocked(rows)
            return updated

    def delete_by(self, column: str, value: str) -> bool:
        """根据某列的值删除匹配的记录，返回是否删除了至少一条。"""
        with self.lock:
            if not self.path.exists():
                return False
            rows = self.read_rows_unlocked()
            remaining_rows = [r for r in rows if r.get(column) != value]
            if len(remaining_rows) == len(rows):
                return False
            self.write_rows_unlocked(remaining_rows)
            return True

    def count(self) -> int:
        """获取表内记录总条数。"""
        return len(self.read_rows())

    def clear(self) -> None:
        """清空表内全部数据并重置表头。"""
        with self.lock:
            self.write_rows_unlocked([])
