"""检查项目中是否存在以下划线开头的函数、方法与类定义。

根据规范，除双下划线包裹的魔术方法外，禁止以 _ 开头的命名。
"""

from __future__ import annotations

import ast
import sys
from collections.abc import Iterable
from pathlib import Path

PROJECT_ROOT = Path(__file__).resolve().parent.parent


def is_dunder(name: str) -> bool:
    """判断是否为双下划线魔术方法（如 __init__）。"""
    return len(name) > 4 and name.startswith("__") and name.endswith("__")


def is_violation(name: str) -> bool:
    """判断名称是否违规（以下划线开头且不是魔术方法）。"""
    return name.startswith("_") and not is_dunder(name)


class NamingVisitor(ast.NodeVisitor):
    """AST 访问器，收集违规命名的类、方法和函数。"""

    def __init__(self, path: Path) -> None:
        self.path = path
        self.findings: list[tuple[Path, int, int, str, str]] = []
        self.class_depth = 0

    def visit_ClassDef(self, node: ast.ClassDef) -> None:
        if is_violation(node.name):
            self.findings.append((self.path, node.lineno, node.col_offset, node.name, "class"))
        self.class_depth += 1
        self.generic_visit(node)
        self.class_depth -= 1

    def visit_FunctionDef(self, node: ast.FunctionDef) -> None:
        kind = "method" if self.class_depth > 0 else "function"
        if is_violation(node.name):
            self.findings.append((self.path, node.lineno, node.col_offset, node.name, kind))
        self.generic_visit(node)

    def visit_AsyncFunctionDef(self, node: ast.AsyncFunctionDef) -> None:
        kind = "method" if self.class_depth > 0 else "function"
        if is_violation(node.name):
            self.findings.append((self.path, node.lineno, node.col_offset, node.name, kind))
        self.generic_visit(node)


def scan_file(path: Path) -> list[tuple[Path, int, int, str, str]]:
    """扫描单个 Python 文件。"""
    try:
        content = path.read_text(encoding="utf-8")
        tree = ast.parse(content, filename=str(path))
    except (SyntaxError, UnicodeDecodeError, OSError):
        return []
    visitor = NamingVisitor(path)
    visitor.visit(tree)
    return visitor.findings


def scan(targets: Iterable[Path]) -> list[tuple[Path, int, int, str, str]]:
    """扫描目标列表中的所有文件和目录。"""
    findings: list[tuple[Path, int, int, str, str]] = []
    for target in targets:
        if not target.exists():
            continue
        if target.is_file():
            if target.suffix == ".py":
                findings.extend(scan_file(target))
        elif target.is_dir():
            for py_file in sorted(target.rglob("*.py")):
                findings.extend(scan_file(py_file))
    return findings


def main() -> int:
    """命令行执行入口。"""
    target_dirs = ("app", "tests", "scripts")
    targets = [PROJECT_ROOT / name for name in target_dirs]
    findings = scan(targets)
    if not findings:
        print("未发现违规命名，所有函数/方法/类名均符合规范。")
        return 0

    print(f"发现 {len(findings)} 处违规命名：")
    for path, line, col, name, kind in findings:
        rel = path.relative_to(PROJECT_ROOT) if path.is_relative_to(PROJECT_ROOT) else path
        print(f"  {rel}:{line}:{col + 1}: {kind} '{name}'")
    return 1


if __name__ == "__main__":
    sys.exit(main())
