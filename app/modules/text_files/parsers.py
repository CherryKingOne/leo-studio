"""小说文件名、章节与文本统计解析器。

提供小说文件名元信息解析、章节切分、文本清洗与字数统计能力。
所有函数与类定义均不使用下划线前缀，符合项目代码风格规范。

文本清洗采用“结构识别 + 强信号校验”的方式处理颜文字（kaomoji）：

- 括号型颜文字：先按括号家族做配对，再校验括号内是否只包含颜文字字符，
  并要求存在眼睛/嘴巴/轮廓类“强信号”字符；
- 裸颜文字：匹配不含括号的连续颜文字字符片段，同样要求存在强信号；
- ASCII 颜文字：覆盖 orz、T_T、QAQ、^_^ 等常见写法；
- 独立成行的颜文字行整体删除；汉字与颜文字同行时只删除颜文字片段。

这种设计不依赖穷举具体颜文字，因此能覆盖同一结构下的变体，同时通过
“不得包含普通汉字/英文单词/ASCII 数字”等约束避免误删正文。
"""

from __future__ import annotations

import re
import unicodedata
from pathlib import Path

# 文件名中的字段分隔符：中文双破折号（两个 U+2014）
FIELD_SEPARATOR = "\u2014\u2014"

# 中文数字字符集
CN_NUM = r"零一二三四五六七八九十百千万两"

# 章节标题正则：行首可选空白 + 章节标记 + 可选标题文字到行尾
CHAPTER_HEADER = re.compile(
    r"^[ \t\u3000]*"
    r"(?:"
    r"第\s*[\d" + CN_NUM + r"]+\s*[章回节卷篇集]"
    r"|Chapter\s+\d+"
    r"|第\s*[\d" + CN_NUM + r"]+\s*部分"
    r")"
    r"(?:[ \t\u3000:：．.、\-—]+.*)?$",
    re.IGNORECASE | re.MULTILINE,
)

# 常见可见/不可见空白字符集
# （含空格、制表符、全角空格、换行、换页、垂直制表符、零宽空格与 BOM 等）
BLANK_CHARS = " \t\r\n\u3000\u200b\ufeff\u200e\u200f\f\v"

# 过滤单独占一行的省略号：
# 1. 纯省略号行（如“……”、“......”、“…”、“⋯⋯”、“······”等，支持前后及字符间空白）；
# 2. 带引号包裹的单独省略号行
#    （如““……””、“"......"”、“‘……’”、“「……」”等人物沉默或占位无意义行）。
STANDALONE_ELLIPSIS = re.compile(
    r"^[ \t\u3000]*"
    r"[“\"'‘「『”’]?[ \t\u3000]*"
    r"(?:[.…．⋯‥·•][ \t\u3000]*)+"
    r"[”\"'’」』“‘]?[ \t\u3000]*$"
)

# 过滤单独占一行的分隔线：
# 1. 连续或带空格的横线、破折号、等号、星号、波浪号等分隔线
#    （如 "----------"、"——————"、"====="、"******" 等）；
# 2. 支持前后可选引号包裹（如 “----------”、“"----------"” 等）。
STANDALONE_SEPARATOR = re.compile(
    r"^[ \t\u3000]*"
    r"[“\"'‘「『”’]?[ \t\u3000]*"
    r"(?:[-—–―_－=＝*＊~～#＃][ \t\u3000]*){3,}"
    r"[”\"'’」』“‘]?[ \t\u3000]*$"
)

# ============================================================================
# 颜文字字符集与匹配规则
# ============================================================================

