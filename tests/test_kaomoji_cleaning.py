"""颜文字数据清洗测试。

覆盖：
- 独立成行的颜文字（含带汉字部件的颜文字）；
- 汉字与颜文字同行的片段删除；
- 正常正文、标点、公式、外文与括号内容的误伤保护；
- 正文清洗与切句清洗的一致性。
"""

from __future__ import annotations

import pytest

from app.modules.text_files.parsers import (
    clean_line_text,
    clean_text,
    is_standalone_symbol_line,
    strip_kaomoji,
)
from app.modules.text_files.processor import split_sentences_by_lines

#: 应被整体删除的独立颜文字行（覆盖用户提供颜文字大全中的主要家族）。
CLEANABLE_KAOMOJI = [
    # 打招呼 / 爱心 / 猫爪
    "╭(●｀∀´●)╯╰(●’◡’●)╮",
    "(●’◡’●)ﾉ",
    "ヾ(*´▽‘*)ﾉ",
    "╭(′▽`)╭(′▽`)╯",
    "(ฅ´ω`ฅ)",
    "( ˘ ³˘)♥",
    "(๑ơ ₃ ơ)ﻌﻌﻌ♥",
    "ε٩(๑> ₃ <)۶з",
    "ʕ•̀ω•́ʔ✧",
    "ლ(◉益◉ლ)",
    "(๑•̀ㅂ•́)و✧",
    # 开心 / 卖萌 / 害羞
    "｡:.ﾟヽ(｡◕‿◕｡)ﾉﾟ.:｡+ﾟ",
    "♪（＾∀＾●）ﾉｼ",
    "(๑•́ ₃•̀๑٥)",
    "(●′ω`●)",
    "(´,,•ω•,,‘)",
    "(๑•ั็ω•็ั๑)",
    "(๑˘ ˘๑)",
    "(●'◡'●)ﾉ♥",
    "（//▽//）",
    "๑乛◡乛๑",
    # 困 / 汗 / 无奈 / 惊讶
    "_(:з」∠)_",
    "_(┐「ε:)_",
    "(:3[▓▓]",
    "(¦3[▓▓]",
    "٩(๑´0`๑)۶",
    "(;¬_¬)",
    "(눈_눈)",
    "( ∙̆ .̯ ∙̆ )",
    "（⊙.⊙）",
    "Σ(っ °Д °;)っ",
    "Σ( ° △ °|||)︴",
    "ᕙ(⇀‸↼‶)ᕗ",
    "( ͡° ͜ʖ ͡°)",
    # 哭 / 生气 / 掀桌
    "π__π",
    "T.T",
    "ε(┬┬＿┬┬)3",
    "╥﹏╥",
    "(╥╯^╰╥)",
    "(╯°Д°)╯︵ ┻━┻",
    "(ノ=Д=)ノ┻━┻",
    "（╯－＿－）╯╧╧",
    "（╯#-_-)╯╧═╧",
    "(╯’ – ‘)╯︵ ┻━┻",
    "(┙>∧<)┙へ┻┻",
    "凸(艹皿艹 )",
    "(╬▔皿▔)凸",
    "（╯#-皿-)╯~~╧═╧",
    # 其他组合 / 包裹符号
    "╰(*°▽°*)╯",
    "(๑>◡<๑)۶",
    "(๑•̀ㅁ•́ฅ)ฅ",
    "(๑꒪◞౪◟꒪๑)",
    "(๑◕ܫ￩๑)",
    "(´・ω・｀)",
    "(´；ω；`)",
    "(๑＞ڡ＜)☆",
    "o(*≧д≦)o!!",
    "ヽ(●-`Д´-)ノ",
    "才ノヽ∋―_φ(≧ω≦*)♪",
    "山(≧皿≦)山",
    "人(・ε・；)",
    "٩(๛˘³˘)۶♥",
    "♫ ♫♬♪♫ ヾ(。◕ฺ∀◕ฺ)ノ♫♬",
]

