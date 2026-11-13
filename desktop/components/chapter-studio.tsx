"use client";

import { useEffect, useMemo, useRef, useState } from "react";
import { useRouter } from "next/navigation";
import { anchorOf, type AnchorRect } from "@/components/floating-card";
import { ParagraphOutline, type StudioPane } from "@/components/paragraph-outline";
import { PolyphonePopover } from "@/components/polyphone-popover";
import { ScriptFeed, type FeedItem } from "@/components/script-feed";
import { StudioHeader } from "@/components/studio-header";
import { TuningDock } from "@/components/tuning-dock";
import { useToast } from "@/components/toast";
import { clearSentencePolyphonesAction, saveSentencePolyphonesAction } from "@/lib/actions";
import { playAudition } from "@/lib/audio";
import { isHanChar } from "@/lib/polyphone";
import {
  isSentenceId,
  type CastOption,
  type ChapterStudio as StudioData,
  type HubChapter,
  type PolyphoneMark,
  type StudioParagraph,
  type StudioProject,
  type StudioTocEntry,
  type TextSelection,
} from "@/lib/projects";

/** 滚动意图：目标 DOM 元素 + 对齐方式，nonce 让重复点同一段也能再滚一次 */
type ScrollIntent = { target: string; block: "center" | "nearest"; nonce: number };

/** dock 多音字按钮的操作目标：正文里被划选的那单个汉字 */
type PolyTarget = { paragraphId: string; index: number; char: string; reading?: string };

/**
 * 上次看的是哪一栏记在模块里：从目录切章会让工作台子树重挂载，
 * 客户端导航不换 JS runtime，所以这份记忆能跨章节保留，整页刷新才回到大纲。
 */
let rememberedPane: StudioPane = "outline";