#: 颜文字中可能出现的 Unicode 区段。
#: 这里刻意不包含常用汉字区（4E00-9FFF），普通汉字只有在 FACE_HAN_CHARS
#: 白名单中时才允许出现在括号型颜文字内部。
KAOMOJI_RANGES: tuple[tuple[int, int], ...] = (
    (0x00A1, 0x00BF),  # Latin-1 符号
    (0x00C0, 0x024F),  # 拉丁扩展字母（ŏ 等）
    (0x0250, 0x02AF),  # IPA（ʘ ʖ ʕ ʔ ʅ ʃ ʬ）
    (0x02B0, 0x02FF),  # 修饰符号（ˊ ˋ ˘ ˙ ˚ ˖ ˝ 等）
    (0x0300, 0x036F),  # 组合附加符号（͡ ͜ ̀ ́ 等）
    (0x0370, 0x03FF),  # 希腊字母（ω Θ Φ σ ε 等）
    (0x0400, 0x052F),  # 西里尔字母（д Д з 等）
    (0x0530, 0x058F),  # 亚美尼亚
    (0x0590, 0x05FF),  # 希伯来
    (0x0600, 0x06FF),  # 阿拉伯（٩ ۶）
    (0x0700, 0x074F),  # 叙利亚（ܫ）
    (0x0750, 0x077F),  # 阿拉伯补充
    (0x0900, 0x097F),  # 天城文（१）
    (0x0980, 0x09FF),  # 孟加拉
    (0x0A00, 0x0A7F),  # 古木基
    (0x0A80, 0x0AFF),  # 古吉拉特
    (0x0B00, 0x0B7F),  # 奥里亚（୧ ୨）
    (0x0B80, 0x0BFF),  # 泰米尔（இ）
    (0x0C00, 0x0C7F),  # 泰卢固（౪）
    (0x0C80, 0x0CFF),  # 卡纳达（ಠ ಡ ಥ）
    (0x0D00, 0x0D7F),  # 马拉雅拉姆
    (0x0D80, 0x0DFF),  # 僧伽罗
    (0x0E00, 0x0E7F),  # 泰文（๑ ง ฅ）
    (0x0E80, 0x0EFF),  # 老挝（ຶ）
    (0x0F00, 0x0FFF),  # 藏文（༎）
    (0x1000, 0x109F),  # 缅甸
    (0x10A0, 0x10FF),  # 格鲁吉亚（ლ）
    (0x1100, 0x11FF),  # 谚文字母
    (0x1200, 0x137F),  # 埃塞俄比亚（ꈨ）
    (0x13A0, 0x13FF),  # 切罗基
    (0x1400, 0x167F),  # 加拿大原住民（ᕙ ᕗ）
    (0x1680, 0x169F),  # 欧甘
    (0x16A0, 0x16FF),  # 卢恩
    (0x1700, 0x17FF),  # 高棉
    (0x1800, 0x18AF),  # 蒙古
    (0x1B00, 0x1B7F),  # 巴厘
    (0x1DC0, 0x1DFF),  # 组合附加符号补充
    (0x1E00, 0x1EFF),  # 拉丁扩展附加
    (0x1F00, 0x1FFF),  # 希腊扩展
    (0x2000, 0x206F),  # 通用标点（• ‥ … ‾ ⁀）
    (0x2070, 0x209F),  # 上下标（⁰ ² ³ ₃）
    (0x20A0, 0x20CF),  # 货币符号
    (0x20D0, 0x20FF),  # 组合用符号
    (0x2100, 0x214F),  # 字母式符号（™ Ω ℮）
    (0x2150, 0x218F),  # 数字形式（Ⅰ ①）
    (0x2190, 0x21FF),  # 箭头（→ ⇀ ↼）
    (0x2200, 0x22FF),  # 数学运算符（∀ ∂ ∇ ≧ ≦）
    (0x2300, 0x23FF),  # 技术符号（⌒ ⌓ ⌇）
    (0x2460, 0x24FF),  # 带圈字母数字
    (0x2500, 0x257F),  # 制表符（╮ ╭ ╯ ╰ ┻ ━）
    (0x2580, 0x259F),  # 方块元素（▓ ▒ ░）
    (0x25A0, 0x25FF),  # 几何图形（● ○ ◉ ◕）
    (0x2600, 0x26FF),  # 杂项符号（★ ☆ ♥ ♪ ☼ ⚗）
    (0x2700, 0x27BF),  # 装饰符号（✧ ✿ ❀ ✪）
    (0x27C0, 0x27EF),  # 数学符号 A
    (0x27F0, 0x27FF),  # 补充箭头 A
    (0x2800, 0x28FF),  # 盲文
    (0x2900, 0x297F),  # 补充箭头 B
    (0x2980, 0x29FF),  # 数学符号 B
    (0x2A00, 0x2AFF),  # 补充数学运算符
    (0x2B00, 0x2BFF),  # 杂项符号与箭头
    (0x2E00, 0x2E7F),  # 补充标点
    (0x2E80, 0x2EFF),  # CJK 部首补充
    (0x2F00, 0x2FDF),  # 康熙部首
    (0x3000, 0x303F),  # CJK 标点（。 〃 〆 〇 々）
    (0x3040, 0x309F),  # 平假名
    (0x30A0, 0x30FF),  # 片假名
    (0x3100, 0x312F),  # 注音（ㄟ）
    (0x3130, 0x318F),  # 谚文兼容字母（ㅂ ㅅ ㅠ）
    (0x31A0, 0x31BF),  # 注音扩展
    (0x31C0, 0x31EF),  # CJK 笔画
    (0x31F0, 0x31FF),  # 片假名语音扩展
    (0xAC00, 0xD7AF),  # 谚文音节（눈）
    (0xFE00, 0xFE0F),  # 变体选择符
    (0xFE10, 0xFE1F),  # 竖排形式
    (0xFE20, 0xFE2F),  # 组合半标记
    (0xFE30, 0xFE4F),  # CJK 兼容形式（︵ ︶ ﹏ ﹃）
    (0xFE50, 0xFE6F),  # 小写变体（﹐ ﹑ ﹒）
    (0xFE70, 0xFEFF),  # 阿拉伯展示形式 B
    (0xFF00, 0xFFEF),  # 全角/半角形式（￣ ＿ ﾟ ﾉ ｡）
    (0x1F300, 0x1F5FF),  # 杂项符号和象形文字
    (0x1F600, 0x1F64F),  # 表情符号
    (0x1F900, 0x1F9FF),  # 补充符号
    (0x1FA70, 0x1FAFF),  # 扩展符号
)

