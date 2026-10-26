export type ProjectStatus = "制作中" | "母带就绪";
export type LibraryTab = "全部" | ProjectStatus | "回收站";
export type SortKey = "edited" | "progress" | "duration";

export type GenreTone = "accent" | "emerald" | "amber";

export type FolderEntry = {
  kind: "folder";
  id: string;
  title: string;
  editedHoursAgo: number;
  fav: boolean;
  trashed: boolean;
};

export type ProjectEntry = {
  kind: "project";
  id: string;
  title: string;
  /** 所属文件夹，缺省表示根目录 */
  folderId?: string;
  genre: string;
  status: ProjectStatus;
  chapters: number;
  /** 正在制作的章节序号，用于封面进度条文案 */
  currentChapter?: number;
  /** 待选角角色数，用于封面进度条文案 */
  pendingRoles?: number;
  progress: number;
  generatedSeconds: number;
  tone: GenreTone;
  seed: number;
  editedHoursAgo: number;
  fav: boolean;
  trashed: boolean;
};

export type LibraryEntry = FolderEntry | ProjectEntry;

export const libraryTabs: LibraryTab[] = ["全部", "制作中", "母带就绪", "回收站"];

export const sortOptions: { key: SortKey; label: string }[] = [
  { key: "edited", label: "最近编辑" },
  { key: "progress", label: "按进度" },
  { key: "duration", label: "按已生成时长" },
];

export const ttsModels = ["CosyVoice 2.0", "Fish Speech 1.5", "F5-TTS"];

export const initialEntries: LibraryEntry[] = [
  {
    kind: "folder",
    id: "f-folklore",
    title: "东方民俗系列",
    editedHoursAgo: 168,
    fav: false,
    trashed: false,
  },
  {
    kind: "project",
    id: "p-lord-of-mysteries",
    title: "诡秘之主：第一部 愚者",
    genre: "克苏鲁 / 西幻",
    status: "制作中",
    chapters: 12,
    currentChapter: 1,
    progress: 25,
    generatedSeconds: 858,
    tone: "accent",
    seed: 7,
    editedHoursAgo: 72,
    fav: true,
    trashed: false,
  },
  {
    kind: "project",
    id: "p-dao-gui-yi-xian",
    title: "道诡异仙：清旺来",
    folderId: "f-folklore",
    genre: "东方民俗 / 悬疑",
    status: "母带就绪",
    chapters: 1,
    progress: 100,
    generatedSeconds: 1120,
    tone: "emerald",
    seed: 23,
    editedHoursAgo: 8,
    fav: false,
    trashed: false,
  },
  {
    kind: "project",
    id: "p-long-night",
    title: "长夜余火：灰土列车",
    folderId: "f-folklore",
    genre: "废土 / 科幻",
    status: "制作中",
    chapters: 8,
    pendingRoles: 3,
    progress: 10,
    generatedSeconds: 0,
    tone: "amber",
    seed: 41,
    editedHoursAgo: 120,
    fav: false,
    trashed: false,
  },
];

export function relativeTime(hoursAgo: number) {
  if (hoursAgo < 1) return "刚刚";
  if (hoursAgo < 24) return `${Math.round(hoursAgo)} 小时前`;
  if (hoursAgo < 168) return `${Math.round(hoursAgo / 24)} 天前`;
  return `${Math.round(hoursAgo / 168)} 周前`;
}

export function durationLabel(seconds: number) {
  if (!seconds) return "待批量生成";
  return `已生成 ${Math.floor(seconds / 60)}分${String(seconds % 60).padStart(2, "0")}秒`;
}

export function coverHint(project: ProjectEntry) {
  if (project.progress === 100) return "全章混音完成";
  if (project.pendingRoles) return `${project.pendingRoles} 位角色待选角`;
  return `第 ${String(project.currentChapter ?? 1).padStart(3, "0")} 章 / 共 ${project.chapters} 章`;
}

export function projectsInFolder(entries: LibraryEntry[], folderId: string) {
  return entries.filter(
    (entry): entry is ProjectEntry =>
      entry.kind === "project" && !entry.trashed && entry.folderId === folderId,
  );
}

export function entryMeta(entry: LibraryEntry, entries: LibraryEntry[]) {
  if (entry.kind === "folder") {
    return `${projectsInFolder(entries, entry.id).length} 部剧目 · ${relativeTime(entry.editedHoursAgo)}`;
  }
  return [
    `${entry.chapters} 章`,
    `进度 ${entry.progress}%`,
    durationLabel(entry.generatedSeconds),
    relativeTime(entry.editedHoursAgo),
  ].join(" · ");
}

export function sortEntries<T extends LibraryEntry>(entries: T[], key: SortKey) {
  const value = (entry: T) =>
    key === "edited"
      ? -entry.editedHoursAgo
      : entry.kind === "project" && key === "progress"
        ? entry.progress
        : entry.kind === "project" && key === "duration"
          ? entry.generatedSeconds
          : 0;
  return [...entries].sort((a, b) => value(b) - value(a));
}
