use crate::ast::node::SegmentKind;

/// 文本清洗扩展点（注册多个时按注册层配置的顺序串成一条清洗链）
pub trait TextCleaner: Send + Sync {
    fn clean_line(&self, text: &str) -> String;
}

/// 章节拆分结果
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawChapter {
    pub title: String,
    pub content: String,
}

/// 台词拆分片段结果
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawDialogueSegment {
    pub text: String,
    pub kind: SegmentKind,
}

/// 章节分章扩展点
pub trait ChapterSplitter: Send + Sync {
    fn split_chapters(&self, full_text: &str) -> Vec<RawChapter>;
    fn resolve_fingerprint(&self, title: &str) -> String;

    /// 从文件名解析（书名, 作者, 类型）。默认只去掉扩展名；分章插件可以按自己的规则覆盖。
    fn parse_filename(&self, filename: &str) -> (String, String, String) {
        let stem = filename.rsplit_once('.').map(|(s, _)| s).unwrap_or(filename);
        (stem.trim().to_string(), String::new(), String::new())
    }
}

/// 单句内台词/旁白切分扩展点
pub trait DialogueSplitter: Send + Sync {
    fn split_sentence(&self, sentence: &str) -> Vec<RawDialogueSegment>;
}