#: 允许出现在括号型颜文字内部的汉字部件。
#: 这些字大多由部首、方框或“嘴/眼”形状构成，如 皿、罒、艹、灬、益、曲。
FACE_HAN_CHARS = (
    "乂丿丶丷乛亅一二十人八冖几匕匚卩厂厶又口囗土士夕大女子宀寸小尢尸屮山川"
    "工己巾干广廴廾弋弓彐彡彳心戈户手支攵文斗斤方无日曰月木欠止歹殳毋比毛氏"
    "气水火爪父爻爿片牙牛犬玄玉瓜瓦甘生用田疋疒癶白皮皿目矛矢石示禸禾穴立竹"
    "米糸缶网羊羽而耒耳聿肉臣自至臼舌舛舟艮色虍虫血行衣襾角言谷豆豕豸貝贝赤"
    "走足身車车辛辰辵邑酉釆里金長长門门阜隶隹雨青非面革韋韦韭音頁页風风飛飞"
    "食首香馬马骨高髟鬥斗鬯鬲鬼魚鱼鳥鸟鹵卤鹿麥麦麻黃黄黍黑黹黽黾鼎鼓鼠鼻齊"
    "齐齒齿龍龙龜龟龠罒艹灬益曲凸凹〆々〇"
)

#: 允许出现在“裸颜文字”（不带括号）中的汉字部件。
#: 这里必须严格收窄：人、大、小、了 等常用字一旦放开，会把“了T_T”整段吞掉。
BARE_FACE_HAN_CHARS = "乛皿罒艹灬益曲凸凹〆々〇"

#: 强信号字符：眼睛、嘴巴、轮廓、特殊符号。出现任意一个才可能是颜文字。
STRONG_LITERALS = (
    "∀∂∇≧≦≖￢℥℮℧℩Åℬℭℯℰℱℳℴℹ℺℻ℼℽℾℿ⅀⅁⅂⅃⅄ⅅⅆⅇⅈⅉ"
    "ωΩΘΦσΣεΕλΛμΜπΠ"
    "дДзЗэЭ눈"
    "ʘʖʕʔʅʃʬʭʮʯᴥᴗᴖᵔᵕಠಡಥஇ"
    "⌒⌓⌇⌁⌐¬^_•⇀↼‸⚗⚈ˇˍ＠＞＜‿"
    "╮╭╯╰╬╪╫╤╧╩╦╠╣║╔╗╚╝╒╓╕╖╙╛╜╞╟╡╢╣╥╨╪╫╬"
    "︵︶︷︸︹︺︻︼︽︾︿﹀﹁﹂﹃﹄﹉﹊﹋﹌﹍﹎﹏"
    "＿￣＾｀´ˋˊ˙˚˘¯‾⁀⁔"
    "・･゜ﾟ°"
    "●○◎◕◔◉◠◡⊙⊕⊗⊘⊚⊛⊝⊞⊟⊠⊡"
    "☼☀☁☂☃☄☇☈☉☊☋☌☍☎☏"
    "⚗⚘⚙⚚⚛⚜"
    "๑٩۶୧୨๏₀₁₂₃₄₅₆₇₈₉⁰¹²³⁴⁵⁶⁷⁸⁹￩￫"
    "ㅂㅅㅈㅊㅋㅌㅍㅎㅠㅜㅡㅇ"
    "ღლ"
    "乛"
    "〆"
)

#: 强信号所在的 Unicode 区段（数学、制表、几何、补充箭头等）。
STRONG_RANGES: tuple[tuple[int, int], ...] = (
    (0x2200, 0x23FF),  # 数学运算符与技术符号
    (0x2500, 0x257F),  # 制表符
    (0x25A0, 0x25FF),  # 几何图形
    (0x2900, 0x297F),  # 补充箭头 B
    (0x2B00, 0x2BFF),  # 杂项符号与箭头
)

