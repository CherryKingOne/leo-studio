"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";

const tabs = [
  { href: "/project", label: "全部" },
  { href: "/project/trash", label: "回收站" },
];

/** 剧目库与回收站共用的一级 tab，两个 tab 是两条独立路由 */
export function LibraryTabs({ trashCount }: { trashCount?: number }) {
  const pathname = usePathname();

  return (
    <div className="flex items-center gap-1">
      {tabs.map((tab) => {
        const active = pathname === tab.href;
        return (
          <Link
            key={tab.href}
            href={tab.href}
            scroll={false}
            className={`rounded-lg px-2.5 py-1.5 text-[15px] transition-colors ${
              active ? "font-semibold text-white" : "text-white/40 hover:text-white/70"
            }`}
          >
            {tab.label}
            {tab.label === "回收站" && !!trashCount && (
              <span className="ml-1.5 font-mono text-[11.5px] text-white/35">{trashCount}</span>
            )}
          </Link>
        );
      })}
    </div>
  );
}
