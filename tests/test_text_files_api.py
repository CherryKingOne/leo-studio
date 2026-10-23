"""文本文件上传/下载接口测试。"""

from __future__ import annotations

import io
from pathlib import Path

import pytest
from fastapi.testclient import TestClient

from app.modules.text_files.processor import (
    NovelDataProcessor,
    build_sentence_records,
    split_sentences_by_lines,
)
from app.modules.text_files.service import TextFileStorageService, human_size
from app.shared.errors import ErrorCode

API = "/api/v1/text-files"


class TestUpload:
    """上传接口测试。"""

    def test_upload_single_file(self, client: TestClient) -> None:
        """单文件上传应返回文件元信息与下载地址。"""
        response = client.post(
            f"{API}/upload",
            files={"file": ("hello.txt", io.BytesIO(b"hello world"), "text/plain")},
        )
        assert response.status_code == 200
        body = response.json()
        assert body["code"] == ErrorCode.SUCCESS
        assert body["message"] == "文件上传成功"

        data = body["data"]
        assert data["filename"] == "hello.txt"
        assert data["size"] == 11
        assert data["extension"] == ".txt"
        assert data["content_type"] == "text/plain"
        assert data["category"] == "plain_text"
        assert data["file_id"]
        assert data["download_url"] == f"{API}/{data['file_id']}/download"

    def test_upload_with_description(self, client: TestClient) -> None:
        """上传时携带 description 字段应被保存。"""
        response = client.post(
            f"{API}/upload",
            files={"file": ("note.md", io.BytesIO(b"# title"), "text/markdown")},
            data={"description": "笔记文件"},
        )
        assert response.status_code == 200
        assert response.json()["data"]["description"] == "笔记文件"

    def test_upload_rejects_unsupported_type(self, client: TestClient) -> None:
        """不在白名单中的扩展名应被拒绝。"""
        response = client.post(
            f"{API}/upload",
            files={"file": ("evil.exe", io.BytesIO(b"MZ"), "application/octet-stream")},
        )
        assert response.status_code == 415
        assert response.json()["code"] == ErrorCode.UNSUPPORTED_FILE_TYPE

    def test_upload_rejects_binary_type(self, client: TestClient) -> None:
        """二进制类型不应被文本文件接口接受。"""
        response = client.post(
            f"{API}/upload",
            files={"file": ("logo.png", io.BytesIO(b"\x89PNG"), "image/png")},
        )
        assert response.status_code == 415
        assert response.json()["code"] == ErrorCode.UNSUPPORTED_FILE_TYPE

    def test_upload_rejects_oversize_file(self, client: TestClient) -> None:
        """超过大小限制的文件应被拒绝。"""
        payload = b"x" * (1024 * 1024 + 1024)
        response = client.post(
            f"{API}/upload",
            files={"file": ("big.txt", io.BytesIO(payload), "text/plain")},
        )
        assert response.status_code == 413
        assert response.json()["code"] == ErrorCode.FILE_TOO_LARGE

    def test_upload_sanitizes_path_traversal(self, client: TestClient) -> None:
        """路径穿越文件名应被清洗为安全名称。"""
        response = client.post(
            f"{API}/upload",
            files={"file": ("../../etc/passwd.txt", io.BytesIO(b"x"), "text/plain")},
        )
        assert response.status_code == 200
        assert "/" not in response.json()["data"]["filename"]
        assert ".." not in response.json()["data"]["filename"]

    def test_upload_empty_file_rejected(self, client: TestClient) -> None:
        """空文件应被拒绝。"""
        response = client.post(
            f"{API}/upload",
            files={"file": ("empty.txt", io.BytesIO(b""), "text/plain")},
        )
        assert response.status_code == 422
        assert response.json()["code"] == ErrorCode.INVALID_PARAMETER


