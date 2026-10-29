"use client";

import {
  createContext,
  useContext,
  useEffect,
  useState,
  type ReactNode,
} from "react";
import { ChapterImportModal } from "@/components/chapter-import-modal";
import { useToast } from "@/components/toast";
import {
  chapterAfterRender,
  chapterAfterSplit,
  type HubChapter,
  type PipelineFilter,
} from "@/lib/projects";

type HubChaptersValue = {
  chapters: HubChapter[];
  /** 状态筛选条与流水线共用，导入新章时要复位到「全部」 */
  filter: PipelineFilter;
  setFilter: (filter: PipelineFilter) => void;
  /** 拆解中的章节 id，行内按钮据此显示进度态 */
  splittingIds: string[];
  split: (chapter: HubChapter) => void;
  openImport: () => void;
};

const HubChaptersContext = createContext<HubChaptersValue | null>(null);

export function useHubChapters() {
  const value = useContext(HubChaptersContext);
  if (!value) {
    throw new Error("useHubChapters 必须在 HubChaptersProvider 内使用");
  }
  return value;
}

export function HubChaptersProvider({
  projectId,
  initialChapters,
  studioReady = true,
  children,
}: {
  projectId: string;
  initialChapters: HubChapter[];
  /** 章节是否已有制作工作台数据；后端剧目暂无，拆解完也不给工作台入口 */
  studioReady?: boolean;
  children: ReactNode;
}) {
  const { show } = useToast();
  const [chapters, setChapters] = useState(initialChapters);
  const [splittingIds, setSplittingIds] = useState<string[]>([]);
  const [filter, setFilter] = useState<PipelineFilter>("全部");
  const [importOpen, setImportOpen] = useState(false);

  // 合成中的章节持续推进进度，满 100% 的那一行当场落到母带就绪
  useEffect(() => {
    if (!chapters.some((chapter) => chapter.state === "合成中")) return;
    const timer = setInterval(() => {
      const finished = chapters.filter(
        (chapter) => chapter.state === "合成中" && (chapter.percent ?? 0) >= 99,
      );
      setChapters((prev) =>
        prev.map((chapter) => {
          if (chapter.state !== "合成中" || chapter.percent === undefined) return chapter;
          const percent = Math.min(100, chapter.percent + 1);
          return percent >= 100
            ? chapterAfterRender({ ...chapter, percent })
            : { ...chapter, percent };
        }),
      );
      for (const chapter of finished) {
        show(`${chapter.title} 已生成，可进入制作工作台试听`, "success");
      }
    }, 1800);
    return () => clearInterval(timer);
  }, [chapters, show]);

  function split(chapter: HubChapter) {
    setSplittingIds((prev) => [...prev, chapter.id]);
    show(`正在用 Qwen2.5 拆解《${chapter.title}》的对白与情绪…`, "info");
    setTimeout(() => {
      setChapters((prev) =>
        prev.map((item) => {
          if (item.id !== chapter.id) return item;
          const ready = chapterAfterSplit(item);
          // 后端剧目还没有工作台数据，拆解完也不给进入制作工作台的入口
          return studioReady ? ready : { ...ready, openable: false };
        }),
      );
      setSplittingIds((prev) => prev.filter((id) => id !== chapter.id));
      show("拆解完成：识别出 2 位说话人、86 段对白", "success");
    }, 1600);
  }

  return (
    <HubChaptersContext.Provider
      value={{
        chapters,
        filter,
        setFilter,
        splittingIds,
        split,
        openImport: () => setImportOpen(true),
      }}
    >
      {children}
      {importOpen && (
        <ChapterImportModal
          projectId={projectId}
          chapterCount={chapters.length}
          onClose={() => setImportOpen(false)}
          onImported={(result) => {
            setFilter("全部");
            show(
              `已追加 ${result.addedChapters} 个新章节，跳过同名 ${result.skippedChapters} 章`,
              "success",
            );
          }}
        />
      )}
    </HubChaptersContext.Provider>
  );
}
