"use client";

import { useRef, useState } from "react";
import { floatingCardClass, useAnchoredCard, useDismiss, type AnchorRect } from "@/components/floating-card";
import { readingCandidates } from "@/lib/polyphone";

const pill =
  "flex items-center gap-1 rounded-lg border border-hairline bg-ink-850 px-2.5 py-1.5 text-[13px] text-white/75 transition-colors hover:border-white/25 hover:bg-ink-750 hover:text-white";

const pillOn =
  "flex items-center gap-1 rounded-lg border border-accent/45 bg-accent/15 px-2.5 py-1.5 text-[13px] font-medium text-accent-400";

/**
 * 多音字读音候选：挂在被划选那个字的下方。
 * 坐标由调用方从标注元素的矩形传入，弹窗用 fixed 定位避开滚动容器裁切。
 */
export function PolyphonePopover({
  char,
  current,
  anchor,
  onPick,
  onClose,
}: {
  char: string;
  /** 该字当前已选的读音，用于高亮候选项 */
  current?: string;
  anchor: AnchorRect;
  onPick: (reading: string) => void;
  onClose: () => void;
}) {
  const card = useRef<HTMLDivElement>(null);
  const [customizing, setCustomizing] = useState(false);
  const position = useAnchoredCard(card, anchor);

  useDismiss(card, onClose);

  const readings = readingCandidates(char);

  function commitCustom(value: string) {
    const reading = value.trim();
    if (reading) onPick(reading);
    else onClose();
    setCustomizing(false);
  }

  return (
    <div
      ref={card}
      style={{ left: position.left, top: position.top }}
      className={`${floatingCardClass} w-max max-w-[min(520px,calc(100vw-24px))] p-3`}
    >
      <div className="mb-2.5 flex items-center gap-2 px-0.5 text-[11.5px] text-white/45">
        <span className="text-[15px] font-semibold text-white">{char}</span>
        <span>选择本段读音</span>
        {current && <span className="text-emerald-300">当前 {current}</span>}
      </div>

      <div className="flex flex-wrap items-center gap-1.5">
        {readings.map((reading) => (
          <button
            key={reading.value}
            type="button"
            onClick={() => onPick(reading.value)}
            className={reading.value === current ? pillOn : pill}
          >
            {reading.value}
            {reading.common && (
              <span className="rounded bg-white/10 px-1 text-[9.5px] text-white/55">默</span>
            )}
          </button>
        ))}

        {customizing ? (
          <input
            autoFocus
            onBlur={(event) => commitCustom(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter") commitCustom(event.currentTarget.value);
              if (event.key === "Escape") setCustomizing(false);
            }}
            placeholder="输入拼音，回车确认"
            className="w-36 rounded-lg border border-accent/60 bg-ink-900 px-2.5 py-1.5 text-[13px] text-white outline-none placeholder:text-white/30"
          />
        ) : (
          <button type="button" onClick={() => setCustomizing(true)} className={pill}>
            自定义
          </button>
        )}
      </div>
    </div>
  );
}
