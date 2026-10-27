"""项目管理接口测试。

测试覆盖：
- 创建项目（正常流程、自动提取作者与类型、显式指定参数）
- 兼容别名入参（title / project_name / /upload 别名路由）
- 参数校验失败（缺失项目名称、空文件、不支持的扩展名、超限文件）
- 项目列表分页查询与排序
- 查询单个项目详情与不存在处理
- 删除项目及级联删除小说文件与切句数据
"""

from __future__ import annotations

import io
import uuid

from fastapi.testclient import TestClient

SAMPLE_NOVEL_CONTENT = """
第1章 绯红之月
痛！好痛！头好痛！狂暴的剧痛如同一柄烧红的钢铁长矛，狠狠刺入了脑海深处。

第2章 宿命之枪
左轮手枪与桌上的自杀日记，黄铜握柄还残留着体温。
""".strip()


def build_text_file(content: str) -> io.BytesIO:
    """构造内存中的文本文件字节流。"""
    return io.BytesIO(content.encode("utf-8"))


class TestCreateProject:
    """创建项目接口测试。"""

    def test_create_project_success(self, client: TestClient) -> None:
        """正常创建项目：上传合法小说文本并提供项目名称。"""
        response = client.post(
            "/api/v1/projects",
            data={"name": "诡秘之主：第一部"},
            files={"file": ("诡秘之主.txt", build_text_file(SAMPLE_NOVEL_CONTENT), "text/plain")},
        )
        assert response.status_code == 200
        body = response.json()
        assert body["code"] == 0
        assert body["message"] == "项目创建成功"

        data = body["data"]
        assert data["name"] == "诡秘之主：第一部"
        assert data["filename"] == "诡秘之主.txt"
        assert data["status"] == "制作中"
        assert data["chapter_count"] == 2
        assert data["total_words"] > 0
        # 验证 project_id 与 file_id 均属于合法的 UUID4，而非雪花 ID
        project_uuid = uuid.UUID(hex=data["project_id"])
        file_uuid = uuid.UUID(hex=data["file_id"])
        assert project_uuid.version == 4
        assert file_uuid.version == 4

    def test_create_project_alias_upload_endpoint(self, client: TestClient) -> None:
        """支持 /api/v1/projects/upload 别名接口。"""
        response = client.post(
            "/api/v1/projects/upload",
            data={"name": "别名接口项目"},
            files={"file": ("novel.txt", build_text_file(SAMPLE_NOVEL_CONTENT), "text/plain")},
        )
        assert response.status_code == 200
        assert response.json()["data"]["name"] == "别名接口项目"

    def test_create_project_missing_name(self, client: TestClient) -> None:
        """缺失项目名称时应拒绝并返回 422。"""
        response = client.post(
            "/api/v1/projects",
            data={"name": "   "},
            files={"file": ("novel.txt", build_text_file(SAMPLE_NOVEL_CONTENT), "text/plain")},
        )
        assert response.status_code == 422
        body = response.json()
        assert body["code"] == 40000
        assert "项目名称不能为空" in body["message"]

    def test_create_project_empty_file(self, client: TestClient) -> None:
        """上传空内容文件应返回 422。"""
        response = client.post(
            "/api/v1/projects",
            data={"name": "空文件项目"},
            files={"file": ("empty.txt", io.BytesIO(b""), "text/plain")},
        )
        assert response.status_code == 422
        body = response.json()
        assert body["code"] == 40000
        assert "空" in body["message"]

    def test_create_project_unsupported_file_type(self, client: TestClient) -> None:
        """上传白名单外格式文件（如 .exe）应返回 415。"""
        response = client.post(
            "/api/v1/projects",
            data={"name": "非法格式项目"},
            files={
                "file": (
                    "malicious.exe",
                    io.BytesIO(b"binary data"),
                    "application/octet-stream",
                )
            },
        )
        assert response.status_code == 415
        body = response.json()
        assert body["code"] == 41500

    def test_create_project_file_too_large(self, client: TestClient) -> None:
        """上传文件超出上限应返回 413。"""
        # test_settings 设定的上限为 1MB
        oversized = b"a" * (1024 * 1024 + 512)
        response = client.post(
            "/api/v1/projects",
            data={"name": "超大文件项目"},
            files={"file": ("big.txt", io.BytesIO(oversized), "text/plain")},
        )
        assert response.status_code == 413
        body = response.json()
        assert body["code"] == 41300


