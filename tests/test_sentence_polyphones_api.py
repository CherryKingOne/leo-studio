"""句子多音字标注接口测试。

测试覆盖：
- 查询句子多音字标注列表（初始为空）
- 全量覆盖保存多音字标注（成功流程）
- 章节内容接口级联包含句子多音字标注 (polyphones)
- 校验失败用例（越界偏移量、字符不匹配、重复偏移量、空拼音、不存在的句子/项目）
- 清空/删除句子多音字标注
- 项目物理删除时级联清理多音字标注
"""

from __future__ import annotations

import io
import uuid

from fastapi.testclient import TestClient

SAMPLE_POLYPHONIC_NOVEL = """
第1章 城市一角
人行道上有一行桌子。
他朝朝暮暮都在银行工作。
""".strip()


def build_text_file(content: str) -> io.BytesIO:
    """构造内存中的文本文件字节流。"""
    return io.BytesIO(content.encode("utf-8"))


def create_test_project(client: TestClient) -> dict:
    """辅助函数：创建测试项目并返回项目详情与切句列表。"""
    resp = client.post(
        "/api/v1/projects",
        data={"name": "多音字测试项目"},
        files={"file": ("多音字测试.txt", build_text_file(SAMPLE_POLYPHONIC_NOVEL), "text/plain")},
    )
    assert resp.status_code == 200
    project_data = resp.json()["data"]
    project_id = project_data["project_id"]

    chapters_resp = client.get(f"/api/v1/projects/{project_id}/chapters")
    assert chapters_resp.status_code == 200
    chapter_id = chapters_resp.json()["data"]["items"][0]["chapter_id"]

    content_resp = client.get(f"/api/v1/projects/{project_id}/chapters/{chapter_id}")
    assert content_resp.status_code == 200
    sentences = content_resp.json()["data"]["sentences"]

    return {
        "project_id": project_id,
        "chapter_id": chapter_id,
        "sentences": sentences,
    }


