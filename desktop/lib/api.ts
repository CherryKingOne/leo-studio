import { cacheLife, cacheTag } from "next/cache";
import type {
  ChapterContent,
  ProjectChapter,
  ProjectStatus,
  ProjectSummary,
  TrashedProject,
} from "@/lib/projects";

type ApiEnvelope<T> = {
  code: number;
  message: string;
  data: T;
  request_id: string | null;
};

type ApiProject = {
  project_id: string;
  name: string;
  file_id: string;
  filename: string;
  status: ProjectStatus;
  chapter_count: number;
  total_words: number;
  created_at: string;
  updated_at: string;
  is_deleted?: boolean;
  deleted_at?: string | null;
};

type ApiProjectList = {
  total: number;
  page: number;
  page_size: number;
  pages: number;
  items: ApiProject[];
};

type ApiChapterOutline = {
  chapter_id: string;
  chapter_index: number;
  chapter_title: string;
  word_count: number;
};

type ApiChapterContent = {
  project_id: string;
  chapter_id: string;
  chapter_index: number;
  chapter_title: string;
  word_count: number;
  content: string;
  sentences: {
    sentence_id: string;
    sentence_index: number;
    content: string;
    word_count: number;
  }[];
};

function backendUrl() {
  const base = process.env.API_BASE_URL;
  if (!base) {
    throw new Error(
      "未配置 API_BASE_URL：复制 desktop/.env.example 为 desktop/.env.local 并填写后端地址",
    );
  }
  return base.replace(/\/+$/, "");
}

/** 后端统一响应包装：HTTP 200 但 code 非 0 同样按失败处理 */
async function apiSend<T>(
  method: "GET" | "POST" | "DELETE",
  path: string,
  params: Record<string, string | number> = {},
): Promise<T> {
  const base = backendUrl();
  const query = new URLSearchParams(
    Object.entries(params).map(([key, value]) => [key, String(value)]),
  );
  let res: Response;
  try {
    res = await fetch(`${base}${path}?${query}`, {
      method,
      headers: { Accept: "application/json" },
    });
  } catch (cause) {
    throw new Error(`无法连接后端 ${base}`, { cause });
  }
  const body = (await res.json().catch(() => null)) as ApiEnvelope<T> | null;

  if (res.status === 404 || body?.code === 40400) {
    return null as T;
  }
  if (!body || body.code !== 0) {
    throw new Error(`${method} ${path} 请求失败：${body?.message ?? `HTTP ${res.status}`}`);
  }
  return body.data;
}

const apiGet = <T>(path: string, params?: Record<string, string | number>) =>
  apiSend<T>("GET", path, params);

function toSummary(item: ApiProject, now: number): ProjectSummary {
  return {
    id: item.project_id,
    title: item.name,
    filename: item.filename,
    chapters: item.chapter_count,
    totalWords: item.total_words,
    // 相对时间在服务端算成小时数后下发，避免水合两端各取当前时刻导致不一致
    editedHoursAgo: Math.max(0, (now - Date.parse(item.updated_at)) / 3_600_000),
  };
}

/** multipart 写接口：不手动设 Content-Type，交给 fetch 生成 boundary */
async function apiPostForm<T>(path: string, form: FormData): Promise<T> {
  const base = backendUrl();
  let res: Response;
  try {
    res = await fetch(`${base}${path}`, { method: "POST", body: form });
  } catch (cause) {
    throw new Error(`无法连接后端 ${base}`, { cause });
  }

  const body = (await res.json().catch(() => null)) as ApiEnvelope<T> | null;
  if (!body || body.code !== 0) {
    throw new Error(body?.message ?? `请求失败（HTTP ${res.status}）`);
  }
  return body.data;
}

/** 新建剧目：POST /api/v1/projects，multipart 上传 .txt 正文，服务端自动清洗分章 */
export async function createProject(name: string, file: File): Promise<ProjectSummary> {
  const form = new FormData();
  form.set("name", name);
  form.set("file", file, file.name);

  const data = await apiPostForm<ApiProject>("/api/v1/projects", form);
  return toSummary(data, Date.now());
}

/** 追更上传的预检/执行结果，字段对齐 ProjectChapterAppendResponse */
export type ChapterAppendResult = {
  addedChapters: number;
  skippedChapters: number;
  totalChapters: number;
  totalWords: number;
  hasConflicts: boolean;
  conflicts: {
    chapterTitle: string;
    existingWordCount: number;
    incomingWordCount: number;
    wordDiff: number;
  }[];
};

type ApiChapterAppend = {
  project_id: string;
  file_id: string;
  added_chapters: number;
  skipped_chapters: number;
  total_chapters: number;
  total_words: number;
  has_conflicts: boolean;
  conflicts: {
    chapter_id: string;
    chapter_index: number;
    chapter_title: string;
    existing_word_count: number;
    incoming_word_count: number;
    word_diff: number;
  }[];
  updated_at: string;
};

/**
 * 追更上传章节：POST /api/v1/projects/{id}/chapters/upload
 * dryRun=true 只做差异预检、不落盘，用来渲染弹窗里的「将要导入」摘要。
 */
