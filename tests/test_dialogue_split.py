"""台词切分纯函数与 CSV 落盘测试。"""

from __future__ import annotations

import csv
import io
from pathlib import Path

from fastapi.testclient import TestClient

from app.modules.text_files.dialogue import build_dialogue_records, split_dialogue_line
from app.modules.text_files.models import DialogueSegmentType

API = "/api/v1/text-files"


def read_rows(path: Path) -> list[dict[str, str]]:
    """读取 CSV 为字典列表。"""
    with path.open("r", encoding="utf-8", newline="") as file:
        return list(csv.DictReader(file))


def novel_bytes(*lines: str) -> io.BytesIO:
    """构造内存小说文件。"""
    return io.BytesIO("\n".join(lines).encode("utf-8"))


class TestSplitDialogueLine:
    """台词切分规则测试。"""

    def test_splits_narration_and_dialogue(self) -> None:
        """标准「叙述 + 引号对白」应拆成两段。"""
        segments = split_dialogue_line("唐三对着比比东道：“复活吧我的爱人”")

        assert len(segments) == 2
        assert segments[0].segment_index == 0
        assert segments[0].segment_type == DialogueSegmentType.NARRATION
        assert segments[0].content == "唐三对着比比东道："
        assert segments[0].char_start == 0
        assert segments[0].char_end == 9
        assert segments[0].quote_char == ""

        assert segments[1].segment_index == 1
        assert segments[1].segment_type == DialogueSegmentType.DIALOGUE
        assert segments[1].content == "“复活吧我的爱人”"
        assert segments[1].char_start == 9
        assert segments[1].char_end == 18
        assert segments[1].quote_char == "“"

    def test_keeps_offsets_aligned_with_content(self) -> None:
        """片段偏移应满足 text[start:end] == content。"""
        text = "唐三对着比比东道：“复活吧我的爱人”"
        for segment in split_dialogue_line(text):
            assert text[segment.char_start : segment.char_end] == segment.content

    def test_ignores_pure_dialogue_line(self) -> None:
        """整行纯对白不产生切分记录。"""
        assert split_dialogue_line("“复活吧我的爱人”") == []

    def test_ignores_line_without_quotes(self) -> None:
        """无引号行不产生切分记录。"""
        assert split_dialogue_line("唐三对着比比东道：复活吧我的爱人") == []

    def test_ignores_unclosed_quote(self) -> None:
        """引号未闭合时不切分。"""
        assert split_dialogue_line("唐三对着比比东道：“复活吧我的爱人") == []

    def test_keeps_tail_with_dialogue(self) -> None:
        """闭引号后的尾部内容并入对白段，保证不丢字。"""
        segments = split_dialogue_line("他说：“你好”然后走了。")

        assert len(segments) == 2
        assert segments[0].content == "他说："
        assert segments[1].content == "“你好”然后走了。"

    def test_only_splits_first_quote_pair(self) -> None:
        """一行多对引号时只切第一对。"""
        segments = split_dialogue_line("他说：“第一句”，然后说：“第二句”")

        assert len(segments) == 2
        assert segments[1].content == "“第一句”，然后说：“第二句”"

    def test_empty_input(self) -> None:
        """空输入返回空列表。"""
        assert split_dialogue_line("") == []
        assert split_dialogue_line("   ") == []


class TestBuildDialogueRecords:
    """台词记录构造测试。"""

    def test_builds_records_from_sentence_records(self) -> None:
        """应根据切句记录生成台词片段记录。"""
        sentence_records = [
            {
                "sentence_id": "s1",
                "sentence_index": "0",
                "content": "唐三对着比比东道：“复活吧我的爱人”",
            },
            {
                "sentence_id": "s2",
                "sentence_index": "1",
                "content": "这行没有对白。",
            },
        ]

        records = build_dialogue_records("novel-1", "chapter-1", sentence_records)

        assert len(records) == 2
        assert records[0]["novel_id"] == "novel-1"
        assert records[0]["chapter_id"] == "chapter-1"
        assert records[0]["sentence_id"] == "s1"
        assert records[0]["segment_index"] == "0"
        assert records[0]["segment_type"] == "narration"
        assert records[0]["content"] == "唐三对着比比东道："
        assert records[1]["segment_index"] == "1"
        assert records[1]["segment_type"] == "dialogue"
        assert records[1]["content"] == "“复活吧我的爱人”"


