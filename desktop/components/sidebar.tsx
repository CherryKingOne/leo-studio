"use client";

import {
  BookOpen,
  ChevronRight,
  CircleHelp,
  FileAudio,
  Folder,
  FolderOpen,
  History,
  MessageCircle,
  MicVocal,
  Plus,
} from "lucide-react";
import Link from "next/link";
import { Fragment, useState } from "react";
import { useImportNovelModal } from "@/components/import-novel-modal";
import { projectsInFolder, type LibraryEntry } from "@/lib/projects";

const navItem =
  "flex w-full items-center gap-2.5 rounded-lg px-3 py-2 text-[13px] text-white/65 transition-colors hover:bg-white/5 hover:text-white";
const subItem =
  "flex items-center gap-2.5 rounded-lg py-1.5 pl-3 pr-2 text-[12.5px] transition-colors hover:bg-white/5";

export function Sidebar({ entries }: { entries: LibraryEntry[] }) {
  const { open: openImportModal } = useImportNovelModal();
  const [groupExpanded, setGroupExpanded] = useState(true);

  const folders = entries.filter(
    (entry) => entry.kind === "folder" && !entry.trashed,
  );
  const rootProjects = entries.filter(
    (entry) => entry.kind === "project" && !entry.trashed && !entry.folderId,
  );

  return (
    <aside className="fixed inset-y-0 left-0 z-40 hidden w-[236px] flex-col border-r border-hairline bg-ink-900 lg:flex">
      <div className="flex h-16 items-center px-5">
        <Link href="/project" className="text-[20px] font-extrabold tracking-tight text-white">
          Voice<span className="text-accent">Novel</span>
        </Link>
      </div>

      <nav className="flex-1 overflow-y-auto px-3 pb-3">
        <div className="space-y-0.5">
          <button type="button" onClick={openImportModal} className={navItem}>
            <Plus className="h-4 w-4" />
            <span>新建剧目</span>
          </button>
          <Link href="/voice-library" className={navItem}>
            <MicVocal className="h-4 w-4" />
            <span>声线资产库</span>
          </Link>
        </div>

        <div className="my-3 h-px bg-white/7" />

        <button
          type="button"
          onClick={() => setGroupExpanded((expanded) => !expanded)}
          className="flex w-full items-center gap-2.5 rounded-lg bg-white/6 px-3 py-2 text-[13px] font-medium text-white transition-colors hover:bg-white/10"
        >
          <FolderOpen className="h-4 w-4 text-white/50" />
          <span className="flex-1 text-left">我的剧目</span>
          <ChevronRight
            className={`h-3.5 w-3.5 text-white/35 transition-transform ${
              groupExpanded ? "rotate-90" : ""
            }`}
          />
        </button>

        {groupExpanded && (
          <div className="mt-0.5 space-y-0.5 pl-3">
            {folders.map((folder) => (
              <Fragment key={folder.id}>
                <Link
                  href={`/folder/${folder.id}`}
                  className={`${subItem} font-medium text-white/70`}
                >
                  <Folder className="h-3.5 w-3.5 shrink-0 text-amber-300/45" />
                  <span className="truncate">{folder.title}</span>
                  <span className="ml-auto shrink-0 font-mono text-[10.5px] text-white/25">
                    {projectsInFolder(entries, folder.id).length}
                  </span>
                </Link>
                <div className="space-y-0.5 pl-4">
                  {projectsInFolder(entries, folder.id).map((project) => (
                    <Link
                      key={project.id}
                      href={`/project/${project.id}`}
                      className={`${subItem} text-white/45 hover:text-white`}
                    >
                      <FileAudio className="h-3.5 w-3.5 shrink-0 text-white/25" />
                      <span className="truncate">{project.title}</span>
                    </Link>
                  ))}
                </div>
              </Fragment>
            ))}

            {rootProjects.map((project) => (
              <Link
                key={project.id}
                href={`/project/${project.id}`}
                className={`${subItem} text-white/45 hover:text-white`}
              >
                <FileAudio className="h-3.5 w-3.5 shrink-0 text-white/25" />
                <span className="truncate">{project.title}</span>
              </Link>
            ))}
          </div>
        )}
      </nav>

      <div className="px-3 pb-3">
        <div className="flex items-center gap-2 rounded-xl border border-hairline bg-ink-800 px-3 py-2.5">
          <span className="h-1.5 w-1.5 shrink-0 rounded-full bg-emerald-400" />
          <span className="text-[12px] text-white/60">
            CosyVoice 2 推理集群 · 就绪
          </span>
        </div>
      </div>

      <div className="flex items-center gap-1 border-t border-hairline px-4 py-3 text-white/35">
        {(
          [
            ["帮助", CircleHelp],
            ["更新日志", BookOpen],
            ["生产记录", History],
            ["反馈", MessageCircle],
          ] as const
        ).map(([label, Icon]) => (
          <button
            key={label}
            type="button"
            title={label}
            className="rounded-md p-1.5 transition-colors hover:bg-white/5 hover:text-white"
          >
            <Icon className="h-4 w-4" />
          </button>
        ))}
      </div>
    </aside>
  );
}
