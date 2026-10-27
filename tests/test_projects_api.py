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


class TestAppendProjectChapters:
    """向项目追加上传章节接口测试。"""

    def test_append_new_chapters_success(self, client: TestClient) -> None:
        """纯增量追加新章节：已有 1~2 章，追加 3~4 章。"""
        create_res = client.post(
            "/api/v1/projects",
            data={"name": "连载小说项目"},
            files={"file": ("novel.txt", build_text_file(SAMPLE_NOVEL_CONTENT), "text/plain")},
        )
        assert create_res.status_code == 200
        project_id = create_res.json()["data"]["project_id"]

        increment_content = """
第3章 塔罗之会
愚者先生坐在斑驳长桌的最上首。

第4章 占卜家
魔药在胃里翻滚，灵性视觉开启。
""".strip()

        append_res = client.post(
            f"/api/v1/projects/{project_id}/chapters/upload",
            files={"file": ("increment.txt", build_text_file(increment_content), "text/plain")},
        )
        assert append_res.status_code == 200
        body = append_res.json()
        assert body["code"] == 0
        data = body["data"]
        assert data["project_id"] == project_id
        assert data["added_chapters"] == 2
        assert data["skipped_chapters"] == 0
        assert data["total_chapters"] == 4
        assert data["total_words"] > 0

        # 验证大纲接口现在能够查到 4 个章节，且序号连续自增
        chapters_res = client.get(f"/api/v1/projects/{project_id}/chapters")
        outline = chapters_res.json()["data"]
        assert outline["total"] == 4
        assert [c["chapter_index"] for c in outline["items"]] == [0, 1, 2, 3]
        assert outline["items"][2]["chapter_title"] == "第3章 塔罗之会"
        assert outline["items"][3]["chapter_title"] == "第4章 占卜家"

    def test_append_anti_pollution_skips_existing_by_default(self, client: TestClient) -> None:
        """核心防污染测试：默认 overwrite_existing=false 时，自动跳过已有章节，保留原正文。"""
        create_res = client.post(
            "/api/v1/projects",
            data={"name": "防污染测试项目"},
            files={"file": ("novel.txt", build_text_file(SAMPLE_NOVEL_CONTENT), "text/plain")},
        )
        project_id = create_res.json()["data"]["project_id"]

        # 获取第 1 章的原始 chapter_id
        initial_outline = client.get(f"/api/v1/projects/{project_id}/chapters").json()["data"]
        ch1_id = initial_outline["items"][0]["chapter_id"]
        ch1_detail = client.get(f"/api/v1/projects/{project_id}/chapters/{ch1_id}").json()["data"]
        original_content = ch1_detail["content"]
        assert "狂暴的剧痛如同一柄烧红的钢铁长矛" in original_content

        # 用户从劣质第三方网站爬取了 1~3 章，其中第 1、2 章包含垃圾广告脏数据
        dirty_crawled_content = """
第1章 绯红之月
【澳门首家线上赌场上线啦！点击领取金币】此处章节文本被爬虫污染损坏！

第2章 宿命之枪
【广告插入】劣质爬虫脏数据！

第3章 真正的新章节
这是合法的第3章正文内容。
""".strip()

        append_res = client.post(
            f"/api/v1/projects/{project_id}/chapters/upload",
            files={
                "file": ("crawled_1_to_3.txt", build_text_file(dirty_crawled_content), "text/plain")
            },
            data={"overwrite_existing": "false"},
        )
        assert append_res.status_code == 200
        data = append_res.json()["data"]
        # 应该跳过 1、2 章，仅新增第 3 章
        assert data["added_chapters"] == 1
        assert data["skipped_chapters"] == 2
        assert data["total_chapters"] == 3

        # 核心验证：检查已有第 1 章的正文，确认没有被脏数据覆盖！
        preserved_detail = client.get(
            f"/api/v1/projects/{project_id}/chapters/{ch1_id}"
        ).json()["data"]
        assert preserved_detail["content"] == original_content
        assert "澳门首家线上赌场" not in preserved_detail["content"]
        assert "狂暴的剧痛如同一柄烧红的钢铁长矛" in preserved_detail["content"]

    def test_append_chinese_numeral_normalization_matching(self, client: TestClient) -> None:
        """测试中文数字与阿拉伯数字归一化匹配：'第一章' 与 '第1章' 能够正确识别为相同章节并跳过。"""
        create_res = client.post(
            "/api/v1/projects",
            data={"name": "归一化测试项目"},
            files={"file": ("novel.txt", build_text_file(SAMPLE_NOVEL_CONTENT), "text/plain")},
        )
        project_id = create_res.json()["data"]["project_id"]

        # 上传使用了中文大写“第一章”和“第二章”的文件，外加第 3 章
        mixed_format_content = """
第一章 绯红之月（标题格式略有差异）
重写的第一章正文。

第二章 宿命之枪
重写的第二章正文。

第三章 全新旅程
全新的第三章。
""".strip()

        append_res = client.post(
            f"/api/v1/projects/{project_id}/chapters/upload",
            files={
                "file": ("chinese_nums.txt", build_text_file(mixed_format_content), "text/plain")
            },
        )
        assert append_res.status_code == 200
        data = append_res.json()["data"]
        # 命中归一化指纹 ch_1 与 ch_2，成功跳过前 2 章
        assert data["skipped_chapters"] == 2
        assert data["added_chapters"] == 1
        assert data["total_chapters"] == 3

    def test_append_with_overwrite_existing_true(self, client: TestClient) -> None:
        """当显式指定 overwrite_existing=true 时，覆盖更新已有章节正文并重切句子。"""
        create_res = client.post(
            "/api/v1/projects",
            data={"name": "强制覆盖测试项目"},
            files={"file": ("novel.txt", build_text_file(SAMPLE_NOVEL_CONTENT), "text/plain")},
        )
        project_id = create_res.json()["data"]["project_id"]
        chapters_data = client.get(f"/api/v1/projects/{project_id}/chapters").json()["data"]
        ch1_id = chapters_data["items"][0]["chapter_id"]

        updated_novel = """
第1章 绯红之月
作者精校重写版：周明瑞从睡梦中惊醒，窗外照耀着一轮绯红色的满月。
""".strip()

        append_res = client.post(
            f"/api/v1/projects/{project_id}/chapters/upload",
            files={"file": ("refined.txt", build_text_file(updated_novel), "text/plain")},
            data={"overwrite_existing": "true"},
        )
        assert append_res.status_code == 200
        data = append_res.json()["data"]
        assert data["skipped_chapters"] == 1
        assert data["added_chapters"] == 0

        # 验证第 1 章正文已更新为精校版
        refined_detail = client.get(
            f"/api/v1/projects/{project_id}/chapters/{ch1_id}"
        ).json()["data"]
        assert "作者精校重写版：周明瑞从睡梦中惊醒" in refined_detail["content"]


    def test_append_chapters_not_found(self, client: TestClient) -> None:
        """向不存在的项目追加章节应返回 404。"""
        response = client.post(
            "/api/v1/projects/non_existent_project/chapters/upload",
            files={"file": ("inc.txt", build_text_file("第1章 新章\n正文"), "text/plain")},
        )
        assert response.status_code == 404
        assert response.json()["code"] == 40400

    def test_append_chapters_empty_file(self, client: TestClient) -> None:
        """上传空内容文件应返回 422 错误。"""
        create_res = client.post(
            "/api/v1/projects",
            data={"name": "空文件测试项目"},
            files={"file": ("novel.txt", build_text_file(SAMPLE_NOVEL_CONTENT), "text/plain")},
        )
        project_id = create_res.json()["data"]["project_id"]

        response = client.post(
            f"/api/v1/projects/{project_id}/chapters/upload",
            files={"file": ("empty.txt", build_text_file(""), "text/plain")},
        )
        assert response.status_code == 422
        assert response.json()["code"] == 40000


    def test_append_with_dry_run_and_conflicts(self, client: TestClient) -> None:
        """测试 dry_run 预检模式：检测内容差异冲突，且不写入实际数据。"""
        create_res = client.post(
            "/api/v1/projects",
            data={"name": "预检测试项目"},
            files={"file": ("novel.txt", build_text_file(SAMPLE_NOVEL_CONTENT), "text/plain")},
        )
        project_id = create_res.json()["data"]["project_id"]

        # 上传修改了字数的第 1 章与新第 3 章，带有 dry_run=true
        modified_content = """
第1章 绯红之月
极短正文。

第3章 新世界
崭新的一章。
""".strip()

        dry_run_res = client.post(
            f"/api/v1/projects/{project_id}/chapters/upload",
            files={"file": ("modified.txt", build_text_file(modified_content), "text/plain")},
            data={"dry_run": "true"},
        )
        assert dry_run_res.status_code == 200
        body = dry_run_res.json()
        assert body["code"] == 0
        assert "预检完成" in body["message"]
        data = body["data"]
        assert data["has_conflicts"] is True
        assert len(data["conflicts"]) == 1
        assert data["conflicts"][0]["chapter_title"] == "第1章 绯红之月"
        assert data["conflicts"][0]["word_diff"] < 0

        # 核心验证：预检模式下项目章节数应依然保持为初始的 2 章
        chapters_res = client.get(f"/api/v1/projects/{project_id}/chapters")
        assert chapters_res.json()["data"]["total"] == 2