class TestSupportedExtensions:
    """白名单扩展名的接受与拒绝测试。"""

    @pytest.mark.parametrize(
        ("filename", "expected_category"),
        [
            ("a.txt", "plain_text"),
            ("a.md", "markdown"),
            ("a.doc", "word"),
            ("a.docx", "word"),
            ("a.epub", "epub"),
        ],
    )
    def test_all_supported_extensions_accepted(
        self, client: TestClient, filename: str, expected_category: str
    ) -> None:
        """五种受支持的扩展名都应上传成功并正确归类。"""
        response = client.post(
            f"{API}/upload",
            files={"file": (filename, io.BytesIO(b"payload"), "application/octet-stream")},
        )
        assert response.status_code == 200
        data = response.json()["data"]
        assert data["filename"] == filename
        assert data["category"] == expected_category

    @pytest.mark.parametrize("filename", ["a.csv", "a.json", "a.py", "a.log", "a.rst"])
    def test_removed_extensions_rejected(self, client: TestClient, filename: str) -> None:
        """已从白名单移除的扩展名应返回 415。"""
        response = client.post(
            f"{API}/upload",
            files={"file": (filename, io.BytesIO(b"payload"), "text/plain")},
        )
        assert response.status_code == 415
        assert response.json()["code"] == ErrorCode.UNSUPPORTED_FILE_TYPE

    def test_binary_document_gets_correct_mime_type(self, client: TestClient) -> None:
        """MIME 类型按扩展名推导，不受客户端提交值影响。"""
        response = client.post(
            f"{API}/upload",
            files={"file": ("book.epub", io.BytesIO(b"PK\x03\x04payload"), "text/plain")},
        )
        assert response.status_code == 200
        assert response.json()["data"]["content_type"] == "application/epub+zip"

    def test_docx_gets_correct_mime_type(self, client: TestClient) -> None:
        """`.docx` 应推导出标准 OOXML MIME 类型。"""
        response = client.post(
            f"{API}/upload",
            files={"file": ("report.docx", io.BytesIO(b"PK\x03\x04payload"), "text/plain")},
        )
        assert response.status_code == 200
        data = response.json()["data"]
        assert data["content_type"] == (
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        )
        assert data["category"] == "word"

    def test_doc_gets_correct_mime_type(self, client: TestClient) -> None:
        """`.doc` 应推导出 `application/msword`。"""
        response = client.post(
            f"{API}/upload",
            files={"file": ("legacy.doc", io.BytesIO(b"\xd0\xcf\x11\xe0payload"), "text/plain")},
        )
        assert response.status_code == 200
        assert response.json()["data"]["content_type"] == "application/msword"

    def test_binary_document_preview_is_rejected(self, client: TestClient) -> None:
        """二进制文档不支持文本预览，应返回 422 而非解码异常。"""
        upload = client.post(
            f"{API}/upload",
            files={"file": ("book.epub", io.BytesIO(b"PK\x03\x04payload"), "text/plain")},
        )
        file_id = upload.json()["data"]["file_id"]

        response = client.get(f"{API}/{file_id}/content")
        assert response.status_code == 422
        assert response.json()["code"] == ErrorCode.INVALID_PARAMETER

    def test_binary_document_download_works(self, client: TestClient) -> None:
        """二进制文档仍可通过下载接口获取原始字节。"""
        payload = b"PK\x03\x04epub-bytes"
        upload = client.post(
            f"{API}/upload",
            files={"file": ("book.epub", io.BytesIO(payload), "text/plain")},
        )
        file_id = upload.json()["data"]["file_id"]

        response = client.get(f"{API}/{file_id}/download")
        assert response.status_code == 200
        assert response.content == payload
        assert "book.epub" in response.headers["content-disposition"]


