import { Keyboard, MessageCircleMore } from "lucide-react";

const buttonClass =
  "flex h-9 w-9 items-center justify-center rounded-full border border-hairline bg-ink-800 text-white/50 transition-colors hover:text-white";

export function FloatingActions() {
  return (
    <div className="fixed right-4 top-1/2 z-30 hidden -translate-y-1/2 flex-col gap-2 2xl:flex">
      <button type="button" title="意见反馈" className={buttonClass}>
        <MessageCircleMore className="h-4 w-4" />
      </button>
      <button type="button" title="快捷键" className={buttonClass}>
        <Keyboard className="h-4 w-4" />
      </button>
    </div>
  );
}
