import type { Metadata } from "next";
import { notFound } from "next/navigation";
import { ChapterStudio } from "@/components/chapter-studio";
import { ToastProvider } from "@/components/toast";
import { getChapterContent, getProject } from "@/lib/api";
import {
  chapterFromOutline,
  hubFor,
  initialProjects,
  studioFor,
  studioFromContent,
  type ChapterStudio as StudioData,
  type HubChapter,
  type StudioProject,
} from "@/lib/projects";

// 页面内容按 params 选剧目与章节，属于阻塞式路由
export const instant = false;

type StudioSource = {
  project: StudioProject;
  chapter: HubChapter;
  studio: StudioData;
};

/**
 * 迁移期两数据源：原型剧目用 `lib/projects.ts` 的 mock 剧本，
 * 后端剧目用 GET /projects/{id}/chapters/{chapterId} 的正文与逐句铺工作台。
 */
async function loadStudio(id: string, chapterId: string): Promise<StudioSource | null> {
  const mock = initialProjects.find((project) => project.id === id);
  if (mock) {
    const chapter = hubFor(mock).chapters.find((item) => item.id === chapterId);
    if (!chapter) return null;
    return { project: mock, chapter, studio: studioFor(mock, chapter) };
  }

  const [project, content] = await Promise.all([
    getProject(id),
    getChapterContent(id, chapterId),
  ]);
  if (!project || !content) return null;

  const chapter = chapterFromOutline({
    id: chapterId,
    index: content.index,
    title: content.title,
    words: content.words,
  });

  return {
    project: { id: project.id, title: project.title },
    chapter,
    studio: studioFromContent(chapter, content),
  };
}

/** 预渲染原型剧目里可进入的章节，后端剧目走请求时渲染 */
export function generateStaticParams() {
  return initialProjects.flatMap((project) =>
    hubFor(project)
      .chapters.filter((chapter) => chapter.openable)
      .map((chapter) => ({ id: project.id, chapterId: chapter.id })),
  );
}

export async function generateMetadata(
  props: PageProps<"/project/[id]/chapter/[chapterId]">,
): Promise<Metadata> {
  const { id, chapterId } = await props.params;
  const source = await loadStudio(id, chapterId);
  return {
    title: source ? `${source.chapter.title} · 制作工作台` : "章节制作工作台",
  };
}

export default async function ChapterStudioPage(
  props: PageProps<"/project/[id]/chapter/[chapterId]">,
) {
  const { id, chapterId } = await props.params;
  const source = await loadStudio(id, chapterId);
  if (!source) notFound();

  return (
    <ToastProvider>
      <ChapterStudio
        project={source.project}
        chapter={source.chapter}
        studio={source.studio}
      />
    </ToastProvider>
  );
}