class TestDownload:
    """下载与查询接口测试。"""

    def test_download_returns_content(self, client: TestClient) -> None:
        """下载接口应返回原始文件内容。"""
        upload = client.post(
            f"{API}/upload",
            files={"file": ("doc.txt", io.BytesIO(b"content-bytes"), "text/plain")},
        )
        file_id = upload.json()["data"]["file_id"]

        response = client.get(f"{API}/{file_id}/download")
        assert response.status_code == 200
        assert response.content == b"content-bytes"
        assert "doc.txt" in response.headers["content-disposition"]

    def test_get_content_returns_text(self, client: TestClient) -> None:
        """内容预览接口应返回解码后的文本。"""
        upload = client.post(
            f"{API}/upload",
            files={"file": ("readme.md", io.BytesIO("你好".encode()), "text/markdown")},
        )
        file_id = upload.json()["data"]["file_id"]

        response = client.get(f"{API}/{file_id}/content")
        assert response.status_code == 200
        data = response.json()["data"]
        assert data["content"] == "你好"
        assert data["encoding"] == "utf-8"
        assert data["size"] == 6

    def test_get_file_detail(self, client: TestClient) -> None:
        """详情接口应返回文件元信息。"""
        upload = client.post(
            f"{API}/upload",
            files={"file": ("doc.txt", io.BytesIO(b"abc"), "text/plain")},
        )
        file_id = upload.json()["data"]["file_id"]

        response = client.get(f"{API}/{file_id}")
        assert response.status_code == 200
        assert response.json()["data"]["filename"] == "doc.txt"

    def test_get_missing_file_returns_404(self, client: TestClient) -> None:
        """查询不存在的文件应返回 404。"""
        response = client.get(f"{API}/does-not-exist")
        assert response.status_code == 404
        assert response.json()["code"] == ErrorCode.FILE_NOT_FOUND

    def test_list_files_pagination(self, client: TestClient) -> None:
        """列表接口应返回分页信息。"""
        for i in range(3):
            client.post(
                f"{API}/upload",
                files={"file": (f"f{i}.txt", io.BytesIO(b"x"), "text/plain")},
            )

        response = client.get(API, params={"page": 1, "page_size": 2})
        assert response.status_code == 200
        data = response.json()["data"]
        assert data["total"] == 3
        assert data["pages"] == 2
        assert len(data["items"]) == 2


class TestDelete:
    """删除接口测试。"""

    def test_delete_file(self, client: TestClient) -> None:
        """删除文件后再次查询应返回 404。"""
        upload = client.post(
            f"{API}/upload",
            files={"file": ("tmp.txt", io.BytesIO(b"x"), "text/plain")},
        )
        file_id = upload.json()["data"]["file_id"]

        response = client.delete(f"{API}/{file_id}")
        assert response.status_code == 200
        assert response.json()["data"]["deleted"] is True
        assert client.get(f"{API}/{file_id}").status_code == 404

    def test_delete_missing_file_returns_404(self, client: TestClient) -> None:
        """删除不存在的文件应返回 404。"""
        response = client.delete(f"{API}/nope")
        assert response.status_code == 404
        assert response.json()["code"] == ErrorCode.FILE_NOT_FOUND


class TestSystem:
    """系统与通用约定测试。"""

    def test_health_check(self, client: TestClient) -> None:
        """健康检查接口应返回服务状态。"""
        response = client.get("/health")
        assert response.status_code == 200
        assert response.json()["status"] == "ok"

    def test_system_module_health(self, client: TestClient) -> None:
        """带版本前缀的系统健康检查应可用。"""
        response = client.get("/api/v1/system/health")
        assert response.status_code == 200
        assert response.json()["status"] == "ok"

    def test_root_returns_module_index(self, client: TestClient) -> None:
        """根路径应返回服务信息与模块索引。"""
        response = client.get("/")
        assert response.status_code == 200
        body = response.json()
        assert body["api_prefix"] == "/api/v1"
        assert "text-files" in body["modules"]

    def test_request_id_in_header_and_body(self, client: TestClient) -> None:
        """响应头与响应体都应携带 request_id。"""
        response = client.get("/api/v1/text-files")
        assert response.status_code == 200
        header_id = response.headers["X-Request-ID"]
        assert response.json()["request_id"] == header_id

    def test_custom_request_id_is_echoed(self, client: TestClient) -> None:
        """客户端传入的 X-Request-ID 应被沿用。"""
        response = client.get("/api/v1/text-files", headers={"X-Request-ID": "my-trace-id"})
        assert response.status_code == 200
        assert response.headers["X-Request-ID"] == "my-trace-id"
        assert response.json()["request_id"] == "my-trace-id"

    def test_old_generic_route_is_gone(self, client: TestClient) -> None:
        """旧的通用 /api/v1/files 路由应已移除，避免与文本文件接口冲突。"""
        response = client.post(
            "/api/v1/files/upload",
            files={"file": ("hello.txt", io.BytesIO(b"x"), "text/plain")},
        )
        assert response.status_code == 404