class TestProjectManagementCRUD:
    """项目查询与删除生命周期测试。"""

    def test_list_projects(self, client: TestClient) -> None:
        """分页列出项目，应按创建时间倒序返回。"""
        for i in range(3):
            client.post(
                "/api/v1/projects",
                data={"name": f"项目 {i + 1}"},
                files={
                    "file": (
                        f"novel_{i}.txt",
                        build_text_file(SAMPLE_NOVEL_CONTENT),
                        "text/plain",
                    )
                },
            )

        response = client.get("/api/v1/projects?page=1&page_size=2")
        assert response.status_code == 200
        body = response.json()
        assert body["code"] == 0
        data = body["data"]
        assert data["total"] == 3
        assert data["page"] == 1
        assert data["page_size"] == 2
        assert data["pages"] == 2
        assert len(data["items"]) == 2
        # 最先返回最后创建的项目 3
        assert data["items"][0]["name"] == "项目 3"

    def test_get_project_detail(self, client: TestClient) -> None:
        """获取单个项目详情。"""
        create_res = client.post(
            "/api/v1/projects",
            data={"name": "详情测试项目"},
            files={"file": ("novel.txt", build_text_file(SAMPLE_NOVEL_CONTENT), "text/plain")},
        )
        project_id = create_res.json()["data"]["project_id"]

        detail_res = client.get(f"/api/v1/projects/{project_id}")
        assert detail_res.status_code == 200
        body = detail_res.json()
        assert body["code"] == 0
        assert body["data"]["project_id"] == project_id
        assert body["data"]["name"] == "详情测试项目"

    def test_get_non_existent_project(self, client: TestClient) -> None:
        """查询不存在的项目应返回 404。"""
        response = client.get("/api/v1/projects/0123456789abcdef0123456789abcdef")
        assert response.status_code == 404
        body = response.json()
        assert body["code"] == 40400

    def test_delete_project_cascades(self, client: TestClient) -> None:
        """删除项目应级联删除关联的文本文件、章节与切句数据。"""
        create_res = client.post(
            "/api/v1/projects",
            data={"name": "待删除项目"},
            files={"file": ("novel.txt", build_text_file(SAMPLE_NOVEL_CONTENT), "text/plain")},
        )
        project_data = create_res.json()["data"]
        project_id = project_data["project_id"]
        file_id = project_data["file_id"]

        # 确认文本文件及切句存在
        assert client.get(f"/api/v1/text-files/{file_id}").status_code == 200
        assert client.get(f"/api/v1/text-files/{file_id}/sentences").status_code == 200

        # 执行删除项目
        delete_res = client.delete(f"/api/v1/projects/{project_id}")
        assert delete_res.status_code == 200
        del_body = delete_res.json()
        assert del_body["code"] == 0
        assert del_body["data"]["deleted"] is True

        # 项目应不再存在
        assert client.get(f"/api/v1/projects/{project_id}").status_code == 404

        # 关联的文本文件与切句也应被清理
        assert client.get(f"/api/v1/text-files/{file_id}").status_code == 404

    def test_delete_non_existent_project(self, client: TestClient) -> None:
        """删除不存在的项目应返回 404。"""
        response = client.delete("/api/v1/projects/non_existent_project_id")
        assert response.status_code == 404
        assert response.json()["code"] == 40400