#: (原始行, 期望清洗结果)
INLINE_CASES = [
    ("他笑了笑(￣▽￣)。", "他笑了笑。"),
    ("“变……变态！”(☼Д☼)", "“变……变态！”"),
    (
        "柳媚有气无力的声音传来：“╭(╯^╰)╮，不去了！你饶了我吧。”",
        "柳媚有气无力的声音传来：“不去了！你饶了我吧。”",
    ),
    (
        "“天邪圣君叶雪枫？(⊙o⊙)哇，能看到活的了，不虚此行啊，这一顿没白飞！”",
        "“天邪圣君叶雪枫？哇，能看到活的了，不虚此行啊，这一顿没白飞！”",
    ),
    (
        "洛雪哼了一声，带小情绪道：“那你还想怎么样？o(´^｀)o”",
        "洛雪哼了一声，带小情绪道：“那你还想怎么样？”",
    ),
    ("他笑了笑o(≧v≦)o！", "他笑了笑！"),
    ("他笑了笑(๑•̀ㅂ•́)و✧", "他笑了笑"),
    ("他笑了^_^", "他笑了"),
    ("他难过了T_T", "他难过了"),
    ("他困了(:3[▓▓]", "他困了"),
    ("他摊了摊手╮(￣▽￣)╭", "他摊了摊手"),
    ("他比了个(๑>◡<๑)۶", "他比了个"),
    ("她脸红了（//▽//）", "她脸红了"),
    ("他叹了口气_(:з」∠)_", "他叹了口气"),
    ("“站住！！！”(艹皿艹)", "“站住！！！”"),
    ("他笑了 (￣▽￣) 啊", "他笑了啊"),
    ("他笑了T_T！", "他笑了！"),
    ("他很有才(￣▽￣)", "他很有才"),
    ("这是正文(￣▽￣)结束。", "这是正文结束。"),
    ("普通英文 He said (￣▽￣) okay.", "普通英文 He said okay."),
]

#: 不应被清洗改动的正常正文。
PRESERVED_LINES = [
    "林风眠：“……”",
    "“你……你说什么？”",
    "他叹了口气——这世道。",
    "这是正文--带有合法连字符",
    "他拿出100元（人民币）。",
    "室外温度 30°C~35°C。",
    "2024年10月23日 20:16:53",
    "第1章 登仙之路",
    "（笑）",
    "（注：此处有删改）",
    "他笑了笑（无奈地）。",
    "价格是 3×4=12 元。",
    "比例为 1:2:3。",
    "他喜欢《斗破苍穹》这本书。",
    "【系统提示】任务完成。",
    "函数 f(x)=x^2+1 在 (0,1) 上连续",
    "（x^2）",
    "变量（a_b）",
    "变量名 foo_bar 不能删",
    "地址：https://example.com/a?b=1&c=2",
    "符号 ★★★ 表示难度",
    "她哼着歌♪♪",
    "中文标点“”，。！？；：、（）《》【】「」『』",
    "他吹奏一曲，曲调悠扬。",
    "她身材凹凸有致。",
    "这次双方都受益。",
    "他惊恐道：“艹，什么玩意？”",
    "选择题（A）B（C）D",
    "箭头 A→B→C",
    "公式 a≤b≥c",
    "0.5%～1.5%",
    "版本 v1.2.3",
    "他用了 `code` 标记",
    "下划线 __init__",
]


class TestStandaloneKaomoji:
    """独立成行的颜文字应被整体删除。"""

    @pytest.mark.parametrize("kaomoji", CLEANABLE_KAOMOJI)
    def test_clean_text_removes_standalone_kaomoji(self, kaomoji: str) -> None:
        assert clean_text(kaomoji) == ""
        assert clean_text(f"第一行正文。\n{kaomoji}\n第二行正文。") == "第一行正文。\n第二行正文。"

    @pytest.mark.parametrize("kaomoji", CLEANABLE_KAOMOJI)
    def test_split_sentences_removes_standalone_kaomoji(self, kaomoji: str) -> None:
        sentences = split_sentences_by_lines(f"第一句。\n{kaomoji}\n第二句。")
        assert sentences == ["第一句。", "第二句。"]

    @pytest.mark.parametrize(
        "kaomoji",
        ["ლ(◉益◉ლ)", "๑乛◡乛๑", "(艹皿艹)", "凸(艹皿艹 )"],
    )
    def test_kaomoji_containing_han_components_is_standalone(self, kaomoji: str) -> None:
        assert is_standalone_symbol_line(kaomoji) is True
        assert clean_line_text(kaomoji) == ""


