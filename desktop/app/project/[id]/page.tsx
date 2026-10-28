import type { Metadata } from "next";
import { notFound } from "next/navigation";
import { CloneVoiceModalProvider } from "@/components/clone-voice-modal";
import { HubChaptersProvider } from "@/components/hub-chapters";
import { HubHeader } from "@/components/hub-header";
import { ProjectHub } from "@/components/project-hub";
import { ToastProvider } from "@/components/toast";
import { getProject, listChapters } from "@/lib/api";
import {
  chapterFromOutline,
  hubFor,
  hubSubtitle,
  initialProjects,
  relativeTime,
  wordsLabel,
  type CastMember,
  type HubChapter,
  type HubIdentity,
} from "@/lib/projects";

// 页面内容按 params 选剧目，属于阻塞式路由
export const instant = false;

type HubData = {
  identity: HubIdentity;
  chapters: HubChapter[];
  cast: CastMember[];
  /** 章节是否已有制作工作台数据 */
  studioReady: boolean;
};

/**
 * 迁移期两数据源：原型剧目走 `lib/projects.ts` 的 mock（带角色台账与工作台剧本），
 * 其余剧目走后端项目详情与章节大纲，后端暂无生产状态与角色接口。
 */
async function loadHub(id: string): Promise<HubData | null> {
  const mock = initialProjects.find((project) => project.id === id);
  if (mock) {
    const hub = hubFor(mock);
    return {
      identity: {
        title: mock.title,
        subtitle: hubSubtitle(mock, hub.chapters),
        progress: mock.progress,
      },
      chapters: hub.chapters,
      cast: hub.cast,
      studioReady: true,
    };
  }

  const project = await getProject(id);
  if (!project) return null;

  const outline = await listChapters(id);
  return {
    identity: {
      title: project.title,
      subtitle: [
        project.filename,
        wordsLabel(project.totalWords),
        relativeTime(project.editedHoursAgo),
      ].join(" · "),
    },
    chapters: outline.map(chapterFromOutline),
    cast: [],
    studioReady: false,
  };
}

export function generateStaticParams() {
  return initialProjects.map((project) => ({ id: project.id }));
}

export async function generateMetadata(
  props: PageProps<"/project/[id]">,
): Promise<Metadata> {
  const { id } = await props.params;
  const hub = await loadHub(id);
  return {
    title: hub ? `${hub.identity.title} · 剧目看板` : "剧目看板",
  };
}

export default async function ProjectHubPage(props: PageProps<"/project/[id]">) {
  const { id } = await props.params;
  const hub = await loadHub(id);
  if (!hub) notFound();

  return (
    <ToastProvider>
      <CloneVoiceModalProvider>
        <HubChaptersProvider initialChapters={hub.chapters} studioReady={hub.studioReady}>
          <div className="flex min-h-screen flex-col">
            <HubHeader projectId={id} identity={hub.identity} />

            <main className="mx-auto w-full max-w-6xl flex-1 px-5 pb-16 pt-8 sm:px-8">
              <ProjectHub projectId={id} cast={hub.cast} />
            </main>
          </div>
        </HubChaptersProvider>
      </CloneVoiceModalProvider>
    </ToastProvider>
  );
}