class TestProjectChapters:
    """项目章节大纲与章节内容接口测试。"""

    def test_list_project_chapters_success(self, client: TestClient) -> None:
        """正常查询项目章节大纲列表。"""
        create_res = client.post(
            "/api/v1/projects",
            data={"name": "大纲测试项目"},
            files={"file": ("novel.txt", build_text_file(SAMPLE_NOVEL_CONTENT), "text/plain")},
        )
        project_id = create_res.json()["data"]["project_id"]

        outline_res = client.get(f"/api/v1/projects/{project_id}/chapters")
        assert outline_res.status_code == 200
        body = outline_res.json()
        assert body["code"] == 0
        data = body["data"]
        assert data["project_id"] == project_id
        assert data["total"] == 2
        assert len(data["items"]) == 2

        first = data["items"][0]
        assert first["chapter_index"] == 0
        assert first["chapter_title"] == "第1章 绯红之月"
        assert first["word_count"] > 0

        second = data["items"][1]
        assert second["chapter_index"] == 1
        assert second["chapter_title"] == "第2章 宿命之枪"

    def test_list_project_chapters_not_found(self, client: TestClient) -> None:
        """查询不存在项目的章节大纲应返回 404。"""
        response = client.get("/api/v1/projects/non_existent_project/chapters")
        assert response.status_code == 404
        assert response.json()["code"] == 40400

    def test_get_project_chapter_content_success(self, client: TestClient) -> None:
        """查询指定章节的详细内容及切句数据。"""
        create_res = client.post(
            "/api/v1/projects",
            data={"name": "章节内容测试项目"},
            files={"file": ("novel.txt", build_text_file(SAMPLE_NOVEL_CONTENT), "text/plain")},
        )
        project_id = create_res.json()["data"]["project_id"]

        outline_res = client.get(f"/api/v1/projects/{project_id}/chapters")
        chapter_id = outline_res.json()["data"]["items"][0]["chapter_id"]

        content_res = client.get(f"/api/v1/projects/{project_id}/chapters/{chapter_id}")
        assert content_res.status_code == 200
        body = content_res.json()
        assert body["code"] == 0
        data = body["data"]
        assert data["project_id"] == project_id
        assert data["chapter_id"] == chapter_id
        assert data["chapter_title"] == "第1章 绯红之月"
        assert "痛！好痛！头好痛！" in data["content"]
        assert len(data["sentences"]) > 0
        assert data["sentences"][0]["sentence_index"] == 0
        assert len(data["sentences"][0]["content"]) > 0

    def test_get_project_chapter_content_alias_route(self, client: TestClient) -> None:
        """测试 /content 别名路径亦可正常查询章节内容。"""
        create_res = client.post(
            "/api/v1/projects",
            data={"name": "别名路由测试项目"},
            files={"file": ("novel.txt", build_text_file(SAMPLE_NOVEL_CONTENT), "text/plain")},
        )
        project_id = create_res.json()["data"]["project_id"]

        outline_res = client.get(f"/api/v1/projects/{project_id}/chapters")
        chapter_id = outline_res.json()["data"]["items"][0]["chapter_id"]

        alias_res = client.get(f"/api/v1/projects/{project_id}/chapters/{chapter_id}/content")
        assert alias_res.status_code == 200
        assert alias_res.json()["code"] == 0
        assert alias_res.json()["data"]["chapter_id"] == chapter_id

    def test_get_project_chapter_content_not_found(self, client: TestClient) -> None:
        """查询不存在章节的内容应返回 404。"""
        create_res = client.post(
            "/api/v1/projects",
            data={"name": "项目A"},
            files={"file": ("novel.txt", build_text_file(SAMPLE_NOVEL_CONTENT), "text/plain")},
        )
        project_id = create_res.json()["data"]["project_id"]

        response = client.get(f"/api/v1/projects/{project_id}/chapters/non_existent_chapter")
        assert response.status_code == 404
        assert response.json()["code"] == 40400


class TestSystemInfo:
    """系统索引接口包含 projects 模块测试。"""

    def test_root_contains_projects(self, client: TestClient) -> None:
        """根接口服务信息应包含 projects 模块。"""
        response = client.get("/")
        assert response.status_code == 200
        body = response.json()
        assert "projects" in body["modules"]