class TestInlineKaomoji:
    """汉字与颜文字同行时，只删除颜文字片段。"""

    @pytest.mark.parametrize(("raw", "expected"), INLINE_CASES)
    def test_clean_line_text_removes_inline_kaomoji(self, raw: str, expected: str) -> None:
        assert clean_line_text(raw) == expected

    @pytest.mark.parametrize(("raw", "expected"), INLINE_CASES)
    def test_clean_text_removes_inline_kaomoji(self, raw: str, expected: str) -> None:
        assert clean_text(raw) == expected

    @pytest.mark.parametrize(("raw", "expected"), INLINE_CASES)
    def test_split_sentences_removes_inline_kaomoji(self, raw: str, expected: str) -> None:
        assert split_sentences_by_lines(f"第一句。\n{raw}\n第二句。") == [
            "第一句。",
            expected,
            "第二句。",
        ]


class TestNormalTextPreserved:
    """正常正文不能被颜文字清洗误伤。"""

    @pytest.mark.parametrize("line", PRESERVED_LINES)
    def test_normal_lines_are_unchanged(self, line: str) -> None:
        assert clean_line_text(line) == line
        assert strip_kaomoji(line) == line

    @pytest.mark.parametrize(
        "text",
        [
            "「これはテストです」",
            "Привет, мир!",
            "Καλημέρα κόσμε",
            "안녕하세요",
            "สวัสดีครับ",
        ],
    )
    def test_foreign_language_is_not_stripped_as_kaomoji(self, text: str) -> None:
        assert strip_kaomoji(text) == text

    def test_words_containing_face_components_are_preserved(self) -> None:
        """曾因“曲/凸/凹/益/艹”被当作裸颜文字强信号而误删，此处做回归保护。"""
        raw = (
            "他吹奏一曲，曲调悠扬。\n"
            "她身材凹凸有致。\n"
            "眼睛外凸，模样古怪。\n"
            "这次双方都受益。\n"
            "他惊恐道：“艹，什么玩意？”"
        )
        assert clean_text(raw) == raw

    def test_ellipsis_and_quotes_are_preserved(self) -> None:
        raw = "林风眠：“……”\n“你……你说什么？”\n脚步声由远及近……"
        assert clean_text(raw) == raw


class TestCleaningConsistency:
    """正文清洗与切句清洗应使用同一套行级规则。"""

    def test_clean_text_and_sentence_split_are_consistent(self) -> None:
        raw = (
            "第一行正文。\n"
            "……\n"
            "\n"
            "他笑了笑(￣▽￣)。\n"
            "╭(￣▽￣)╯\n"
            "----------\n"
            "第二行正文。\n"
        )
        cleaned_lines = clean_text(raw).splitlines()
        assert cleaned_lines == split_sentences_by_lines(raw)
        assert cleaned_lines == ["第一行正文。", "他笑了笑。", "第二行正文。"]

    @pytest.mark.parametrize(
        "text",
        [
            "他笑了(￣▽￣)。",
            "他笑了T_T",
            "（x^2）",
            "林风眠：“……”",
            "（笑）",
        ],
    )
    def test_strip_kaomoji_is_idempotent(self, text: str) -> None:
        once = strip_kaomoji(text)
        assert strip_kaomoji(once) == once


class TestUploadPipeline:
    """上传落盘链路应使用同一套颜文字清洗规则。"""

    def test_upload_cleans_chapter_contents_and_sentences(
        self, client, csv_dir
    ) -> None:
        import io

        api = "/api/v1/text-files"
        novel_text = (
            "第1章 起程\n"
            "他笑了笑(￣▽￣)。\n"
            "╮(￣▽￣)╭\n"
            "第二句T_T！\n"
        )
        upload = client.post(
            f"{api}/upload",
            files={
                "file": (
                    "清洗测试.txt",
                    io.BytesIO(novel_text.encode("utf-8")),
                    "text/plain",
                )
            },
        )
        assert upload.status_code == 200

        contents = (csv_dir / "novel_contents.csv").read_text(encoding="utf-8")
        sentences = (csv_dir / "novel_sentences.csv").read_text(encoding="utf-8")
        for text in (contents, sentences):
            assert "他笑了笑。" in text
            assert "第二句！" in text
            assert "￣▽￣" not in text