#: 弱信号字符。单个弱信号不足以判定颜文字，但两个以上可以。
SECONDARY_LITERALS = (
    "=<>~|/\\,.:;'\""
    "‵′″‴‶‷"
    "·‧∙⋅"
    "ー〜～"
    "ゝゞヽヾノﾉ彡ミ"
)

#: 弱信号所在的 Unicode 区段（箭头、杂项符号、装饰符号、补充标点、谚文音节）。
SECONDARY_RANGES: tuple[tuple[int, int], ...] = (
    (0x2190, 0x21FF),  # 箭头
    (0x2600, 0x26FF),  # 杂项符号（★ ♥ ♪ ☼ ⚗）
    (0x2700, 0x27BF),  # 装饰符号（✧ ✿ ❀ ✪）
    (0x2E00, 0x2E7F),  # 补充标点（‸ 等）
    (0xAC00, 0xD7AF),  # 谚文音节（눈）
)

#: 紧贴括号型颜文字出现的手臂、轮廓、装饰符号。删除颜文字后一并清理。
WRAP_LITERALS = (
    "╮╭╯╰╬╪╫╤╧╩╦╠╣║╔╗╚╝╒╓╕╖╙╛╜╞╟╡╢╣╥╨╪╫╬"
    "︵︶︷︸︹︺︻︼︽︾︿﹀﹁﹂﹃﹄﹏"
    "ヽヾゝゞノﾉミ彡っッつ"
    "ʕʔლʅʃงฅᕙᕗ٩۶୧୨ო"
    "oObBdDpPqQvVwWxXzZ"
    "☆★♥♡✧✿❀♪♫✪˖˚°·｡ﾟ゜"
    "و"
    "凸_/~\\～〜︴Σ∑εھзЗ┘└┌┐ｼﾍﾞ゛゜꒰꒱づԅㄟㄏႺჂﻌﾞ▁▂▃▄▅▆▇█▉▊▋▌▍▎▏▐░▒▓"
)

#: 裸颜文字中允许出现的 ASCII 字符（眼睛、手臂、下划线、标点等）。
KAOMOJI_ASCII_LITERALS = "_^=<>~|/\\.,:;'\"*#-+$%&@!?oOxXvVmMnNuUtTzZeEyYwWqQpPbBdD"

#: 括号家族：同一家族内的左右括号允许混用，例如 “（” 配 “)”。
BRACKET_FAMILIES: tuple[tuple[str, str], ...] = (
    ("(（", ")）"),
    ("[［【〔〖〘〚", "]］】〕〗〙〛"),
    ("{｛﹛", "}｝﹜"),
    ("<〈《", ">〉》"),
    ("「『", "」』"),
    ("｢", "｣"),
    ("﹙", "﹚"),
    ("﹝", "﹞"),
    ("꒰", "꒱"),
)

#: 删除颜文字后用于标记空位的私有区字符，避免递归匹配时发生跨段拼接。
MARKER = "\ue000"

#: 常见 ASCII 颜文字。按长度倒序匹配，避免 orz 被 or2 截断之类的歧义。
ASCII_KAOMOJI_LITERALS = (
    "orz", "or2", "orƶ", "OTL", "OTZ", "STO", "TAT", "QAQ", "QWQ", "QwQ",
    "OwO", "UwU", "OvO", "owo", "uwu", "T_T", "T.T", "T^T", "Q_Q", "Q.Q",
    "*_*", "*^*", "x_x", "X_X", "e_e", "n_n", "v_v", ";_;", "o_O", "O_o",
    "o.O", "O.o", ">_<", ">.<", ">o<", "O_O", "o_o", "O_o", "0_0", "@_@",
    "=_=", "-_-", "-__-", "._.", "._.", "^_^", "^-^", "^o^", "U_U", "u_u",
    "= =", "~_~", ":D", ":P", ":p", ":)", ":(", ";)", "ㅠㅠ", "ㅜㅜ", "ㅋㅋ",
)

#: 可视为颜文字整体包裹的括号字符集合（裸颜文字匹配时直接排除）。
BRACKET_CHARS = frozenset(
    "".join(open_chars + close_chars for open_chars, close_chars in BRACKET_FAMILIES)
)

#: 括号开启符到同家族闭合符的映射。
BRACKET_OPEN_MAP = {
    char: close_chars
    for open_chars, close_chars in BRACKET_FAMILIES
    for char in open_chars
}

#: 括号闭合符到同家族开启符的映射。
BRACKET_CLOSE_MAP = {
    char: open_chars
    for open_chars, close_chars in BRACKET_FAMILIES
    for char in close_chars
}