export async function appendChapters(
  projectId: string,
  file: File,
  options: { overwriteExisting: boolean; dryRun: boolean },
): Promise<ChapterAppendResult> {
  const form = new FormData();
  form.set("file", file, file.name);
  form.set("overwrite_existing", String(options.overwriteExisting));
  form.set("dry_run", String(options.dryRun));

  const data = await apiPostForm<ApiChapterAppend>(
    `/api/v1/projects/${projectId}/chapters/upload`,
    form,
  );

  return {
    addedChapters: data.added_chapters,
    skippedChapters: data.skipped_chapters,
    totalChapters: data.total_chapters,
    totalWords: data.total_words,
    hasConflicts: data.has_conflicts,
    conflicts: data.conflicts.map((item) => ({
      chapterTitle: item.chapter_title,
      existingWordCount: item.existing_word_count,
      incomingWordCount: item.incoming_word_count,
      wordDiff: item.word_diff,
    })),
  };
}

/** 剧目库列表：GET /api/v1/projects，单页取后端允许的上限 100 条 */
export async function listProjects(): Promise<ProjectSummary[]> {
  // 缓 5 秒：prefetch 与页面内多次导航不再反复打后端，新建/删除后用 revalidateTag("projects") 立即失效
  "use cache";
  cacheLife("seconds");
  cacheTag("projects");

  const data = await apiGet<ApiProjectList>("/api/v1/projects", { page: 1, page_size: 100 });
  const now = Date.now();

  return data.items.map((item) => toSummary(item, now));
}

/** 剧目详情：GET /api/v1/projects/{id}，后端没有该剧目时返回 null */
export async function getProject(id: string): Promise<ProjectSummary | null> {
  "use cache";
  cacheLife("seconds");
  cacheTag("projects");

  const data = await apiGet<ApiProject>(`/api/v1/projects/${id}`);
  return data ? toSummary(data, Date.now()) : null;
}

/** 章节大纲：GET /api/v1/projects/{id}/chapters，按章节序号升序 */
export async function listChapters(id: string): Promise<ProjectChapter[]> {
  "use cache";
  cacheLife("seconds");
  cacheTag("projects");

  const data = await apiGet<{ project_id: string; total: number; items: ApiChapterOutline[] }>(
    `/api/v1/projects/${id}/chapters`,
  );
  if (!data) return [];

  return data.items.map((item) => ({
    id: item.chapter_id,
    index: item.chapter_index,
    title: item.chapter_title,
    words: item.word_count,
  }));
}

/**
 * 章节正文与逐句切分：GET /api/v1/projects/{id}/chapters/{chapterId}
 * 制作工作台用它铺剧本流与原文对照，后端没有该章节时返回 null。
 */
export async function getChapterContent(
  projectId: string,
  chapterId: string,
): Promise<ChapterContent | null> {
  "use cache";
  cacheLife("seconds");
  cacheTag("projects");

  const data = await apiGet<ApiChapterContent>(
    `/api/v1/projects/${projectId}/chapters/${chapterId}`,
  );
  if (!data) return null;

  return {
    index: data.chapter_index,
    title: data.chapter_title,
    words: data.word_count,
    content: data.content,
    sentences: data.sentences.map((item) => ({
      id: item.sentence_id,
      index: item.sentence_index,
      text: item.content,
      words: item.word_count,
    })),
  };
}

/** 回收站列表：GET /api/v1/projects/recycle-bin，按删除时间倒序 */
export async function listTrash(): Promise<TrashedProject[]> {
  "use cache";
  cacheLife("seconds");
  cacheTag("projects");

  const data = await apiGet<ApiProjectList>("/api/v1/projects/recycle-bin", {
    page: 1,
    page_size: 100,
  });
  const now = Date.now();

  return data.items.map((item) => ({
    ...toSummary(item, now),
    deletedHoursAgo: item.deleted_at
      ? Math.max(0, (now - Date.parse(item.deleted_at)) / 3_600_000)
      : 0,
  }));
}

/** 移入回收站（软删除）：DELETE /api/v1/projects/{id} */
export async function deleteProject(id: string): Promise<string | null> {
  const data = await apiSend<{ project_id: string; name: string; deleted: boolean }>(
    "DELETE",
    `/api/v1/projects/${id}`,
  );
  return data?.name ?? null;
}

/** 从回收站恢复：POST /api/v1/projects/{id}/restore */
export async function restoreProject(id: string): Promise<void> {
  await apiSend("POST", `/api/v1/projects/${id}/restore`);
}

/** 彻底删除（物理硬删除，不可逆）：DELETE /api/v1/projects/{id}/hard */
export async function hardDeleteProject(id: string): Promise<void> {
  await apiSend("DELETE", `/api/v1/projects/${id}/hard`);
}

/** 清空回收站：DELETE /api/v1/projects/recycle-bin */
export async function clearTrash(): Promise<number> {
  const data = await apiSend<{ cleared_count: number; cleared: boolean }>(
    "DELETE",
    "/api/v1/projects/recycle-bin",
  );
  return data?.cleared_count ?? 0;
}
