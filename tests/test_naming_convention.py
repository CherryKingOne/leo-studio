"""项目命名规范测试：禁止以下划线开头的函数、方法与类名。"""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "scripts"))

from check_no_underscore_prefix import PROJECT_ROOT, scan  # noqa: E402

TARGET_DIRS = ("app", "tests", "scripts")


def build_violation_source(name: str) -> str:
    """构造包含违规命名的示例源码。"""
    return f"class Sample:\n    def {name}(self) -> None:\n        pass\n"


class TestNoUnderscorePrefix:
    """校验项目内不存在以下划线开头的函数、方法与类名。"""

    def test_project_has_no_violations(self) -> None:
        """扫描整个项目源码，确保没有违规命名。"""
        targets = [PROJECT_ROOT / name for name in TARGET_DIRS]
        findings = scan(targets)
        details = [
            f"{path.relative_to(PROJECT_ROOT)}:{line}:{col + 1}: {kind} '{name}'"
            for path, line, col, name, kind in findings
        ]
        assert not findings, "以下划线开头的命名违反项目规范:\n" + "\n".join(details)

    @pytest.mark.parametrize(
        "source",
        [
            "def _helper() -> None:\n    pass\n",
            "async def _helper() -> None:\n    pass\n",
            "class _Helper:\n    pass\n",
            "class Helper:\n    def _method(self) -> None:\n        pass\n",
            "class Helper:\n    @staticmethod\n    def _method() -> None:\n        pass\n",
            "class Helper:\n    @classmethod\n    def _method(cls) -> None:\n        pass\n",
        ],
    )
    def test_scanner_detects_violations(self, source: str, tmp_path: Path) -> None:
        """扫描器应识别各类以下划线开头的命名。"""
        target = tmp_path / "sample.py"
        target.write_text(source, encoding="utf-8")

        findings = scan([target])

        assert len(findings) == 1
        _, _, _, name, _ = findings[0]
        assert name.startswith("_")

    @pytest.mark.parametrize(
        "source",
        [
            "def helper() -> None:\n    pass\n",
            "async def helper() -> None:\n    pass\n",
            "class Helper:\n    pass\n",
            "class Helper:\n    def method(self) -> None:\n        pass\n",
            "class Helper:\n    def __init__(self) -> None:\n        pass\n",
            "class Helper:\n    def __repr__(self) -> str:\n        return 'Helper'\n",
        ],
    )
    def test_scanner_allows_valid_names(self, source: str, tmp_path: Path) -> None:
        """扫描器不应误报普通命名与魔术方法。"""
        target = tmp_path / "sample.py"
        target.write_text(source, encoding="utf-8")

        assert scan([target]) == []

    def test_scanner_reports_method_kind(self, tmp_path: Path) -> None:
        """类体中的函数应被识别为方法。"""
        target = tmp_path / "sample.py"
        target.write_text(build_violation_source("_method"), encoding="utf-8")

        findings = scan([target])

        assert findings[0][4] == "method"