class TestProjectVersionAndRollback:
    """项目历史版本快照、滑动窗口淘汰与一键回退测试。"""

    def test_version_snapshot_and_rollback(self, client: TestClient) -> None:
        """完整测试：版本生成、列表查询、一键回滚。"""
        # 1. 创建项目（自动生成 V1）
        create_res = client.post(
            "/api/v1/projects",
            data={"name": "版本回退测试项目"},
            files={"file": ("v1.txt", build_text_file("第1章 初始章\n原始内容"), "text/plain")},
        )
        assert create_res.status_code == 200
        project_id = create_res.json()["data"]["project_id"]

        # 查询版本列表，应包含 V1
        v_res = client.get(f"/api/v1/projects/{project_id}/versions")
        assert v_res.status_code == 200
        versions = v_res.json()["data"]["items"]
        assert len(versions) == 1
        assert versions[0]["version_number"] == 1
        v1_id = versions[0]["version_id"]

        # 2. 追加第 2 章（生成 V2）
        append_res = client.post(
            f"/api/v1/projects/{project_id}/chapters/upload",
            files={"file": ("v2.txt", build_text_file("第2章 新增章\n第二章正文"), "text/plain")},
        )
        assert append_res.status_code == 200
        assert append_res.json()["data"]["version_number"] == 2

        # 检查当前章节数应为 2
        assert client.get(f"/api/v1/projects/{project_id}/chapters").json()["data"]["total"] == 2

        # 3. 执行一键回滚至 V1
        rollback_res = client.post(
            f"/api/v1/projects/{project_id}/versions/{v1_id}/rollback"
        )
        assert rollback_res.status_code == 200
        r_body = rollback_res.json()
        assert r_body["code"] == 0
        assert r_body["data"]["restored_version_number"] == 1
        assert r_body["data"]["chapter_count"] == 1

        # 验证章节已恢复为仅有 1 章
        outline_res = client.get(f"/api/v1/projects/{project_id}/chapters")
        outline = outline_res.json()["data"]
        assert outline["total"] == 1
        assert outline["items"][0]["chapter_title"] == "第1章 初始章"

    def test_sliding_window_retention_max_3_versions(self, client: TestClient) -> None:
        """测试滑动窗口淘汰机制：保留最近 3 个版本，超出后自动淘汰最旧版本。"""
        create_res = client.post(
            "/api/v1/projects",
            data={"name": "滑动窗口测试项目"},
            files={"file": ("init.txt", build_text_file("第1章 第一\n正文"), "text/plain")},
        )
        project_id = create_res.json()["data"]["project_id"]

        # 连续追加 3 次，加上初始创建总共产生 4 个版本操作
        for i in range(2, 5):
            res = client.post(
                f"/api/v1/projects/{project_id}/chapters/upload",
                files={
                    "file": (
                        f"inc_{i}.txt",
                        build_text_file(f"第{i}章 第{i}篇\n正文"),
                        "text/plain",
                    )
                },
            )
            assert res.status_code == 200

        # 查询版本快照列表，应被严格封顶在 3 个版本（MAX_PROJECT_VERSIONS）
        v_res = client.get(f"/api/v1/projects/{project_id}/versions")
        versions = v_res.json()["data"]["items"]
        assert len(versions) == 3
        # 保留的是最新的版本（4, 3, 2），最老的版本 1 已被安全淘汰
        version_numbers = [v["version_number"] for v in versions]
        assert version_numbers == [4, 3, 2]

    def test_rollback_non_existent_version(self, client: TestClient) -> None:
        """回退不存在的版本应返回 404。"""
        create_res = client.post(
            "/api/v1/projects",
            data={"name": "错误测试项目"},
            files={"file": ("a.txt", build_text_file("第1章 一\n文"), "text/plain")},
        )
        project_id = create_res.json()["data"]["project_id"]

        res = client.post(
            f"/api/v1/projects/{project_id}/versions/non_existent_id/rollback"
        )
        assert res.status_code == 404
        assert res.json()["code"] == 40400


class TestSystemInfo:


    """系统索引接口包含 projects 模块测试。"""

    def test_root_contains_projects(self, client: TestClient) -> None:
        """根接口服务信息应包含 projects 模块。"""
        response = client.get("/")
        assert response.status_code == 200
        body = response.json()
        assert "projects" in body["modules"]
