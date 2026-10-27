"use client";

import {
  ChevronDown,
  ChevronUp,
  GripVertical,
  Inbox,
  Play,
  Redo2,
  RotateCcw,
  Undo2,
  Zap,
} from "lucide-react";
import { Fragment, useState, type DragEvent } from "react";
import { useToast } from "@/components/toast";
import { inkScrollbar } from "@/lib/scrollbar";
import { studioClock, type CastOption, type HubTone, type StudioParagraph } from "@/lib/projects";

export type FeedItem = {
  paragraph: StudioParagraph;
  /** 当前顺序下的序号，从 1 开始 */
  seq: number;
  /** 本段起点（秒），由前序段落时长累加 */
  startSeconds: number;
  first: boolean;
  last: boolean;
};

const dotTone: Record<HubTone, string> = {
  neutral: "bg-white/45",
  muted: "bg-white/45",
  accent: "bg-accent",
  emerald: "bg-emerald-400",
  amber: "bg-amber-400",
  violet: "bg-violet-400",
};

const nameTone: Record<HubTone, string> = {
  neutral: "text-white/85",
  muted: "text-white/85",
  accent: "text-accent-400",
  emerald: "text-emerald-300",
  amber: "text-amber-300",
  violet: "text-violet-300",
};

const pillTone: Record<HubTone, string> = {
  neutral: "border-white/10 bg-white/5 text-white/55",
  muted: "border-white/10 bg-white/5 text-white/55",
  accent: "border-accent/25 bg-accent/10 font-medium text-accent-400",
  emerald: "border-emerald-400/20 bg-emerald-400/10 font-medium text-emerald-300",
  amber: "border-amber-400/20 bg-amber-400/10 font-medium text-amber-300",
  violet: "border-violet-400/20 bg-violet-400/10 font-medium text-violet-300",
};

const playTone: Record<HubTone, string> = {
  neutral:
    "border border-hairline bg-ink-850 text-white/55 hover:border-white/25 hover:bg-white/5 hover:text-white",
  muted:
    "border border-hairline bg-ink-850 text-white/55 hover:border-white/25 hover:bg-white/5 hover:text-white",
  accent: "bg-accent text-white hover:bg-accent-600",
  emerald:
    "border border-hairline bg-ink-850 text-white/55 hover:border-emerald-400/40 hover:bg-emerald-400/10 hover:text-emerald-300",
  amber:
    "border border-amber-400/25 bg-amber-400/10 text-amber-200 hover:bg-amber-400/20",
  violet:
    "border border-violet-400/25 bg-violet-400/10 text-violet-300 hover:bg-violet-400/20",
};

const orderButton =
  "flex items-center gap-1.5 rounded-full border border-hairline bg-ink-800 px-2.5 py-1.5 text-[12px] text-white/65 transition-colors hover:border-white/20 hover:text-white disabled:cursor-not-allowed disabled:opacity-35 disabled:hover:border-hairline";

const moveButton =
  "rounded-md p-1.5 text-white/40 transition-colors hover:bg-white/5 hover:text-white disabled:cursor-not-allowed disabled:opacity-30";

/** 按下处若是按钮/下拉/可编辑正文，临时关闭整卡拖拽以保留其原生交互 */
const interactiveSelector = "button, select, input, a, [contenteditable]";

function MarkLine({ side }: { side: "before" | "after" }) {
  return (
    <span
      className={`pointer-events-none absolute left-0 right-0 h-0.5 rounded-full bg-accent shadow-[0_0_12px_rgb(240_41_124/0.55)] ${
        side === "before" ? "-top-[9px]" : "-bottom-[9px]"
      }`}
    />
  );
}