/** 原型数据里预置的 polyphone 短语首字，转成与划选同一套标注呈现 */
function seedPolyphones(paragraphs: StudioParagraph[]) {
  const seeded: Record<string, PolyphoneMark[]> = {};
  for (const paragraph of paragraphs) {
    const { polyphone } = paragraph;
    const at = polyphone ? paragraph.text.indexOf(polyphone.phrase) : -1;
    if (polyphone && at >= 0) {
      seeded[paragraph.id] = [{ index: at, char: polyphone.phrase[0], reading: polyphone.reading }];
    }
  }
  return seeded;
}

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
  const [polyphones, setPolyphones] = useState(() => ({
    ...seedPolyphones(studio.paragraphs),
    ...studio.polyphones,
  }));
  /** 正文里当前那段划选：多音字标注以它为操作目标，弹窗也按它定位 */
  const [selection, setSelection] = useState<TextSelection | null>(null);
  const [polyAnchor, setPolyAnchor] = useState<AnchorRect | null>(null);

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
   * 点到正文之外就收掉划选底色，但 dock 与读音弹窗除外：
   * 那两处都拿当前选区当操作目标，一并清掉就没法给选中的字标读音了。
   */
  useEffect(() => {
    if (!selection) return;

    function onMouseUp(event: MouseEvent) {
      const target = event.target;
      if (!(target instanceof Element)) return;
      if (
        target.closest("[data-text]") ||
        target.closest("[data-dock]") ||
        target.closest(".fixed")
      )
        return;
      setSelection(null);
    }

    document.addEventListener("mouseup", onMouseUp);
    return () => document.removeEventListener("mouseup", onMouseUp);
  }, [selection]);

  /**
   * `column` 是这次要点亮的列：剧本流卡片或左栏。
   * 左栏跟随用户当前 pane，点卡片不再把大纲换成正文；目录没有逐段锚点，只更新选中态。
   */
  function jump(id: string, column: "feed" | "pane") {
    setActiveId(id);
    setPolyAnchor(null);

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

  /** 划选正好是一个汉字时，才有多音字可读 */
  const polyTarget: PolyTarget | null =
    selection && selection.end === selection.start + 1 && isHanChar(selection.text)
      ? {
          paragraphId: selection.paragraphId,
          index: selection.start,
          char: selection.text,
          reading: polyphones[selection.paragraphId]?.find(
            (item) => item.index === selection.start,
          )?.reading,
        }
      : null;

  /**
   * 划选或点标注：记下这段区间，正文给它铺底色，读音弹窗也以它为目标。
   * `target` 只有点正文里已标注那个字时才有，直接把读音弹窗开在它下面，不用再回 dock 按一次。
   */
  function selectText(paragraphId: string, start: number, end: number, target?: HTMLElement) {
    setSelection({ paragraphId, start, end, text: byId.get(paragraphId)?.text.slice(start, end) ?? "" });
    setPolyAnchor(target ? anchorOf(target) : null);
  }

  /** 选区底色元素的矩形；取不到说明选区已不在当前正文里（换段、筛选或改了文案） */
  function selectionAnchor() {
    if (!selection) return null;
    return anchorOf(document.getElementById(`sel-${selection.paragraphId}-${selection.start}`));
  }

  /** dock 的多音字按钮：只认单个汉字 */
  function openPolyphone() {
    const anchor = polyTarget ? selectionAnchor() : null;
    setPolyAnchor(anchor);
    if (!anchor) show("请滑选单个汉字", "warn");
  }

  /**
   * 标注先改本地再整句同步后端：PUT 是全量覆盖，所以一次发该句的全部标注，
   * 列表空了就改走 DELETE。失败退回改动前的那份，界面不会和落库数据两套。
   */
  async function syncPolyphones(
    sentenceId: string,
    next: PolyphoneMark[],
    previous: PolyphoneMark[],
    success: string,
  ) {
    // 原型剧目的段落 id 不是后端句子 id，没有可写的数据源
    if (!isSentenceId(sentenceId)) {
      show(success, "success");
      return;
    }

    const result = next.length
      ? await saveSentencePolyphonesAction({
          projectId: project.id,
          sentenceId,
          text: byId.get(sentenceId)?.text ?? "",
          marks: next,
        })
      : await clearSentencePolyphonesAction({ projectId: project.id, sentenceId });

    if (!result.ok) {
      // 只退回这一句的标注，别把同期改动的其它句子一起抹掉
      setPolyphones((prev) => ({ ...prev, [sentenceId]: previous }));
      show(`标注未保存：${result.message}`, "warn");
      return;
    }
    show(success, "success");
  }

  function applyReading(reading: string) {
    if (!polyTarget) return;
    const { paragraphId, index, char } = polyTarget;
    const previous = polyphones[paragraphId] ?? [];
    const next: PolyphoneMark[] = [
      ...previous.filter((item) => item.index !== index),
      { index, char, reading },
    ].sort((a, b) => a.index - b.index);

    setPolyphones((prev) => ({ ...prev, [paragraphId]: next }));
    setPolyAnchor(null);
    setSelection(null);
    void syncPolyphones(
      paragraphId,
      next,
      previous,
      `已把「${char}」标注为 ${reading}，本段重新合成时生效`,
    );
  }

  function removeReading(paragraphId: string, index: number, char: string) {
    const previous = polyphones[paragraphId] ?? [];
    const next = previous.filter((item) => item.index !== index);

    setPolyphones((prev) => ({ ...prev, [paragraphId]: next }));
    setPolyAnchor(null);
    setSelection(null);
    void syncPolyphones(paragraphId, next, previous, `已移除「${char}」的注音`);
  }

  function clearSentenceReadings(target: PolyTarget) {
    const previous = polyphones[target.paragraphId] ?? [];

    setPolyphones((prev) => {
      const next = { ...prev };
      delete next[target.paragraphId];
      return next;
    });
    setPolyAnchor(null);
    setSelection(null);
    void syncPolyphones(
      target.paragraphId,
      [],
      previous,
      `已清空本句 ${previous.length} 处注音，本段重新合成时生效`,
    );
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
              polyphones={polyphones}
              selection={selection}
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
              onSelectText={selectText}
              onRemovePolyphone={(paragraphId, mark) =>
                removeReading(paragraphId, mark.index, mark.char)
              }
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
          onPolyphone={openPolyphone}
        />

        {polyAnchor && polyTarget && (
          <PolyphonePopover
            char={polyTarget.char}
            current={polyTarget.reading}
            anchor={polyAnchor}
            onPick={applyReading}
            onRemove={
              polyTarget.reading
                ? () => removeReading(polyTarget.paragraphId, polyTarget.index, polyTarget.char)
                : undefined
            }
            onClear={
              polyphones[polyTarget.paragraphId]?.length
                ? () => clearSentenceReadings(polyTarget)
                : undefined
            }
            onClose={() => setPolyAnchor(null)}
          />
        )}
      </div>
    </div>
  );
}