def build_regex_class(
    ranges: tuple[tuple[int, int], ...],
    literals: str,
) -> str:
    """根据 Unicode 区段与字面字符构造正则字符类主体。"""
    parts = [f"{chr(start)}-{chr(end)}" for start, end in ranges]
    parts.extend(re.escape(char) for char in literals)
    return "".join(parts)


KAOMOJI_CLASS = build_regex_class(KAOMOJI_RANGES, FACE_HAN_CHARS)
BARE_CLASS = build_regex_class(KAOMOJI_RANGES, BARE_FACE_HAN_CHARS + KAOMOJI_ASCII_LITERALS)
STRONG_CLASS = build_regex_class(STRONG_RANGES, STRONG_LITERALS)
SECONDARY_CLASS = build_regex_class(SECONDARY_RANGES, SECONDARY_LITERALS)
WRAP_CLASS = build_regex_class((), WRAP_LITERALS)

HAN_RE = re.compile(r"[\u3400-\u4dbf\u4e00-\u9fff\uf900-\ufaff]")
STRONG_RE = re.compile(rf"[{STRONG_CLASS}]")
SECONDARY_RE = re.compile(rf"[{SECONDARY_CLASS}]")
BARE_RE = re.compile(rf"[{BARE_CLASS}]{{2,24}}")
BARE_SPACED_RE = re.compile(
    rf"[{BARE_CLASS}]{{1,12}}(?:[ \t\u3000]+[{BARE_CLASS}]{{1,12}}){{1,3}}"
)
BARE_BOUNDARY_CHARS = "，。！？；：、,.!?;:…—“”‘’「」『』\"'"
SPACED_FACE_HINT_RE = re.compile(
    rf"[•●○◎◕◔◉◠◡⊙{re.escape(BARE_FACE_HAN_CHARS)}ㅂㅅㅈㅊㅋㅌㅍㅎㅠㅜㅡㅇ]"
)
SLEEPING_FACE_RE = re.compile(r"\([:¦]3\s*\[[▓▒░]+\]")
BRACKET_HINT_RE = re.compile(rf"[{re.escape(''.join(BRACKET_OPEN_MAP))}]")
ASCII_RE = re.compile(
    r"(?<![A-Za-z0-9])(?:"
    + "|".join(
        re.escape(item) for item in sorted(ASCII_KAOMOJI_LITERALS, key=len, reverse=True)
    )
    + r")(?![A-Za-z0-9])"
)
WRAPPER_BEFORE_RE = re.compile(
    rf"(?<![A-Za-z0-9])[{WRAP_CLASS}]{{1,4}}[ \t\u3000]*(?={re.escape(MARKER)})"
)
WRAPPER_AFTER_RE = re.compile(
    rf"(?<={re.escape(MARKER)})[ \t\u3000]*[{WRAP_CLASS}]{{1,4}}(?![A-Za-z0-9])"
)
EMPTY_BRACKET_RE = re.compile(
    r"(?:\([\s\ue000]*\)|（[\s\ue000]*）|\[[\s\ue000]*\]|［[\s\ue000]*］|"
    r"【[\s\ue000]*】|〔[\s\ue000]*〕|\{[\s\ue000]*\}|｛[\s\ue000]*｝|"
    r"<[\s\ue000]*>|〈[\s\ue000]*〉|《[\s\ue000]*》|「[\s\ue000]*」|"
    r"『[\s\ue000]*』|〖[\s\ue000]*〗|〘[\s\ue000]*〙|〚[\s\ue000]*〛|"
    r"｢[\s\ue000]*｣|﹙[\s\ue000]*﹚|﹛[\s\ue000]*﹜|﹝[\s\ue000]*﹞)+"
)
SPACE_COLLAPSE_RE = re.compile(r"(?<=\S)[ \t\u3000]{2,}")
CJK_SPACE_RE = re.compile(
    r"(?<=[\u3400-\u4dbf\u4e00-\u9fff，。！？；：、])[ \t\u3000]+"
    r"(?=[\u3400-\u4dbf\u4e00-\u9fff，。！？；：、])"
)
SPACE_BEFORE_PUNCT_RE = re.compile(r"[ \t\u3000]+(?=[，。！？；：、])")
QUOTE_PUNCT_RE = re.compile(r"([“「『])\s*[，。；：、]")

FACE_HAN_SET = frozenset(FACE_HAN_CHARS)
BARE_FACE_HAN_SET = frozenset(BARE_FACE_HAN_CHARS)
#: 只允许在括号型颜文字内部充当“嘴/眼/轮廓”的汉字符号。
FACE_HAN_SIGNAL_CHARS = "灬皿罒艹益曲凸凹"
FACE_HAN_SIGNAL_SET = frozenset(FACE_HAN_SIGNAL_CHARS)
FACE_HAN_SIGNAL_RE = re.compile(rf"[{re.escape(FACE_HAN_SIGNAL_CHARS)}]")
ASCII_KAOMOJI_SET = frozenset(ASCII_KAOMOJI_LITERALS)
WRAPPER_HAN_CHARS = frozenset("人山凸凹才")


