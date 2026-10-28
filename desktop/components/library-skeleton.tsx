import { TopBar } from "@/components/top-bar";

/**
 * 骨架屏：结构与真实列表页一致（固定侧栏 + 顶栏 + 工具栏 + 卡片网格），
 * 取数期间先占住版式，避免数据到位时页面跳动。
 */
const shimmer =
  "animate-shimmer bg-[linear-gradient(100deg,var(--color-ink-800)_38%,rgb(255_255_255/0.07)_50%,var(--color-ink-800)_62%)] bg-[length:200%_100%]";

function SidebarSkeleton() {
  return (
    <div className="fixed inset-y-0 left-0 z-40 hidden w-[236px] flex-col gap-3 border-r border-hairline bg-ink-900 px-3 pt-5 lg:flex">
      <div className={`${shimmer} mx-2 h-5 w-28 rounded-md`} />
      <div className={`${shimmer} mt-2 h-8 rounded-lg`} />
      <div className={`${shimmer} h-8 rounded-lg`} />
      <div className={`${shimmer} mt-3 h-8 w-40 rounded-lg`} />
      {[0, 1, 2].map((row) => (
        <div key={row} className={`${shimmer} ml-3 h-6 w-36 rounded-md`} />
      ))}
    </div>
  );
}

function CardSkeleton() {
  return (
    <div className="overflow-hidden rounded-2xl border border-hairline bg-ink-800">
      <div className={`${shimmer} aspect-[16/10] w-full`} />
      <div className="space-y-2 border-t border-hairline px-4 py-3.5">
        <div className={`${shimmer} h-3.5 w-3/5 rounded-full`} />
        <div className={`${shimmer} h-3 w-4/5 rounded-full`} />
      </div>
    </div>
  );
}

export function LibrarySkeleton() {
  return (
    <div className="flex min-h-screen">
      <SidebarSkeleton />

      <div className="flex min-w-0 flex-1 flex-col lg:ml-[236px]">
        <TopBar />
        <main className="flex-1 px-5 pb-16 pt-8 sm:px-8">
          <h1 className="text-[26px] font-bold leading-none tracking-tight text-white">
            我的剧目
          </h1>

          <div className="mt-6 flex flex-wrap items-center justify-between gap-3">
            <div className={`${shimmer} h-6 w-14 rounded-lg`} />
            <div className="flex items-center gap-2">
              <div className={`${shimmer} h-8 w-24 rounded-full`} />
              <div className={`${shimmer} size-8 rounded-full`} />
              <div className={`${shimmer} h-8 w-[164px] rounded-full xl:w-[220px]`} />
            </div>
          </div>

          <div className="mt-6 grid grid-cols-1 gap-5 sm:grid-cols-2 xl:grid-cols-3 2xl:grid-cols-4">
            {Array.from({ length: 8 }, (_, index) => (
              <CardSkeleton key={index} />
            ))}
          </div>
        </main>
      </div>
    </div>
  );
}