class TestDialogueCsv:
    """台词切分 CSV 集成测试。"""

    def test_upload_writes_dialogues_csv(self, client: TestClient, csv_dir: Path) -> None:
        """上传后应写入 novel_dialogues.csv，且不改动切句内容。"""
        novel_text = (
            "第1章 对白测试\n"
            "唐三对着比比东道：“复活吧我的爱人”\n"
            "这行没有对白。"
        )
        response = client.post(
            f"{API}/upload",
            files={
                "file": (
                    "对白.txt",
                    novel_bytes(*novel_text.splitlines()),
                    "text/plain",
                )
            },
        )
        assert response.status_code == 200
        file_id = response.json()["data"]["file_id"]

        dialogues_csv = csv_dir / "novel_dialogues.csv"
        assert dialogues_csv.exists()

        rows = read_rows(dialogues_csv)
        assert len(rows) == 2
        assert rows[0]["novel_id"] == file_id
        assert rows[0]["segment_type"] == "narration"
        assert rows[0]["content"] == "唐三对着比比东道："
        assert rows[1]["segment_type"] == "dialogue"
        assert rows[1]["content"] == "“复活吧我的爱人”"
        assert rows[1]["sentence_id"] == rows[0]["sentence_id"]

        sentences_csv = csv_dir / "novel_sentences.csv"
        sentences = read_rows(sentences_csv)
        assert len(sentences) == 2
        assert sentences[0]["content"] == "唐三对着比比东道：“复活吧我的爱人”"

    def test_pure_dialogue_line_has_no_dialogue_rows(
        self, client: TestClient, csv_dir: Path
    ) -> None:
        """整行纯对白不产生台词切分记录。"""
        response = client.post(
            f"{API}/upload",
            files={
                "file": (
                    "纯对白.txt",
                    novel_bytes("第1章 测试", "“你好”"),
                    "text/plain",
                )
            },
        )
        assert response.status_code == 200

        dialogues_csv = csv_dir / "novel_dialogues.csv"
        assert dialogues_csv.exists()
        assert read_rows(dialogues_csv) == []

    def test_delete_cascades_dialogues(
        self, client: TestClient, csv_dir: Path
    ) -> None:
        """删除小说时应级联删除台词切分记录。"""
        upload = client.post(
            f"{API}/upload",
            files={
                "file": (
                    "待删.txt",
                    novel_bytes("第1章 测试", "唐三对着比比东道：“复活吧我的爱人”"),
                    "text/plain",
                )
            },
        )
        file_id = upload.json()["data"]["file_id"]
        assert len(read_rows(csv_dir / "novel_dialogues.csv")) == 2

        response = client.delete(f"{API}/{file_id}")
        assert response.status_code == 200
        assert read_rows(csv_dir / "novel_dialogues.csv") == []


class TestDialogueAppendOverwrite:
    """追更覆盖章节时台词切分的生命周期测试。"""

    def test_append_new_chapter_writes_dialogues(
        self, client: TestClient, csv_dir: Path
    ) -> None:
        """增量追全新章节时应写入对应台词记录。"""
        create = client.post(
            "/api/v1/projects",
            data={"name": "追更测试"},
            files={
                "file": (
                    "追更.txt",
                    novel_bytes("第1章 旧章", "没有对白的叙述。"),
                    "text/plain",
                )
            },
        )
        assert create.status_code == 200
        project_id = create.json()["data"]["project_id"]
        assert read_rows(csv_dir / "novel_dialogues.csv") == []

        append = client.post(
            f"/api/v1/projects/{project_id}/chapters/upload",
            data={"overwrite_existing": "false", "dry_run": "false"},
            files={
                "file": (
                    "追更.txt",
                    novel_bytes("第2章 新章", "唐三对着比比东道：“复活吧我的爱人”"),
                    "text/plain",
                )
            },
        )
        assert append.status_code == 200

        rows = read_rows(csv_dir / "novel_dialogues.csv")
        assert len(rows) == 2
        assert rows[0]["content"] == "唐三对着比比东道："
        assert rows[1]["content"] == "“复活吧我的爱人”"

    def test_overwrite_replaces_dialogues(
        self, client: TestClient, csv_dir: Path
    ) -> None:
        """覆盖已有章节时应删除旧台词记录并写入新记录。"""
        create = client.post(
            "/api/v1/projects",
            data={"name": "覆盖测试"},
            files={
                "file": (
                    "覆盖.txt",
                    novel_bytes("第1章 旧章", "唐三对着比比东道：“复活吧我的爱人”"),
                    "text/plain",
                )
            },
        )
        assert create.status_code == 200
        project_id = create.json()["data"]["project_id"]
        assert len(read_rows(csv_dir / "novel_dialogues.csv")) == 2

        append = client.post(
            f"/api/v1/projects/{project_id}/chapters/upload",
            data={"overwrite_existing": "true", "dry_run": "false"},
            files={
                "file": (
                    "覆盖.txt",
                    novel_bytes("第1章 新章", "唐三对着比比东道：“这一次我会保护你”"),
                    "text/plain",
                )
            },
        )
        assert append.status_code == 200

        rows = read_rows(csv_dir / "novel_dialogues.csv")
        assert len(rows) == 2
        assert rows[0]["content"] == "唐三对着比比东道："
        assert rows[1]["content"] == "“这一次我会保护你”"