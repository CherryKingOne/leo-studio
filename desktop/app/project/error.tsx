"use client";

import { AlertTriangle } from "lucide-react";
import { useEffect } from "react";

export default function ProjectLibraryError({
  error,
  reset,
}: {
  error: Error;
  reset: () => void;
}) {
  useEffect(() => {
    console.error(error);
  }, [error]);

  return (
    <div className="flex min-h-screen items-center justify-center px-6">
      <div className="w-full max-w-md rounded-2xl border border-hairline bg-ink-800 px-6 py-7 text-center">
        <AlertTriangle className="mx-auto h-7 w-7 text-amber-300/70" />
        <h2 className="mt-3 text-[16px] font-semibold text-white">剧目列表读取失败</h2>
        <p className="mt-2 break-words text-[12.5px] leading-relaxed text-white/45">
          {error.message}
        </p>
        <p className="mt-3 text-[11.5px] leading-relaxed text-white/30">
          确认后端服务已启动，且 desktop/.env.local 的 API_BASE_URL 指向该地址。
        </p>
        <button
          type="button"
          onClick={reset}
          className="mt-5 rounded-lg bg-accent px-4 py-2 text-[13px] font-medium text-white transition-colors hover:bg-accent-600"
        >
          重试
        </button>
      </div>
    </div>
  );
}
