use leo_app::text::kaomoji::{clean_line_text, strip_kaomoji};
use leo_app::text::parsers::{clean_text, count_words, parse_chapters};
use leo_app::text::processor::split_sentences_by_lines;

const CLEANABLE_KAOMOJI: &[&str] = &[
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
];

#[test]
fn removes_standalone_kaomoji() {
    for kaomoji in CLEANABLE_KAOMOJI {
        assert_eq!(clean_text(kaomoji), "", "should remove {kaomoji}");
        assert_eq!(
            clean_text(&format!("第一行正文。\n{kaomoji}\n第二行正文。")),
            "第一行正文。\n第二行正文。",
            "should remove inline line {kaomoji}"
        );
        assert_eq!(
            split_sentences_by_lines(&format!("第一句。\n{kaomoji}\n第二句。")),
            vec!["第一句。", "第二句。"],
            "sentence split should remove {kaomoji}"
        );
    }
}

#[test]
fn removes_inline_kaomoji() {
    let cases = [
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
    ];

    for (raw, expected) in cases {
        assert_eq!(clean_line_text(raw), expected, "line {raw}");
        assert_eq!(strip_kaomoji(raw), expected, "strip {raw}");
    }
}

#[test]
fn preserves_normal_text() {
    let lines = [
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
    ];

    for line in lines {
        assert_eq!(clean_line_text(line), line, "line {line}");
        assert_eq!(strip_kaomoji(line), line, "strip {line}");
    }
}

#[test]
fn preserves_foreign_language() {
    for text in [
        "「これはテストです」",
        "Привет, мир!",
        "Καλημέρα κόσμε",
        "안녕하세요",
        "สวัสดีครับ",
    ] {
        assert_eq!(strip_kaomoji(text), text);
    }
}

#[test]
fn cleans_ellipsis_and_separators() {
    let raw =
        "第一行正文。\n……\n　　“……”\n\"......\"\n‘……’\n第二行正文。\n林风眠：“……”\n第三行正文……";
    assert_eq!(
        clean_text(raw),
        "第一行正文。\n第二行正文。\n林风眠：“……”\n第三行正文……"
    );
}

#[test]
fn splits_chapters() {
    let text = "第1章 开始\n正文一\n第2章 继续\n正文二";
    let chapters = parse_chapters(text);
    assert_eq!(chapters.len(), 2);
    assert_eq!(chapters[0].0, "第1章 开始");
    assert_eq!(chapters[0].1, "正文一");
    assert_eq!(chapters[1].0, "第2章 继续");
    assert_eq!(chapters[1].1, "正文二");
    assert_eq!(count_words("你好世界。"), 5);
}
