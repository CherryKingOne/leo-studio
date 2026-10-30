"use client";

import { useEffect, useLayoutEffect, useState, type RefObject } from "react";

/** 浮层锚点：目标元素在视口里的水平中点与上下边界 */
export type AnchorRect = { x: number; top: number; bottom: number };

export function anchorOf(element: Element | null): AnchorRect | null {
  if (!element) return null;
  const box = element.getBoundingClientRect();
  return { x: box.left + box.width / 2, top: box.top, bottom: box.bottom };
}

/** 卡片跟随锚点：水平居中并夹在视口内，下方放不下就翻到上方 */
export function useAnchoredCard(ref: RefObject<HTMLElement | null>, anchor: AnchorRect) {
  const [position, setPosition] = useState({ left: anchor.x, top: anchor.bottom });

  // 量完真实尺寸再定位，贴边与翻转都在首帧完成
  useLayoutEffect(() => {
    const box = ref.current?.getBoundingClientRect();
    if (!box) return;
    const left = Math.min(
      Math.max(anchor.x - box.width / 2, 12),
      window.innerWidth - box.width - 12,
    );
    const below = anchor.bottom + box.height + 12 < window.innerHeight;
    setPosition({ left, top: below ? anchor.bottom + 8 : anchor.top - box.height - 8 });
  }, [anchor, ref]);

  return position;
}

/** 点卡片外或按 Esc 即收起 */
export function useDismiss(ref: RefObject<HTMLElement | null>, onClose: () => void) {
  useEffect(() => {
    function onPointerDown(event: MouseEvent) {
      if (!ref.current?.contains(event.target as Node)) onClose();
    }
    function onKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") onClose();
    }
    document.addEventListener("mousedown", onPointerDown);
    document.addEventListener("keydown", onKeyDown);
    return () => {
      document.removeEventListener("mousedown", onPointerDown);
      document.removeEventListener("keydown", onKeyDown);
    };
  }, [onClose, ref]);
}

export const floatingCardClass =
  "fixed z-50 rounded-2xl border border-hairline bg-ink-800 shadow-2xl shadow-black/70";