class TestStorageService:
    """存储服务单元测试。"""

    def test_human_size(self) -> None:
        """字节数应转换为可读格式。"""
        assert human_size(512) == "512 B"
        assert human_size(2048) == "2.00 KB"
        assert human_size(1024 * 1024) == "1.00 MB"

    def test_sanitize_filename(self, storage: TextFileStorageService) -> None:
        """文件名清洗应移除路径与危险字符。"""
        assert storage.sanitize_filename("a/b/c.txt") == "c.txt"
        assert storage.sanitize_filename("..\\..\\win.txt") == "win.txt"

    def test_save_file_creates_unique_ids(self, storage: TextFileStorageService) -> None:
        """同名文件应生成不同的 file_id。"""
        first = storage.save_file("same.txt", io.BytesIO(b"a"), content_type="text/plain")
        second = storage.save_file("same.txt", io.BytesIO(b"b"), content_type="text/plain")
        assert first.file_id != second.file_id
        assert first.filename == second.filename == "same.txt"

    def test_cleanup(self, storage: TextFileStorageService, upload_dir: Path) -> None:
        """cleanup 应清空上传目录。"""
        storage.save_file("x.txt", io.BytesIO(b"a"), content_type="text/plain")
        storage.cleanup()
        assert not any(upload_dir.iterdir())


class TestNovelCsvStorage:
    """小说 CSV 表格管理测试。"""

    def test_upload_writes_to_novels_csv(
        self, client: TestClient, csv_dir: Path
    ) -> None:
        """上传小说文件后应在 novels.csv 中记录小说元信息，包含 format 字段，且不含 author、genre 及底层存储冗余字段。"""
        response = client.post(
            f"{API}/upload",
            files={
                "file": (
                    "凡人修仙传.txt",
                    io.BytesIO("第1章 山边小村\n正文内容在此".encode("utf-8")),
                    "text/plain",
                )
            },
        )
        assert response.status_code == 200
        file_id = response.json()["data"]["file_id"]

        novels_csv = csv_dir / "novels.csv"
        assert novels_csv.exists()
        header = novels_csv.read_text(encoding="utf-8").splitlines()[0]
        # 确保包含纯小说业务字段与文件格式
        assert "novel_id" in header
        assert "title" in header
        assert "format" in header
        assert "chapter_count" in header
        assert "total_words" in header
        assert "created_at" in header
        # 确保不含已移除字段与底层技术字段
        assert "author" not in header
        assert "genre" not in header
        assert "avg_words_per_chapter" not in header
        assert "stored_name" not in header
        assert "content_type" not in header
        assert "category" not in header
        assert "description" not in header

        content = novels_csv.read_text(encoding="utf-8")
        assert file_id in content
        assert "凡人修仙传" in content
        assert ",txt," in content

    def test_upload_splits_chapters_in_novel_contents_csv(
        self, client: TestClient, csv_dir: Path
    ) -> None:
        """上传小说文件后应按章节解析并写入 novel_contents.csv，通过 novel_id 关联。"""
        novel_text = (
            "第1章 初入仙门\n少年踏入宗门大殿，心生震撼。\n"
            "第2章 执事发难\n执事长老冷冷一笑，说道：'且慢。'"
        )
        response = client.post(
            f"{API}/upload",
            files={"file": ("合欢宗.txt", io.BytesIO(novel_text.encode("utf-8")), "text/plain")},
        )
        assert response.status_code == 200
        file_id = response.json()["data"]["file_id"]

        contents_csv = csv_dir / "novel_contents.csv"
        assert contents_csv.exists()
        lines = contents_csv.read_text(encoding="utf-8").splitlines()
        header = lines[0]
        # 校验表头字段
        assert "chapter_id" in header
        assert "novel_id" in header
        assert "chapter_index" in header
        assert "chapter_title" in header
        assert "content" in header
        assert "word_count" in header

        # 校验章节切分与关联
        raw_csv = contents_csv.read_text(encoding="utf-8")
        assert file_id in raw_csv
        assert "第1章 初入仙门" in raw_csv
        assert "少年踏入宗门大殿" in raw_csv
        assert "第2章 执事发难" in raw_csv
        assert "执事长老冷冷一笑" in raw_csv

        # 通过 content 接口获取全文应与原内容一致
        content_res = client.get(f"{API}/{file_id}/content")
        assert content_res.status_code == 200
        assert content_res.json()["data"]["content"] == novel_text

    def test_delete_removes_from_all_csv_files(
        self, client: TestClient, csv_dir: Path
    ) -> None:
        """删除文件时应同时从 novels.csv、novel_contents.csv 与 novel_sentences.csv 中删除记录。"""
        novel_text = "第1章 待删章节\n即将被删除的内容第一行。\n即将被删除的内容第二行。"
        upload = client.post(
            f"{API}/upload",
            files={"file": ("to_delete.txt", io.BytesIO(novel_text.encode("utf-8")), "text/plain")},
        )
        file_id = upload.json()["data"]["file_id"]

        novels_csv = csv_dir / "novels.csv"
        contents_csv = csv_dir / "novel_contents.csv"
        sentences_csv = csv_dir / "novel_sentences.csv"
        assert file_id in novels_csv.read_text(encoding="utf-8")
        assert file_id in contents_csv.read_text(encoding="utf-8")
        assert file_id in sentences_csv.read_text(encoding="utf-8")

        delete_res = client.delete(f"{API}/{file_id}")
        assert delete_res.status_code == 200

        assert file_id not in novels_csv.read_text(encoding="utf-8")
        assert file_id not in contents_csv.read_text(encoding="utf-8")
        assert file_id not in sentences_csv.read_text(encoding="utf-8")

    def test_large_novel_content_csv_handling(
        self, client: TestClient, csv_dir: Path
    ) -> None:
        """大型长篇小说内容落盘至 CSV 应不受字段大小限制报错。"""
        chapter_1 = "第1章 巨篇开启\n" + "长篇小说章节内容测试，包含标点、\"引号\"、,逗号与换行。\n" * 2000
        chapter_2 = "第2章 渐入佳境\n" + "第二章长篇内容测试，浩瀚长篇正文在此。\n" * 2000
        full_text = chapter_1 + "\n" + chapter_2

        upload = client.post(
            f"{API}/upload",
            files={"file": ("big_novel.txt", io.BytesIO(full_text.encode("utf-8")), "text/plain")},
        )
        assert upload.status_code == 200
        file_id = upload.json()["data"]["file_id"]

        content_res = client.get(f"{API}/{file_id}/content")
        assert content_res.status_code == 200
        assert content_res.json()["data"]["content"] == full_text