/** 多音字校对：命中短语首字加虚线并注读音 */
function renderText(paragraph: StudioParagraph) {
  const { polyphone } = paragraph;
  if (!polyphone) return paragraph.text;

  const at = paragraph.text.indexOf(polyphone.phrase);
  if (at < 0) return paragraph.text;

  return (
    <>
      {paragraph.text.slice(0, at)}
      <span
        title={`多音字校对为：${polyphone.reading}`}
        className="border-b border-dotted border-amber-400/70 text-amber-200"
      >
        {polyphone.phrase.slice(0, 1)}
        <span className="align-super font-mono text-[9.5px] text-amber-400/90">
          ({polyphone.reading})
        </span>
      </span>
      {polyphone.phrase.slice(1)}
      {paragraph.text.slice(at + polyphone.phrase.length)}
    </>
  );
}

export function ScriptFeed({
  items,
  activeId,
  keyword,
  orderLabel,
  canUndo,
  canRedo,
  canReset,
  onUndo,
  onRedo,
  onReset,
  onMove,
  onReorder,
  onActivate,
  onReassign,
  onAudition,
}: {
  items: FeedItem[];
  activeId: string;
  keyword: string;
  orderLabel: string;
  canUndo: boolean;
  canRedo: boolean;
  canReset: boolean;
  onUndo: () => void;
  onRedo: () => void;
  onReset: () => void;
  onMove: (id: string, delta: number) => void;
  onReorder: (dragId: string, targetId: string, before: boolean) => void;
  onActivate: (id: string) => void;
  onReassign: (id: string, option: CastOption) => void;
  onAudition: (hz: number, seconds: number, label?: string) => void;
}) {
  const [dragId, setDragId] = useState<string | null>(null);
  const [mark, setMark] = useState<{ id: string; side: "before" | "after" } | null>(null);

  const trimmed = keyword.trim();
  // 关键词筛选时隐去音效提示，与原型一致
  const showCues = !trimmed;

  function dragStart(e: DragEvent<HTMLElement>, id: string) {
    e.dataTransfer.effectAllowed = "move";
    e.dataTransfer.setData("text/plain", id);
    setDragId(id);
  }

  function dragOver(e: DragEvent<HTMLElement>, id: string) {
    if (!dragId || id === dragId) return;
    e.preventDefault();
    const box = e.currentTarget.getBoundingClientRect();
    setMark({ id, side: e.clientY < box.top + box.height / 2 ? "before" : "after" });
  }

  function drop(e: DragEvent<HTMLElement>) {
    if (!dragId || !mark) return;
    e.preventDefault();
    onReorder(dragId, mark.id, mark.side === "before");
    setDragId(null);
    setMark(null);
  }

  return (
    <div className="flex h-full flex-col overflow-hidden rounded-2xl border border-hairline bg-ink-850">
      <div className="flex h-11 shrink-0 items-center border-b border-hairline px-4">
        <div className="mx-auto flex w-full max-w-3xl flex-wrap items-center justify-between gap-2">
          <div className="flex items-center gap-2 text-[11.5px]">
            <span className="text-white/35">段落顺序</span>
            <span className="font-mono text-white/55">{orderLabel}</span>
          </div>
          <div className="flex items-center gap-1.5">
            <button type="button" disabled={!canUndo} onClick={onUndo} className={orderButton}>
              <Undo2 className="h-3.5 w-3.5" />
              撤销
            </button>
            <button type="button" disabled={!canRedo} onClick={onRedo} className={orderButton}>
              <Redo2 className="h-3.5 w-3.5" />
              重做
            </button>
            <button type="button" disabled={!canReset} onClick={onReset} className={orderButton}>
              <RotateCcw className="h-3.5 w-3.5" />
              重置顺序
            </button>
          </div>
        </div>
      </div>

      <div className={`min-h-0 flex-1 overflow-y-auto px-4 py-4 ${inkScrollbar}`}>
        <div className="mx-auto max-w-3xl space-y-4">
          {items.map((item) => (
            <Fragment key={item.paragraph.id}>
              <ParagraphCard
                item={item}
                active={item.paragraph.id === activeId}
                dragging={dragId === item.paragraph.id}
                markSide={mark?.id === item.paragraph.id ? mark.side : null}
                onDragStart={dragStart}
                onDragOver={dragOver}
                onDrop={drop}
                onDragEnd={() => {
                  setDragId(null);
                  setMark(null);
                }}
                onActivate={onActivate}
                onMove={onMove}
                onReassign={onReassign}
                onAudition={onAudition}
              />

              {showCues && item.paragraph.cue && (
                <div className="flex justify-center">
                  <div className="inline-flex items-center gap-2 rounded-full border border-dashed border-amber-400/25 bg-amber-400/7 py-1.5 pl-3 pr-1.5 text-[11.5px] text-amber-200/90">
                    <Zap className="h-3.5 w-3.5 text-amber-400" />
                    <span className="truncate">{item.paragraph.cue.text}</span>
                    <button
                      type="button"
                      onClick={() => {
                        const cue = item.paragraph.cue!;
                        onAudition(cue.hz, cue.seconds, "拟音");
                      }}
                      className="shrink-0 rounded-full bg-amber-400/10 px-2 py-0.5 text-[10.5px] font-medium text-amber-200 transition-colors hover:bg-amber-400/20"
                    >
                      试听拟音
                    </button>
                  </div>
                </div>
              )}
            </Fragment>
          ))}

          {items.length === 0 && (
            <div className="rounded-2xl border border-dashed border-white/10 bg-ink-800 px-6 py-12 text-center">
              <Inbox className="mx-auto h-6 w-6 text-white/20" />
              <p className="mt-2.5 text-[12.5px] text-white/40">
                {trimmed ? `没有匹配「${trimmed}」的段落` : "没有匹配的段落"}
              </p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

function ParagraphCard({
  item,
  active,
  dragging,
  markSide,
  onDragStart,
  onDragOver,
  onDrop,
  onDragEnd,
  onActivate,
  onMove,
  onReassign,
  onAudition,
}: {
  item: FeedItem;
  active: boolean;
  dragging: boolean;
  markSide: "before" | "after" | null;
  onDragStart: (e: DragEvent<HTMLElement>, id: string) => void;
  onDragOver: (e: DragEvent<HTMLElement>, id: string) => void;
  onDrop: (e: DragEvent<HTMLElement>) => void;
  onDragEnd: () => void;
  onActivate: (id: string) => void;
  onMove: (id: string, delta: number) => void;
  onReassign: (id: string, option: CastOption) => void;
  onAudition: (hz: number, seconds: number, label?: string) => void;
}) {
  const { show } = useToast();
  const [draggable, setDraggable] = useState(false);
  const paragraph = item.paragraph;

  return (
    <section
      id={paragraph.id}
      draggable={draggable}
      onMouseDown={(e) => {
        if (e.target instanceof Element) setDraggable(!e.target.closest(interactiveSelector));
      }}
      onDragStart={(e) => onDragStart(e, paragraph.id)}
      onDragOver={(e) => onDragOver(e, paragraph.id)}
      onDrop={onDrop}
      onDragEnd={onDragEnd}
      onClick={(e) => {
        if (e.target instanceof Element && e.target.closest(interactiveSelector)) return;
        onActivate(paragraph.id);
      }}
      className={`relative cursor-grab rounded-2xl border bg-ink-800 p-5 transition-colors active:cursor-grabbing ${
        active ? "border-accent/25 ring-1 ring-accent/40" : "border-hairline hover:border-white/16"
      } ${dragging ? "opacity-40" : ""}`}
    >
      {markSide && <MarkLine side={markSide} />}

      <div className="flex items-center justify-between gap-3">
        <div className="flex min-w-0 flex-wrap items-center gap-2.5">
          <span title="拖拽整段可移动位置" className="-ml-1 flex h-6 w-4 shrink-0 items-center justify-center text-white/20">
            <GripVertical className="h-3.5 w-3.5" />
          </span>
          <span className={`h-1.5 w-1.5 shrink-0 rounded-full ${dotTone[paragraph.tone]}`} />
          <span className={`shrink-0 text-[12.5px] font-semibold ${nameTone[paragraph.tone]}`}>
            {paragraph.displayName}
          </span>
          <span
            className={`shrink-0 rounded border px-1.5 py-0.5 text-[10px] ${pillTone[paragraph.moodTone ?? paragraph.tone]}`}
          >
            {paragraph.mood}
          </span>
          <span className="shrink-0 font-mono text-[10.5px] text-white/25">
            #{String(item.seq).padStart(3, "0")}
          </span>
          {paragraph.aside && (
            <span className="shrink-0 font-mono text-[10.5px] text-white/25">{paragraph.aside}</span>
          )}
          {paragraph.castOptions && (
            <select
              defaultValue={paragraph.tuning.speaker}
              onChange={(e) => {
                const option = paragraph.castOptions?.find(
                  (candidate) => candidate.option === e.target.value,
                );
                if (option) onReassign(paragraph.id, option);
              }}
              className="shrink-0 rounded-lg border border-hairline bg-ink-900 px-2 py-0.5 text-[11px] text-white [color-scheme:dark] focus:border-accent/60 focus:outline-none"
            >
              {paragraph.castOptions.map((option) => (
                <option key={option.option} value={option.option}>
                  {option.option}
                </option>
              ))}
            </select>
          )}
        </div>

        <div className="flex shrink-0 items-center gap-2">
          <span className="font-mono text-[11px] text-white/40">
            {studioClock(item.startSeconds)} – {studioClock(item.startSeconds + paragraph.seconds)}
          </span>
          <div className="flex items-center gap-0.5">
            <MoveButton
              direction="up"
              disabled={item.first}
              onClick={() => onMove(paragraph.id, -1)}
            />
            <MoveButton
              direction="down"
              disabled={item.last}
              onClick={() => onMove(paragraph.id, 1)}
            />
          </div>
          <button
            type="button"
            title="试听本段"
            onClick={() => onAudition(paragraph.hz, paragraph.seconds)}
            className={`rounded-lg p-2 transition-colors ${playTone[paragraph.tone]}`}
          >
            <Play className="h-3.5 w-3.5" />
          </button>
        </div>
      </div>

      {paragraph.editable ? (
        <div
          contentEditable
          suppressContentEditableWarning
          spellCheck={false}
          onBlur={(e) => {
            const value = e.currentTarget.textContent?.trim() ?? "";
            if (value && value !== paragraph.text) show("文案已更新，后续合成使用新文案", "success");
          }}
          className="mt-3 cursor-text rounded-xl border border-hairline bg-ink-900 px-3.5 py-2.5 text-[13px] leading-relaxed text-white focus:underline focus:[text-decoration-style:dotted] focus:[text-decoration-color:rgb(255_255_255/0.35)] focus:[text-underline-offset:3px] focus:outline-none"
        >
          {paragraph.text}
        </div>
      ) : (
        <div className="mt-3 text-[13px] leading-relaxed text-white/75">
          {renderText(paragraph)}
        </div>
      )}

      <div className="mt-3.5 flex flex-wrap items-center justify-between gap-2 border-t border-hairline pt-3 text-[11.5px]">
        <span className="text-white/35">
          {paragraph.voice}
          {paragraph.voiceValue && (
            <>
              {" "}
              <span
                className={
                  paragraph.voiceValueMono ? "font-mono text-white/50" : "text-white/55"
                }
              >
                {paragraph.voiceValue}
              </span>
            </>
          )}
        </span>
        <div className="flex items-center gap-3">
          {paragraph.links.map((link) => (
            <button
              key={link.label}
              type="button"
              onClick={() => show(link.toast ?? "", link.tone ?? "info")}
              className={
                link.emphasis
                  ? "text-accent-400 hover:underline"
                  : "text-white/45 transition-colors hover:text-white"
              }
            >
              {link.label}
            </button>
          ))}
        </div>
      </div>
    </section>
  );
}

function MoveButton({
  direction,
  disabled,
  onClick,
}: {
  direction: "up" | "down";
  disabled: boolean;
  onClick: () => void;
}) {
  const Icon = direction === "up" ? ChevronUp : ChevronDown;
  return (
    <button
      type="button"
      disabled={disabled}
      title={direction === "up" ? "上移一段" : "下移一段"}
      onClick={onClick}
      className={moveButton}
    >
      <Icon className="h-3.5 w-3.5" />
    </button>
  );
}
