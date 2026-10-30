"use client";

import { useEffect, useMemo, useRef, useState } from "react";
import { useRouter } from "next/navigation";
import { ParagraphOutline, type StudioPane } from "@/components/paragraph-outline";
import { ScriptFeed, type FeedItem } from "@/components/script-feed";
import { StudioHeader } from "@/components/studio-header";
import { TuningDock } from "@/components/tuning-dock";
import { useToast } from "@/components/toast";
import { playAudition } from "@/lib/audio";
import type {
  CastOption,
  ChapterStudio as StudioData,
  HubChapter,
  StudioParagraph,
  StudioProject,
  StudioTocEntry,
} from "@/lib/projects";

/** 滚动意图：目标 DOM 元素 + 对齐方式，nonce 让重复点同一段也能再滚一次 */
type ScrollIntent = { target: string; block: "center" | "nearest"; nonce: number };

/**
 * 上次看的是哪一栏记在模块里：从目录切章会让工作台子树重挂载，
 * 客户端导航不换 JS runtime，所以这份记忆能跨章节保留，整页刷新才回到大纲。
 */
let rememberedPane: StudioPane = "outline";

export function ChapterStudio({
  project,
  chapter,
  studio,
  toc,
}: {
  project: StudioProject;
  chapter: HubChapter;
  studio: StudioData;
  /** 全书章节目录，来自 GET /projects/{id}/chapters */
  toc: StudioTocEntry[];
}) {
  const { show } = useToast();
  const router = useRouter();
  const scrollNonce = useRef(0);

  const [paragraphs, setParagraphs] = useState(studio.paragraphs);
  const [order, setOrder] = useState(() => studio.paragraphs.map((item) => item.id));
  const [undoStack, setUndoStack] = useState<string[][]>([]);
  const [redoStack, setRedoStack] = useState<string[][]>([]);
  const [activeId, setActiveId] = useState(studio.paragraphs[0].id);
  const [pane, setPane] = useState<StudioPane>(() => rememberedPane);
  const [keyword, setKeyword] = useState("");
  const [tuningOpen, setTuningOpen] = useState(false);
  const [scrollIntent, setScrollIntent] = useState<ScrollIntent | null>(null);

  function selectPane(next: StudioPane) {
    rememberedPane = next;
    setPane(next);
  }

  const baseOrder = useMemo(() => studio.paragraphs.map((item) => item.id), [studio]);
  const byId = useMemo(
    () => new Map(paragraphs.map((item) => [item.id, item])),
    [paragraphs],
  );

  // 重排后卡片序号与时间轴都要按新顺序重新累加
  const timeline = useMemo(() => {
    const map = new Map<string, { seq: number; start: number }>();
    let cursor = 0;
    for (const [index, id] of order.entries()) {
      const paragraph = byId.get(id);
      if (!paragraph) continue;
      map.set(id, { seq: index + 1, start: cursor });
      cursor += paragraph.seconds;
    }
    return map;
  }, [order, byId]);

  const ordered = order
    .map((id) => byId.get(id))
    .filter((item): item is StudioParagraph => item !== undefined);

  const needle = keyword.trim().toLowerCase();
  const items: FeedItem[] = ordered
    .filter((paragraph) => {
      if (!needle) return true;
      return `${paragraph.speaker} ${paragraph.displayName} ${paragraph.text}`
        .toLowerCase()
        .includes(needle);
    })
    .map((paragraph) => {
      const slot = timeline.get(paragraph.id)!;
      return {
        paragraph,
        seq: slot.seq,
        startSeconds: slot.start,
        first: slot.seq === 1,
        last: slot.seq === order.length,
      };
    });

  const active = byId.get(activeId) ?? paragraphs[0];
  const activeSeq = timeline.get(active.id)?.seq ?? 1;
  const cueCount = paragraphs.filter((item) => item.cue).length;
  const isDirty = order.join() !== baseOrder.join();

  // 卡片与左栏条目各用自己的 id，切换 pane 后由 effect 统一滚动
  useEffect(() => {
    if (!scrollIntent) return;
    document
      .getElementById(scrollIntent.target)
      ?.scrollIntoView({ behavior: "smooth", block: scrollIntent.block });
  }, [scrollIntent]);

  /**
   * `column` 是这次要点亮的列：剧本流卡片或左栏。
   * 左栏跟随用户当前 pane，点卡片不再把大纲换成正文；目录没有逐段锚点，只更新选中态。
   */
  function jump(id: string, column: "feed" | "pane") {
    setActiveId(id);

    const anchor =
      column === "feed"
        ? { target: id, block: "center" as const }
        : pane === "outline"
          ? { target: `outline-${id}`, block: "center" as const }
          : pane === "source"
            ? { target: `source-${id}`, block: "nearest" as const }
            : null;
    if (!anchor) return;

    scrollNonce.current += 1;
    setScrollIntent({ ...anchor, nonce: scrollNonce.current });
  }

  function commit(next: string[]) {
    if (next.join() === order.join()) return;
    setUndoStack((prev) => [...prev, order]);
    setRedoStack([]);
    setOrder(next);
  }

  function movePara(id: string, delta: number) {
    const from = order.indexOf(id);
    const to = from + delta;
    if (to < 0 || to >= order.length) return;
    const next = order.filter((item) => item !== id);
    next.splice(to, 0, id);
    commit(next);
    show(`该段已${delta < 0 ? "前移" : "后移"}至第 ${to + 1} 位`, "success");
  }

  function reorder(dragId: string, targetId: string, before: boolean) {
    const next = order.filter((item) => item !== dragId);
    next.splice(next.indexOf(targetId) + (before ? 0 : 1), 0, dragId);
    commit(next);
  }

  function undo() {
    const prev = undoStack[undoStack.length - 1];
    if (!prev) return;
    setUndoStack((stack) => stack.slice(0, -1));
    setRedoStack((stack) => [...stack, order]);
    setOrder(prev);
    show("已撤销上一步顺序调整", "info");
  }

  function redo() {
    const next = redoStack[redoStack.length - 1];
    if (!next) return;
    setRedoStack((stack) => stack.slice(0, -1));
    setUndoStack((stack) => [...stack, order]);
    setOrder(next);
    show("已重做顺序调整", "info");
  }

  function resetOrder() {
    commit(baseOrder);
    show("已恢复段落原始顺序", "success");
  }

  function reassign(id: string, option: CastOption) {
    setParagraphs((prev) =>
      prev.map((paragraph) =>
        paragraph.id === id
          ? {
              ...paragraph,
              displayName: option.label,
              speaker: option.label.split("·")[0],
              tuning: { ...paragraph.tuning, speaker: option.option },
            }
          : paragraph,
      ),
    );
    show(`本段说话人已改绑为「${option.label}」，仅影响后续合成`, "success");
  }

  function audition(hz: number, seconds: number, label = "本段音频") {
    playAudition(hz);
    show(`正在试听${label} ${hz}Hz · ${seconds}s`, "info");
  }

  return (
    <div className="flex h-screen min-h-0 flex-col overflow-hidden">
      <StudioHeader
        project={project}
        chapter={chapter}
        keyword={keyword}
        onKeywordChange={setKeyword}
        paragraphCount={paragraphs.length}
        cueCount={cueCount}
      />

      <div className="relative flex min-h-0 flex-1">
        <ParagraphOutline
          items={ordered.map((paragraph) => ({
            paragraph,
            seq: timeline.get(paragraph.id)?.seq ?? 1,
          }))}
          source={studio.source}
          toc={toc}
          pane={pane}
          activeId={active.id}
          words={chapter.words}
          onPaneChange={selectPane}
          onJumpToFeed={(id) => jump(id, "feed")}
          onTocSelect={(entry) =>
            entry.current || router.push(`/project/${project.id}/chapter/${entry.id}`)
          }
        />

        <main className="flex min-w-0 flex-1 overflow-hidden">
          <div
            className={`min-w-0 flex-1 p-3 transition-[margin] duration-200 ease-out ${
              tuningOpen ? "lg:mr-[304px]" : ""
            }`}
          >
            <ScriptFeed
              items={items}
              activeId={active.id}
              keyword={keyword}
              orderLabel={isDirty ? `自定义顺序 · 可撤销 ${undoStack.length} 步` : "原始顺序"}
              canUndo={undoStack.length > 0}
              canRedo={redoStack.length > 0}
              canReset={isDirty}
              onUndo={undo}
              onRedo={redo}
              onReset={resetOrder}
              onMove={movePara}
              onReorder={reorder}
              onActivate={(id) => jump(id, "pane")}
              onReassign={reassign}
              onAudition={audition}
            />
          </div>
        </main>

        <TuningDock
          open={tuningOpen}
          paragraph={active}
          seq={activeSeq}
          speakerOptions={studio.speakerOptions}
          modelOptions={studio.modelOptions}
          reverbOptions={studio.reverbOptions}
          onOpenChange={setTuningOpen}
          onAudition={audition}
        />
      </div>
    </div>
  );
}
