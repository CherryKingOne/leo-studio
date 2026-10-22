"""pytest 共享夹具。"""

from __future__ import annotations

import shutil
import tempfile
from pathlib import Path

import pytest
from fastapi.testclient import TestClient

from app.main import app
from app.modules.text_files.config import TextFilesSettings
from app.modules.text_files.dependencies import get_storage_service
from app.modules.text_files.service import TextFileStorageService


@pytest.fixture
def upload_dir() -> Path:
    """提供独立的临时上传目录。"""
    path = Path(tempfile.mkdtemp(prefix="leo-upload-"))
    try:
        yield path
    finally:
        shutil.rmtree(path, ignore_errors=True)


@pytest.fixture
def test_settings(upload_dir: Path) -> TextFilesSettings:
    """指向临时目录的测试配置。"""
    return TextFilesSettings(
        upload_dir=upload_dir,
        max_upload_size_mb=1,
        allowed_extensions=sorted({".txt", ".md", ".csv", ".json", ".py", ".log"}),
    )


@pytest.fixture
def storage(test_settings: TextFilesSettings) -> TextFileStorageService:
    """文本文件存储服务实例。"""
    return TextFileStorageService(test_settings)


@pytest.fixture
def client(test_settings: TextFilesSettings) -> TestClient:
    """使用测试配置的 TestClient。"""
    # 覆盖存储服务依赖，使其写入临时目录。
    service = TextFileStorageService(test_settings)
    app.dependency_overrides[get_storage_service] = lambda: service
    with TestClient(app) as test_client:
        yield test_client
    app.dependency_overrides.clear()
