"use client";

import { Check } from "lucide-react";
import { useRef } from "react";
import { floatingCardClass, useAnchoredCard, useDismiss, type AnchorRect } from "@/components/floating-card";
import { prosodyOptions, type ProsodyKind } from "@/lib/projects";

const row =
  "flex w-full items-center justify-between gap-6 px-3.5 py-2.5 text-left text-[13px] text-white/80 transition-colors hover:bg-white/5";

/**
 * 语气标注菜单：重读 / 拖音 / 重读 + 拖音。
 * 当前区间已有标注时勾选项跟着走，并多出「移除」把标注撤掉。
 */
export function ProsodyMenu({
  current,
  anchor,
  onPick,
  onRemove,
  onClose,
}: {
  /** 该区间已有的标注类型，缺省表示还没标过 */
  current?: ProsodyKind;
  anchor: AnchorRect;
  onPick: (kind: ProsodyKind) => void;
  onRemove: () => void;
  onClose: () => void;
}) {
  const card = useRef<HTMLDivElement>(null);
  const position = useAnchoredCard(card, anchor);

  useDismiss(card, onClose);

  return (
    <div
      ref={card}
      style={{ left: position.left, top: position.top }}
      className={`${floatingCardClass} w-[188px] py-1`}
    >
      {prosodyOptions.map((option) => (
        <button
          key={option.kind}
          type="button"
          onClick={() => onPick(option.kind)}
          className={row}
        >
          {option.label}
          {current === option.kind && <Check className="h-4 w-4 shrink-0 text-accent-400" />}
        </button>
      ))}

      {current && (
        <div className="mt-1 flex justify-end border-t border-hairline px-3.5 py-2.5">
          <button
            type="button"
            onClick={onRemove}
            className="rounded-lg border border-hairline bg-ink-850 px-2.5 py-1 text-[12px] text-white/65 transition-colors hover:border-white/25 hover:text-white"
          >
            移除
          </button>
        </div>
      )}
    </div>
  );
}
