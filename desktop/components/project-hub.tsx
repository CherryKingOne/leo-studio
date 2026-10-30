"use client";

import { ListOrdered, Search, Star, Users, type LucideIcon } from "lucide-react";
import { useState } from "react";
import { CastMatrix } from "@/components/cast-matrix";
import { ChapterPipeline } from "@/components/chapter-pipeline";
import { useHubChapters } from "@/components/hub-chapters";
import type { CastMember } from "@/lib/projects";

type HubTab = "pipeline" | "cast";

export function ProjectHub({
  projectId,
  cast,
  appendReady,
}: {
  projectId: string;
  cast: CastMember[];
  /** 只有后端剧目能追更上传，原型剧目没有对应记录 */
  appendReady: boolean;
}) {
  const { chapters } = useHubChapters();
  const [tab, setTab] = useState<HubTab>("pipeline");
  const [keyword, setKeyword] = useState("");
  const [readyOnly, setReadyOnly] = useState(false);

  const tabs: { key: HubTab; label: string; Icon: LucideIcon }[] = [
    { key: "pipeline", label: `章节生产流水线 (${chapters.length})`, Icon: ListOrdered },
    // 后端还没有角色台账接口，无数据时不展示选角 tab
    ...(cast.length
      ? [{ key: "cast" as HubTab, label: `全书角色阵容与选角 (${cast.length})`, Icon: Users }]
      : []),
  ];

  function switchTab(next: HubTab) {
    setTab(next);
    setKeyword("");
  }

  return (
    <>
      <div className="mt-7 flex flex-wrap items-center justify-between gap-4">
        <div className="flex items-center gap-1">
          {tabs.map(({ key, label, Icon }) => (
            <button
              key={key}
              type="button"
              onClick={() => switchTab(key)}
              className={`flex items-center gap-2 rounded-lg px-2.5 py-1.5 text-[15px] transition-colors ${
                tab === key
                  ? "font-semibold text-white"
                  : "text-white/40 hover:text-white/70"
              }`}
            >
              <Icon className="h-4 w-4" />
              <span>{label}</span>
            </button>
          ))}
        </div>

        <div className="flex items-center gap-2">
          <button
            type="button"
            title="只看已就绪"
            onClick={() => setReadyOnly((value) => !value)}
            className={`rounded-full border p-2 transition-colors ${
              readyOnly
                ? "border-accent/30 bg-accent/15 text-accent-400"
                : "border-hairline bg-ink-800 text-white/50 hover:text-white"
            }`}
          >
            <Star className="h-4 w-4" />
          </button>

          <div className="relative">
            <Search className="pointer-events-none absolute left-3.5 top-2.5 h-4 w-4 text-white/30" />
            <input
              type="text"
              value={keyword}
              onChange={(e) => setKeyword(e.target.value)}
              placeholder="搜索章节或角色"
              className="w-[168px] rounded-full border border-hairline bg-ink-800 py-2 pl-9 pr-3 text-[12.5px] text-white placeholder:text-white/30 focus:border-accent/60 focus:outline-none xl:w-[220px]"
            />
          </div>
        </div>
      </div>

      {tab === "pipeline" ? (
        <ChapterPipeline
          projectId={projectId}
          appendReady={appendReady}
          keyword={keyword}
          readyOnly={readyOnly}
          onGoToCast={() => setTab("cast")}
        />
      ) : (
        <CastMatrix cast={cast} keyword={keyword} readyOnly={readyOnly} />
      )}
    </>
  );
}