def is_han(char: str) -> bool:
    """判断字符是否属于常用汉字区。"""
    return bool(HAN_RE.fullmatch(char))


def signal_score(text: str) -> int:
    """计算颜文字信号分：强信号 2 分，弱信号 1 分。"""
    return sum(
        2 if STRONG_RE.fullmatch(char) else 1 if SECONDARY_RE.fullmatch(char) else 0
        for char in text
    )


def strong_count(text: str) -> int:
    """统计强信号字符数量。"""
    return sum(1 for char in text if STRONG_RE.fullmatch(char))


def face_strong_count(text: str) -> int:
    """统计括号型颜文字内部的强信号数量（含专用汉字部件）。"""
    return strong_count(text) + sum(1 for char in text if char in FACE_HAN_SIGNAL_SET)


def face_signal_score(text: str) -> int:
    """计算括号型颜文字内部的信号分（汉字部件按强信号计分）。"""
    return signal_score(text) + 2 * sum(1 for char in text if char in FACE_HAN_SIGNAL_SET)


def has_face_punctuation(text: str) -> bool:
    """判断片段中是否至少包含一个非字母字符。

    纯字母片段（如英文单词、俄文单词、日文假名句子）即使包含颜文字常用的
    希腊/西里尔字母，也不应被当成颜文字删除。
    """
    return any(
        not char.isspace() and unicodedata.category(char)[0] != "L" for char in text
    )


def has_ascii_word(text: str) -> bool:
    """判断片段中是否包含两个及以上连续的 ASCII 字母。"""
    return bool(re.search(r"[A-Za-z]{2,}", text))


def is_face_inner(inner: str) -> bool:
    """校验括号内片段是否构成一张颜文字脸。

    约束：
    - 不含普通汉字（仅允许 FACE_HAN_CHARS 中的部首/方框类部件）；
    - 不含 ASCII 单词；ASCII 字母只能是颜文字常用的 o/O/x/X/T 等；
    - 不同时包含 ASCII 字母与 ASCII 数字，避免误伤 “（x^2）” 这类公式；
    - 至少包含一个强信号字符；纯部件/纯字母脸还要求片段足够短。
    """
    core = inner.strip(BLANK_CHARS)
    if not core or len(core) > 30:
        return False

    if any(is_han(char) and char not in FACE_HAN_SET for char in core):
        return False

    ascii_letters = [char for char in core if char.isascii() and char.isalpha()]
    ascii_digits = [char for char in core if char.isascii() and char.isdigit()]
    if ascii_letters and ascii_digits:
        return False
    if any(char not in KAOMOJI_ASCII_LITERALS for char in ascii_letters):
        return False
    if has_ascii_word(core) and core not in ASCII_KAOMOJI_SET:
        return False

    if face_strong_count(core) == 0:
        return False
    # 纯部件/纯字母脸（如 艹皿艹、ﾟｰﾟ）没有非字母字符；
    # 仅在片段很短且强信号足够多时才认定，避免误删希腊/俄文单词。
    if not has_face_punctuation(core) and (
        len(core) > 4 or face_signal_score(core) < 4
    ):
        return False
    return face_signal_score(core) >= 2


def is_bare_face(core: str) -> bool:
    """校验不带括号的连续片段是否构成颜文字。

    裸颜文字不包含括号；汉字只允许极少数轮廓部件（如 乛），避免把
    “一曲”“凹凸有致”“受益”等正常词语误判为颜文字。
    """
    if not 2 <= len(core) <= 24:
        return False
    if any(char in BRACKET_CHARS and char not in "<>" for char in core):
        return False
    if any(is_han(char) and char not in BARE_FACE_HAN_SET for char in core):
        return False
    if has_ascii_word(core):
        return False
    if any(char.isascii() and char.isdigit() for char in core):
        return False
    if strong_count(core) == 0:
        return False
    if not has_face_punctuation(core) and (len(core) > 4 or signal_score(core) < 4):
        return False
    return signal_score(core) >= 2


def bracket_balance_ok(inner: str, open_chars: str, close_chars: str) -> bool:
    """拒绝对外跨越多个颜文字的括号片段。

    例如 “(脸)。(另一张脸)” 会先配对到末尾的右括号，此时内容中会出现
    “右括号先于左括号”的情况，应当放弃该长匹配，退回较短的第一张脸。
    """
    depth = 0
    for char in inner:
        if char in open_chars:
            depth += 1
        elif char in close_chars:
            if depth == 0:
                return False
            depth -= 1
    return True


