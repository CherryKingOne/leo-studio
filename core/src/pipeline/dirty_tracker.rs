use crate::ast::{ChapterNode, SegmentNode};

#[derive(Debug, Clone, Default)]
pub struct ChapterDirtySummary {
    pub total_segments: usize,
    pub dirty_segments: usize,
    pub clean_segments: usize,
    pub is_fully_clean: bool,
}

pub struct DirtyTracker;

impl DirtyTracker {
    /// 统计指定章节的脏节点分布
    pub fn inspect_chapter(chapter: &ChapterNode) -> ChapterDirtySummary {
        let mut total = 0;
        let mut dirty = 0;

        for p in &chapter.paragraphs {
            for s in &p.sentences {
                for seg in &s.segments {
                    total += 1;
                    if seg.is_dirty || seg.audio.is_none() {
                        dirty += 1;
                    }
                }
            }
        }

        ChapterDirtySummary {
            total_segments: total,
            dirty_segments: dirty,
            clean_segments: total.saturating_sub(dirty),
            is_fully_clean: total > 0 && dirty == 0,
        }
    }

    /// 提取所有需要跑 TTS 渲染的脏片段引用
    pub fn collect_dirty_segments<'a>(chapter: &'a mut ChapterNode) -> Vec<&'a mut SegmentNode> {
        chapter
            .paragraphs
            .iter_mut()
            .flat_map(|p| &mut p.sentences)
            .flat_map(|s| &mut s.segments)
            .filter(|seg| seg.is_dirty || seg.audio.is_none())
            .collect()
    }
}
