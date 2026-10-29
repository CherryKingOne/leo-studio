"use client";

import { useEffect, useRef } from "react";
import { inkScrollbar } from "@/lib/scrollbar";
import {
  formatWords,
  studioPreview,
  type HubTone,
  type StudioParagraph,
  type StudioSourceBlock,
  type StudioTocEntry,
} from "@/lib/projects";

export type StudioPane = "outline" | "source" | "toc";

const paneTabs: [StudioPane, string][] = [
  ["outline", "大纲"],
  ["source", "正文"],
  ["toc", "目录"],
];

const dotTone: Record<HubTone, string> = {
  neutral: "bg-white/45",
  muted: "bg-white/45",
  accent: "bg-accent",
  emerald: "bg-emerald-400",
  amber: "bg-amber-400",
  violet: "bg-violet-400",
};

export function ParagraphOutline({
  items,
  source,
  toc,
  pane,
  activeId,
  words,
  onPaneChange,
  onJumpToFeed,
  onTocSelect,
}: {
  items: { paragraph: StudioParagraph; seq: number }[];
  source: StudioSourceBlock[];
  toc: StudioTocEntry[];
  pane: StudioPane;
  activeId: string;
  words: number;
  onPaneChange: (pane: StudioPane) => void;
  onJumpToFeed: (id: string) => void;
  onTocSelect: (entry: StudioTocEntry) => void;
}) {
  // 每个剧本段落只把第一次出现的正文块挂 id，供工作台滚动定位
  const firstIndex = new Map<string, number>();
  source.forEach((block, index) => {
    if (!firstIndex.has(block.paragraphId)) firstIndex.set(block.paragraphId, index);
  });

  // 整本书上千章，切到目录 pane 时先滚到当前章那一行
  const currentRow = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    if (pane === "toc") currentRow.current?.scrollIntoView({ block: "center" });
  }, [pane]);

  return (
    <aside
      className={`hidden shrink-0 p-3 transition-[width] duration-200 ease-out md:block ${
        pane === "source" ? "w-[440px]" : pane === "toc" ? "w-[320px]" : "w-[280px]"
      }`}
    >
      <div className="flex h-full flex-col overflow-hidden rounded-2xl border border-hairline bg-ink-800">
        <div className="flex h-11 shrink-0 items-center gap-2 border-b border-hairline px-2.5">
          <div className="flex items-center gap-0.5 rounded-lg bg-ink-900 p-0.5">
            {paneTabs.map(([key, label]) => (
              <button
                key={key}
                type="button"
                onClick={() => onPaneChange(key)}
                className={`rounded-md px-2.5 py-1 text-[12px] transition-colors ${
                  pane === key ? "bg-white/9 text-white" : "text-white/45 hover:text-white/80"
                }`}
              >
                {label}
              </button>
            ))}
          </div>
          <span className="ml-auto shrink-0 font-mono text-[10.5px] text-white/30">
            {pane === "outline"
              ? `共 ${items.length} 段`
              : pane === "source"
                ? `${formatWords(words)} 字`
                : `全 ${toc.length} 章`}
          </span>
        </div>

        {pane === "outline" ? (
          <nav className={`flex-1 space-y-0.5 overflow-y-auto p-2.5 ${inkScrollbar}`}>
            {items.map(({ paragraph, seq }) => {
              const active = paragraph.id === activeId;
              return (
                <button
                  key={paragraph.id}
                  id={`outline-${paragraph.id}`}
                  type="button"
                  onClick={() => onJumpToFeed(paragraph.id)}
                  className={`flex w-full items-center gap-2.5 rounded-lg py-2 pl-2.5 pr-2 text-left text-[12.5px] transition-colors ${
                    active
                      ? "bg-white/6 text-white ring-1 ring-accent/25"
                      : "text-white/50 hover:bg-white/5 hover:text-white"
                  }`}
                >
                  <span
                    className={`h-1.5 w-1.5 shrink-0 rounded-full ${dotTone[paragraph.tone]}`}
                  />
                  <span className="min-w-0 flex-1 truncate">
                    <span className="font-mono text-white/35">
                      #{String(seq).padStart(2, "0")}
                    </span>{" "}
                    {paragraph.speaker} · {studioPreview(paragraph.text)}
                  </span>
                  <span
                    className={`shrink-0 font-mono text-[10.5px] ${
                      active ? "text-white/40" : "text-white/30"
                    }`}
                  >
                    {paragraph.seconds}s
                  </span>
                </button>
              );
            })}
          </nav>
        ) : pane === "source" ? (
          <div
            className={`flex-1 space-y-4 overflow-y-auto p-4 text-[13px] leading-relaxed text-white/60 ${inkScrollbar}`}
          >
            {source.map((block, index) => {
              const active = block.paragraphId === activeId;
              const isFirst = firstIndex.get(block.paragraphId) === index;
              return (
                <p
                  key={`${block.paragraphId}-${index}`}
                  id={isFirst ? `source-${block.paragraphId}` : undefined}
                  onClick={() => onJumpToFeed(block.paragraphId)}
                  className={`cursor-pointer rounded-lg px-2.5 py-1.5 transition-colors hover:bg-white/4 hover:text-white/85 ${
                    active ? "bg-accent/8 text-white/90" : ""
                  }`}
                >
                  {block.text}
                </p>
              );
            })}
            <p className="border-t border-hairline pt-3 text-[11px] leading-relaxed text-white/30">
              点击正文段落可跳转到对应的剧本段落卡片；原文仅作对照，不可直接编辑。
            </p>
          </div>
        ) : (
          <nav className="flex min-h-0 flex-1 flex-col">
            <ul className={`flex-1 space-y-0.5 overflow-y-auto p-2.5 ${inkScrollbar}`}>
              {toc.map((entry) => (
                <li key={entry.id}>
                  <button
                    ref={entry.current ? currentRow : undefined}
                    type="button"
                    onClick={() => onTocSelect(entry)}
                    className={`flex w-full items-center gap-2.5 rounded-lg py-2 pl-2.5 pr-2 text-left text-[12.5px] transition-colors ${
                      entry.current
                        ? "bg-accent/8 text-white ring-1 ring-accent/25"
                        : "text-white/50 hover:bg-white/5 hover:text-white"
                    }`}
                  >
                    <span className="shrink-0 font-mono text-[10.5px] text-white/35">
                      {entry.no}
                    </span>
                    <span className="min-w-0 flex-1 truncate">{entry.title}</span>
                    <span className="shrink-0 font-mono text-[10.5px] text-white/30">
                      {formatWords(entry.words)}
                    </span>
                  </button>
                </li>
              ))}
            </ul>
            <p className="shrink-0 border-t border-hairline px-3 py-2.5 text-[11px] leading-relaxed text-white/30">
              目录取自全书章节大纲，点击任意章节即切到该章的制作工作台。
            </p>
          </nav>
        )}
      </div>
    </aside>
  );
}