def remove_bracket_faces(text: str) -> tuple[str, bool]:
    """删除括号型颜文字，返回 (处理后文本, 是否发生删除)。"""
    if not BRACKET_HINT_RE.search(text):
        return text, False

    pieces: list[str] = []
    changed = False
    index = 0
    length = len(text)

    while index < length:
        char = text[index]
        close_chars = BRACKET_OPEN_MAP.get(char)
        if close_chars:
            limit = min(length, index + 33)
            candidates: list[int] = []
            cursor = index + 1
            while cursor < limit and text[cursor] != "\n":
                if text[cursor] in close_chars:
                    candidates.append(cursor)
                cursor += 1

            open_chars = BRACKET_CLOSE_MAP.get(close_chars[0], "")
            for close_index in reversed(candidates):
                inner = text[index + 1 : close_index]
                if not bracket_balance_ok(inner, open_chars, close_chars):
                    continue
                if is_face_inner(inner):
                    pieces.append(MARKER)
                    index = close_index + 1
                    changed = True
                    break
            else:
                pieces.append(char)
                index += 1
            continue

        pieces.append(char)
        index += 1

    return "".join(pieces), changed


def strip_adjacent_wrappers(text: str) -> str:
    """删除紧贴已识别颜文字的装饰符号（╮、ﾉ、☆、o 等）。"""
    previous = None
    while previous != text:
        previous = text
        text = WRAPPER_BEFORE_RE.sub("", text)
        text = WRAPPER_AFTER_RE.sub("", text)
    return text


def strip_kaomoji(text: str) -> str:
    """删除文本中的颜文字片段，保留正常正文与标点。"""
    if not text:
        return text
    if (
        not STRONG_RE.search(text)
        and not ASCII_RE.search(text)
        and not SLEEPING_FACE_RE.search(text)
        and not (BRACKET_HINT_RE.search(text) and FACE_HAN_SIGNAL_RE.search(text))
    ):
        return text

    changed = False

    text, removed = remove_bracket_faces(text)
    changed = changed or removed
    if changed:
        text = strip_adjacent_wrappers(text)

    before = text
    text = SLEEPING_FACE_RE.sub(MARKER, text)
    changed = changed or text != before

    def build_bare_replacement(match: re.Match[str], spaced: bool) -> str:
        raw = match.group(0)
        left = 0
        while left < len(raw) and raw[left] in BARE_BOUNDARY_CHARS:
            left += 1
        right = len(raw)
        while right > left and raw[right - 1] in BARE_BOUNDARY_CHARS:
            right -= 1
        core_raw = raw[left:right]
        compact = re.sub(r"[ \t\u3000]+", "", core_raw)
        if spaced and not SPACED_FACE_HINT_RE.search(compact):
            return raw
        if len(compact) > 24 or not is_bare_face(compact):
            return raw
        string = match.string
        start = match.start() + left
        end = match.start() + right
        prev_char = string[start - 1] if start > 0 else ""
        next_char = string[end] if end < len(string) else ""
        if prev_char and prev_char.isascii() and prev_char.isalnum():
            return raw
        if next_char and next_char.isascii() and next_char.isalnum():
            return raw
        return raw[:left] + MARKER + raw[right:]

    def replace_spaced(match: re.Match[str]) -> str:
        return build_bare_replacement(match, spaced=True)

    def replace_bare(match: re.Match[str]) -> str:
        return build_bare_replacement(match, spaced=False)

    before = text
    text = BARE_SPACED_RE.sub(replace_spaced, text)
    changed = changed or text != before

    before = text
    text = BARE_RE.sub(replace_bare, text)
    changed = changed or text != before
    if changed:
        text = strip_adjacent_wrappers(text)

    before = text
    text = ASCII_RE.sub(MARKER, text)
    changed = changed or text != before

    if not changed:
        return text

    text = strip_adjacent_wrappers(text)
    previous = None
    while previous != text:
        previous = text
        text = EMPTY_BRACKET_RE.sub("", text)
    text = text.replace(MARKER, "")
    text = SPACE_COLLAPSE_RE.sub(" ", text)
    text = CJK_SPACE_RE.sub("", text)
    text = SPACE_BEFORE_PUNCT_RE.sub("", text)
    text = QUOTE_PUNCT_RE.sub(r"\1", text)
    return text