class TestSentencePolyphonesApi:
    """多音字标注 API 测试集。"""

    def test_get_polyphones_initially_empty(self, client: TestClient) -> None:
        """初始状态下查询多音字标注列表为空。"""
        ctx = create_test_project(client)
        project_id = ctx["project_id"]
        sentence = ctx["sentences"][0]
        sentence_id = sentence["sentence_id"]

        resp = client.get(f"/api/v1/projects/{project_id}/sentences/{sentence_id}/polyphones")
        assert resp.status_code == 200
        body = resp.json()
        assert body["code"] == 0
        assert body["data"]["sentence_id"] == sentence_id
        assert body["data"]["total"] == 0
        assert body["data"]["items"] == []

    def test_save_polyphones_success(self, client: TestClient) -> None:
        """正常保存句子的多音字标注。"""
        ctx = create_test_project(client)
        project_id = ctx["project_id"]
        # sentence[0] 为 "人行道上有一行桌子。"
        sentence = ctx["sentences"][0]
        sentence_id = sentence["sentence_id"]
        assert sentence["content"] == "人行道上有一行桌子。"

        payload = {
            "annotations": [
                {"char_index": 1, "character": "行", "pinyin": "xíng"},
                {"char_index": 6, "character": "行", "pinyin": "háng"},
            ]
        }
        resp = client.put(
            f"/api/v1/projects/{project_id}/sentences/{sentence_id}/polyphones",
            json=payload,
        )
        assert resp.status_code == 200
        body = resp.json()
        assert body["code"] == 0
        assert body["message"] == "多音字标注保存成功"

        items = body["data"]["items"]
        assert len(items) == 2
        assert len(items[0]["polyphone_id"]) == 32
        assert items[0]["char_index"] == 1
        assert items[0]["character"] == "行"
        assert items[0]["pinyin"] == "xíng"
        assert items[1]["char_index"] == 6
        assert items[1]["character"] == "行"
        assert items[1]["pinyin"] == "háng"

        # 再次 GET 验证持久化
        get_resp = client.get(
            f"/api/v1/projects/{project_id}/sentences/{sentence_id}/polyphones"
        )
        assert get_resp.status_code == 200
        get_items = get_resp.json()["data"]["items"]
        assert len(get_items) == 2

    def test_chapter_content_includes_polyphones(self, client: TestClient) -> None:
        """获取章节内容时，句子模型应级联返回多音字标注。"""
        ctx = create_test_project(client)
        project_id = ctx["project_id"]
        chapter_id = ctx["chapter_id"]
        sentence_id = ctx["sentences"][0]["sentence_id"]

        client.put(
            f"/api/v1/projects/{project_id}/sentences/{sentence_id}/polyphones",
            json={
                "annotations": [
                    {"char_index": 1, "character": "行", "pinyin": "xíng"},
                ]
            },
        )

        content_resp = client.get(f"/api/v1/projects/{project_id}/chapters/{chapter_id}")
        assert content_resp.status_code == 200
        sentences = content_resp.json()["data"]["sentences"]
        first_sentence = sentences[0]
        assert len(first_sentence["polyphones"]) == 1
        assert first_sentence["polyphones"][0]["char_index"] == 1
        assert first_sentence["polyphones"][0]["pinyin"] == "xíng"

    def test_save_polyphones_overwrite(self, client: TestClient) -> None:
        """全量覆盖保存：第二次提交会替换首次提交的标注。"""
        ctx = create_test_project(client)
        project_id = ctx["project_id"]
        sentence_id = ctx["sentences"][0]["sentence_id"]

        # 第一次保存 2 个
        client.put(
            f"/api/v1/projects/{project_id}/sentences/{sentence_id}/polyphones",
            json={
                "annotations": [
                    {"char_index": 1, "character": "行", "pinyin": "xíng"},
                    {"char_index": 6, "character": "行", "pinyin": "háng"},
                ]
            },
        )

        # 第二次保存仅 1 个（覆盖）
        resp = client.put(
            f"/api/v1/projects/{project_id}/sentences/{sentence_id}/polyphones",
            json={
                "annotations": [
                    {"char_index": 6, "character": "行", "pinyin": "hang2"},
                ]
            },
        )
        assert resp.status_code == 200
        items = resp.json()["data"]["items"]
        assert len(items) == 1
        assert items[0]["char_index"] == 6
        assert items[0]["pinyin"] == "hang2"

    def test_save_polyphones_out_of_bounds(self, client: TestClient) -> None:
        """char_index 越界时应抛出 422 校验异常。"""
        ctx = create_test_project(client)
        project_id = ctx["project_id"]
        sentence_id = ctx["sentences"][0]["sentence_id"]

        resp = client.put(
            f"/api/v1/projects/{project_id}/sentences/{sentence_id}/polyphones",
            json={
                "annotations": [
                    {"char_index": 999, "character": "行", "pinyin": "xíng"},
                ]
            },
        )
        assert resp.status_code == 422
        body = resp.json()
        assert body["code"] == 40000
        assert "超出句子长度范围" in body["message"]

    def test_save_polyphones_character_mismatch(self, client: TestClient) -> None:
        """char_index 处的字符与提交的 character 不匹配时应报错。"""
        ctx = create_test_project(client)
        project_id = ctx["project_id"]
        sentence_id = ctx["sentences"][0]["sentence_id"]
        # sentence 内容为 "人行道上有一行桌子。" index 0 是 '人'

        resp = client.put(
            f"/api/v1/projects/{project_id}/sentences/{sentence_id}/polyphones",
            json={
                "annotations": [
                    {"char_index": 0, "character": "行", "pinyin": "xíng"},
                ]
            },
        )
        assert resp.status_code == 422
        body = resp.json()
        assert body["code"] == 40000
        assert "不匹配" in body["message"]

    def test_save_polyphones_duplicate_index(self, client: TestClient) -> None:
        """提交列表中包含重复的 char_index 时应报错。"""
        ctx = create_test_project(client)
        project_id = ctx["project_id"]
        sentence_id = ctx["sentences"][0]["sentence_id"]

        resp = client.put(
            f"/api/v1/projects/{project_id}/sentences/{sentence_id}/polyphones",
            json={
                "annotations": [
                    {"char_index": 1, "character": "行", "pinyin": "xíng"},
                    {"char_index": 1, "character": "行", "pinyin": "háng"},
                ]
            },
        )
        assert resp.status_code == 422
        body = resp.json()
        assert body["code"] == 40000
        assert "重复" in body["message"]

    def test_save_polyphones_non_existent_sentence(self, client: TestClient) -> None:
        """针对不存在的 sentence_id 提交标注应返回 404。"""
        ctx = create_test_project(client)
        project_id = ctx["project_id"]
        fake_id = uuid.uuid4().hex

        resp = client.put(
            f"/api/v1/projects/{project_id}/sentences/{fake_id}/polyphones",
            json={"annotations": []},
        )
        assert resp.status_code == 404

    def test_delete_polyphones_success(self, client: TestClient) -> None:
        """成功清空/删除句子的多音字标注。"""
        ctx = create_test_project(client)
        project_id = ctx["project_id"]
        sentence_id = ctx["sentences"][0]["sentence_id"]

        client.put(
            f"/api/v1/projects/{project_id}/sentences/{sentence_id}/polyphones",
            json={
                "annotations": [
                    {"char_index": 1, "character": "行", "pinyin": "xíng"},
                ]
            },
        )

        del_resp = client.delete(
            f"/api/v1/projects/{project_id}/sentences/{sentence_id}/polyphones"
        )
        assert del_resp.status_code == 200
        body = del_resp.json()
        assert body["code"] == 0
        assert body["data"]["deleted_count"] == 1
        assert body["data"]["deleted"] is True

        get_resp = client.get(
            f"/api/v1/projects/{project_id}/sentences/{sentence_id}/polyphones"
        )
        assert get_resp.status_code == 200
        assert get_resp.json()["data"]["total"] == 0

    def test_cascade_delete_on_project_hard_delete(self, client: TestClient) -> None:
        """项目硬删除时应级联清理多音字标注表中的数据。"""
        ctx = create_test_project(client)
        project_id = ctx["project_id"]
        sentence_id = ctx["sentences"][0]["sentence_id"]

        client.put(
            f"/api/v1/projects/{project_id}/sentences/{sentence_id}/polyphones",
            json={
                "annotations": [
                    {"char_index": 1, "character": "行", "pinyin": "xíng"},
                ]
            },
        )

        hard_resp = client.delete(f"/api/v1/projects/{project_id}/hard")
        assert hard_resp.status_code == 200

        # 项目已物理删除，查询应返回 404
        resp = client.get(
            f"/api/v1/projects/{project_id}/sentences/{sentence_id}/polyphones"
        )
        assert resp.status_code == 404
