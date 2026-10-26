"use client";

import { X } from "lucide-react";
import { useEffect, type ReactNode } from "react";

export const fieldLabel = "mb-1.5 block text-[12.5px] font-medium text-white/75";
export const fieldControl =
  "w-full rounded-lg border border-hairline bg-ink-900 px-3 py-2.5 text-[13px] text-white placeholder:text-white/25 focus:border-accent/60 focus:outline-none";
export const modalBody = "space-y-5 px-6 py-5";
export const modalFooter =
  "flex items-center justify-end gap-2 border-t border-hairline px-6 py-4";
export const secondaryButton =
  "rounded-lg border border-hairline px-4 py-2 text-[13px] text-white/65 hover:bg-white/5 hover:text-white";
export const primaryButton =
  "flex items-center gap-1.5 rounded-lg bg-accent px-4 py-2 text-[13px] font-medium text-white hover:bg-accent-600 disabled:cursor-not-allowed disabled:opacity-40";

export function Modal({
  title,
  description,
  children,
  onClose,
}: {
  title: string;
  description?: string;
  children: ReactNode;
  onClose: () => void;
}) {
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, [onClose]);

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-label={title}
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/75 p-4 backdrop-blur-sm"
    >
      <div className="w-full max-w-[520px] rounded-2xl border border-hairline bg-ink-800 shadow-2xl shadow-black/70">
        <div className="flex items-center justify-between border-b border-hairline px-6 py-4">
          <div>
            <h2 className="text-[15px] font-semibold text-white">{title}</h2>
            {description && (
              <p className="mt-0.5 text-[11.5px] text-white/40">{description}</p>
            )}
          </div>
          <button
            type="button"
            onClick={onClose}
            title="关闭"
            className="rounded-md p-1.5 text-white/40 hover:bg-white/5 hover:text-white"
          >
            <X className="h-4 w-4" />
          </button>
        </div>
        {children}
      </div>
    </div>
  );
}