class TestNovelSentenceProcessing:
    """小说章节按行切句与数据处理器测试。"""

    def test_split_sentences_by_lines_filters_empty_and_strips(self) -> None:
        """应按行切分并清理首尾空白字符，同时过滤连续空行。"""
        raw = "  \u3000第一行正文。\u3000 \n\n\n\t第二行正文。\n   \n第三行正文。  "
        sentences = split_sentences_by_lines(raw)
        assert sentences == ["第一行正文。", "第二行正文。", "第三行正文。"]

    def test_split_sentences_empty_input(self) -> None:
        """空字符串或仅含空格换行时应返回空列表。"""
        assert split_sentences_by_lines("") == []
        assert split_sentences_by_lines("  \n\t \n  ") == []

    def test_build_sentence_records(self) -> None:
        """构造切句记录应包含全部标准字段，索引从 0 递增。"""
        sentences = ["你好世界。", "FastAPI 很高效。"]
        records = build_sentence_records("novel-123", "chapter-456", sentences)
        assert len(records) == 2

        assert records[0]["novel_id"] == "novel-123"
        assert records[0]["chapter_id"] == "chapter-456"
        assert records[0]["sentence_index"] == "0"
        assert records[0]["content"] == "你好世界。"
        assert records[0]["word_count"] == "5"

        assert records[1]["sentence_index"] == "1"
        assert records[1]["content"] == "FastAPI 很高效。"
        assert records[1]["word_count"] == "11"

    def test_upload_creates_novel_sentences_csv(
        self, client: TestClient, csv_dir: Path
    ) -> None:
        """上传小说后应生成 novel_sentences.csv，包含切句记录并与章节关联。"""
        novel_text = (
            "第1章 登仙之路\n"
            "少年仰望苍穹，眼神坚定。\n"
            "长生大道漫漫，唯剑作伴。\n"
            "第2章 仙门测试\n"
            "测灵石发出微弱的光芒。\n"
        )
        upload = client.post(
            f"{API}/upload",
            files={"file": ("剑道.txt", io.BytesIO(novel_text.encode("utf-8")), "text/plain")},
        )
        assert upload.status_code == 200
        file_id = upload.json()["data"]["file_id"]

        sentences_csv = csv_dir / "novel_sentences.csv"
        assert sentences_csv.exists()

        header = sentences_csv.read_text(encoding="utf-8").splitlines()[0]
        assert "sentence_id" in header
        assert "novel_id" in header
        assert "chapter_id" in header
        assert "sentence_index" in header
        assert "content" in header
        assert "word_count" in header

        contents = sentences_csv.read_text(encoding="utf-8")
        assert file_id in contents
        assert "少年仰望苍穹，眼神坚定。" in contents
        assert "长生大道漫漫，唯剑作伴。" in contents
        assert "测灵石发出微弱的光芒。" in contents

    def test_get_sentences_api(self, client: TestClient) -> None:
        """切句查询接口应正确返回小说各章节的所有切句记录。"""
        novel_text = (
            "第1章 启程\n"
            "春风拂面，柳絮纷飞。\n"
            "少年整装待发。\n"
        )
        upload = client.post(
            f"{API}/upload",
            files={"file": ("启程.txt", io.BytesIO(novel_text.encode("utf-8")), "text/plain")},
        )
        file_id = upload.json()["data"]["file_id"]

        res = client.get(f"{API}/{file_id}/sentences")
        assert res.status_code == 200
        body = res.json()
        assert body["code"] == ErrorCode.SUCCESS
        data = body["data"]
        assert data["novel_id"] == file_id
        assert data["total"] == 2
        items = data["items"]
        assert len(items) == 2
        assert items[0]["sentence_index"] == 0
        assert items[0]["content"] == "春风拂面，柳絮纷飞。"
        assert items[0]["word_count"] == 10
        assert items[1]["sentence_index"] == 1
        assert items[1]["content"] == "少年整装待发。"

    def test_get_sentences_api_filter_by_chapter_id(
        self, client: TestClient, csv_dir: Path
    ) -> None:
        """切句查询接口支持根据 chapter_id 过滤指定章节的句子。"""
        novel_text = (
            "第1章 第一章内容\n"
            "第一章的一行。\n"
            "第2章 第二章内容\n"
            "第二章的一行。\n"
        )
        upload = client.post(
            f"{API}/upload",
            files={"file": ("多章节.txt", io.BytesIO(novel_text.encode("utf-8")), "text/plain")},
        )
        file_id = upload.json()["data"]["file_id"]

        # 从 contents 表中获取第二章的 chapter_id
        contents_csv = csv_dir / "novel_contents.csv"
        rows = contents_csv.read_text(encoding="utf-8").splitlines()
        chapter_2_id = ""
        for line in rows[1:]:
            parts = line.split(",")
            if len(parts) >= 4 and "第2章" in parts[3]:
                chapter_2_id = parts[0]
                break
        assert chapter_2_id != ""

        res = client.get(f"{API}/{file_id}/sentences", params={"chapter_id": chapter_2_id})
        assert res.status_code == 200
        items = res.json()["data"]["items"]
        assert len(items) == 1
        assert items[0]["chapter_id"] == chapter_2_id
        assert items[0]["content"] == "第二章的一行。"

    def test_processor_save_and_delete_methods(self, storage: TextFileStorageService) -> None:
        """测试 NovelDataProcessor 的保存与删除生命周期。"""
        processor = storage.processor
        novel_id = "test-novel-uuid"
        chapter_id = "test-chapter-uuid"
        content = "落霞与孤鹜齐飞。\n秋水共长天一色。"

        records = processor.process_and_save_chapter(novel_id, chapter_id, content)
        assert len(records) == 2

        query_by_ch = processor.get_sentences_by_chapter(chapter_id)
        assert len(query_by_ch) == 2

        query_by_nov = processor.get_sentences_by_novel(novel_id)
        assert len(query_by_nov) == 2

        deleted = processor.delete_sentences_by_novel(novel_id)
        assert deleted is True

        assert len(processor.get_sentences_by_novel(novel_id)) == 0