def is_residual_symbol_line(text: str) -> bool:
    """判断已删除颜文字后的剩余文本是否仍属于无实质内容的占位行。"""
    s = text.strip(BLANK_CHARS)
    if not s:
        return True

    if STANDALONE_ELLIPSIS.match(s) or STANDALONE_SEPARATOR.match(s):
        return True

    if all(unicodedata.category(char)[0] in ("P", "S", "Z", "C") for char in s):
        return True

    return bool(
        not HAN_RE.search(s)
        and not re.search(r"[A-Za-z]{2,}", s)
        and not re.search(r"[0-9]", s)
    )


def is_standalone_symbol_line(text: str) -> bool:
    """判断一行文本是否为单独成行的纯标点符号、特殊符号或颜文字占位行。

    识别范围：
    1. 各种标点符号组合（如“……？”、“？？？”、“？！”、“！！！”、““……””等）；
    2. 纯标点符号、特殊符号构成的行（如 •﹏•、→_→、(✪﹃✪) 等）；
    3. 分隔线行（如 ----------、—————— 等）；
    4. 删除颜文字后不剩任何正文的占位行（如 ლ(◉益◉ლ)、๑乛◡乛๑ 等）。
    """
    return is_residual_symbol_line(strip_kaomoji(text))


def clean_line_text(line: str) -> str:
    """清洗单行小说文本：删除颜文字、过滤占位行，返回可入库的正文行。

    返回空字符串表示该行应被丢弃。
    """
    if not line:
        return ""
    original = line.strip(BLANK_CHARS)
    if not original:
        return ""

    cleaned = strip_kaomoji(line).strip(BLANK_CHARS)
    if not cleaned:
        return ""

    if (
        cleaned != original
        and len(cleaned) <= 2
        and all(char in WRAPPER_HAN_CHARS for char in cleaned)
    ):
        return ""

    if cleaned != original:
        has_han = bool(HAN_RE.search(cleaned))
        has_word = bool(re.search(r"[A-Za-z]{2,}", cleaned))
        has_long_number = bool(re.search(r"\d{2,}", cleaned))
        if not has_han and not has_word and not has_long_number:
            return ""

    if is_residual_symbol_line(cleaned):
        return ""
    return cleaned


def parse_novel_filename(filename: str) -> tuple[str, str, str]:
    """从文件名解析书名、作者与类型。

    支持格式：
    - 书名.txt -> (书名, "", "")
    - 书名——作者.txt -> (书名, 作者, "")
    - 书名——作者——类型.txt -> (书名, 作者, 类型)
    """
    if not filename:
        return "", "", ""

    stem = Path(filename).stem
    parts = [p.strip() for p in stem.split(FIELD_SEPARATOR)]

    if len(parts) == 1:
        return parts[0], "", ""
    if len(parts) == 2:
        return parts[0], parts[1], ""

    title = parts[0]
    author = parts[1]
    genre = FIELD_SEPARATOR.join(p for p in parts[2:] if p)
    return title, author, genre


def clean_text(text: str) -> str:
    """机械清洗小说文本。

    清洗规则：
    - 过滤所有空行（包括仅含半角空格、Tab、全角空格或不可见空白字符的行）；
    - 过滤单独占一行的纯标点符号与特殊符号行（如“……？”、“？？？”、“•﹏•”、““……””等）；
    - 过滤单独占一行的分隔线行（包括“----------”、“——————”、“======”与带引号形态等）；
    - 过滤删除颜文字后不剩正文的占位行；
    - 删除与汉字同行的颜文字片段（如“他笑了(￣▽￣)。”→“他笑了。”）。
    """
    cleaned_lines: list[str] = []
    for line in text.splitlines():
        cleaned = clean_line_text(line)
        if cleaned:
            cleaned_lines.append(cleaned)
    return "\n".join(cleaned_lines)


def count_words(text: str) -> int:
    """统计文本字数（非空白字符数，含标点）。"""
    return sum(1 for ch in text if not ch.isspace())


def parse_chapters(text: str) -> list[tuple[str, str]]:
    """按章节标题正则切分小说正文。

    Returns:
        [(章节名称, 章节内容), ...]
        若未识别到章节标题，则返回 [("全文", text.strip())]。
    """
    if not text or not text.strip():
        return []

    matches = list(CHAPTER_HEADER.finditer(text))
    if not matches:
        return [("全文", text.strip())]

    chapters: list[tuple[str, str]] = []
    if matches[0].start() > 0:
        preamble = text[: matches[0].start()].strip()
        if preamble:
            chapters.append(("前言", preamble))

    for i, m in enumerate(matches):
        title = m.group(0).strip()
        content_start = m.end()
        content_end = matches[i + 1].start() if i + 1 < len(matches) else len(text)
        content = text[content_start:content_end].strip()
        chapters.append((title, content))

    return chapters
