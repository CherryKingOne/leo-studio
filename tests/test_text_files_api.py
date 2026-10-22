"""文本文件上传/下载接口测试。"""

from __future__ import annotations

import io
from pathlib import Path

import pytest
from fastapi.testclient import TestClient

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
