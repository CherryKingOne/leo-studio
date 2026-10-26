"use client";

import { CircleCheck, Info } from "lucide-react";
import {
  createContext,
  useCallback,
  useContext,
  useRef,
  useState,
  type ReactNode,
} from "react";

type ToastTone = "info" | "success";
type ToastItem = { id: number; message: string; tone: ToastTone };

const ToastContext = createContext<{ show: (message: string, tone?: ToastTone) => void } | null>(
  null,
);

export function useToast() {
  const value = useContext(ToastContext);
  if (!value) {
    throw new Error("useToast 必须在 ToastProvider 内使用");
  }
  return value;
}

export function ToastProvider({ children }: { children: ReactNode }) {
  const [toasts, setToasts] = useState<ToastItem[]>([]);
  const seq = useRef(0);

  const show = useCallback((message: string, tone: ToastTone = "info") => {
    seq.current += 1;
    const id = seq.current;
    setToasts((prev) => [...prev, { id, message, tone }]);
    setTimeout(() => {
      setToasts((prev) => prev.filter((item) => item.id !== id));
    }, 2500);
  }, []);

  return (
    <ToastContext.Provider value={{ show }}>
      {children}

      <div className="pointer-events-none fixed bottom-6 right-6 z-50 flex flex-col gap-2">
        {toasts.map(({ id, message, tone }) => {
          const Icon = tone === "success" ? CircleCheck : Info;
          return (
            <div
              key={id}
              className="pointer-events-auto flex items-center gap-2 rounded-xl border border-hairline bg-ink-750 px-4 py-2.5 text-[12.5px] text-white/80 shadow-2xl shadow-black/60"
            >
              <Icon
                className={`h-4 w-4 shrink-0 ${
                  tone === "success" ? "text-emerald-400" : "text-accent-400"
                }`}
              />
              <span>{message}</span>
            </div>
          );
        })}
      </div>
    </ToastContext.Provider>
  );
}
