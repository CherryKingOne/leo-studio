"use client";

import { Mic, Volume2 } from "lucide-react";
import Link from "next/link";
import { useRef } from "react";
import { useCloneVoiceModal } from "@/components/clone-voice-modal";
import { useToast } from "@/components/toast";
import type { CastMember, HubTone } from "@/lib/projects";

const avatarTone: Record<HubTone, string> = {
  neutral: "border-white/10 bg-white/5 text-white/70",
  muted: "border-white/10 bg-white/5 text-white/70",
  accent: "border-accent/30 bg-accent/10 text-accent-400",
  emerald: "border-emerald-400/20 bg-emerald-400/10 text-emerald-300",
  amber: "border-amber-400/20 bg-amber-400/10 text-amber-300",
  violet: "border-violet-400/20 bg-violet-400/10 text-violet-300",
};

const auditionTone: Record<HubTone, string> = {
  neutral:
    "border border-hairline bg-ink-850 text-white/50 hover:border-white/25 hover:bg-white/5 hover:text-white",
  muted:
    "border border-hairline bg-ink-850 text-white/50 hover:border-white/25 hover:bg-white/5 hover:text-white",
  accent: "bg-accent text-white transition-colors hover:bg-accent-600",
  emerald:
    "border border-hairline bg-ink-850 text-white/50 hover:border-emerald-400/40 hover:bg-emerald-400/10 hover:text-emerald-300",
  amber:
    "border border-hairline bg-ink-850 text-white/50 hover:border-amber-400/40 hover:bg-amber-400/10 hover:text-amber-300",
  violet:
    "border border-hairline bg-ink-850 text-white/50 hover:border-violet-400/40 hover:bg-violet-400/10 hover:text-violet-300",
};

const roleBadgeTone: Record<HubTone, string> = {
  neutral: "border-white/10 bg-white/5 text-white/55",
  muted: "border-white/10 bg-white/5 text-white/55",
  accent: "border-accent/25 bg-accent/10 text-accent-400",
  emerald: "border-emerald-400/20 bg-emerald-400/10 text-emerald-300",
  amber: "border-amber-400/20 bg-amber-400/10 text-amber-300",
  violet: "border-violet-400/20 bg-violet-400/10 text-violet-300",
};

const factTone: Record<HubTone, string> = {
  neutral: "text-white/75",
  muted: "text-white/75",
  accent: "text-accent-400",
  emerald: "text-emerald-300",
  amber: "text-amber-300",
  violet: "text-violet-300",
};

export function CastMatrix({
  cast,
  keyword,
  readyOnly,
}: {
  cast: CastMember[];
  keyword: string;
  readyOnly: boolean;
}) {
  const { open: openCloneModal } = useCloneVoiceModal();

  const needle = keyword.trim().toLowerCase();
  const visible = cast.filter(
    (member) =>
      (!readyOnly || member.state === "已就绪") &&
      (!needle || member.name.toLowerCase().includes(needle)),
  );

  return (
    <div className="mt-5">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h2 className="text-[15px] font-semibold text-white">本书出场角色台账</h2>
          <p className="mt-1 text-[11.5px] text-white/35">
            为全书人物绑定对应的声线模型，设定语速与情感张力基调；改绑仅影响后续合成。
          </p>
        </div>
        <button
          type="button"
          onClick={openCloneModal}
          className="flex items-center gap-1.5 rounded-full bg-accent px-3.5 py-2 text-[12.5px] font-medium text-white transition-colors hover:bg-accent-600"
        >
          <Mic className="h-3.5 w-3.5" />
          <span>克隆新音色加入本书</span>
        </button>
      </div>

      <div className="mt-5 grid grid-cols-1 gap-5 md:grid-cols-2">
        {visible.map((member) => (
          <CastCard key={member.id} member={member} />
        ))}
      </div>
    </div>
  );
}

function CastCard({ member }: { member: CastMember }) {
  const audio = useRef<AudioContext | null>(null);
  const { show } = useToast();

  // 程序化发一段特征音，模拟该声线的试听
  function audition() {
    if (!audio.current) audio.current = new AudioContext();
    const ctx = audio.current;
    if (ctx.state === "suspended") void ctx.resume();

    const osc = ctx.createOscillator();
    const gain = ctx.createGain();
    osc.frequency.setValueAtTime(member.hz, ctx.currentTime);
    gain.gain.setValueAtTime(0.2, ctx.currentTime);
    gain.gain.linearRampToValueAtTime(0.01, ctx.currentTime + 0.6);
    osc.connect(gain);
    gain.connect(ctx.destination);
    osc.start();
    osc.stop(ctx.currentTime + 0.6);

    show(`正在播放「${member.name}」特征声线试听音 (${member.hz}Hz)`, "info");
  }

  return (
    <article
      className={`flex flex-col justify-between rounded-2xl border bg-ink-800 p-5 transition-colors hover:border-white/16 ${
        member.tone === "accent" ? "border-accent/25" : "border-hairline"
      }`}
    >
      <div>
        <div className="flex items-start justify-between gap-3">
          <div className="flex min-w-0 gap-3.5">
            <div
              className={`flex h-12 w-12 shrink-0 items-center justify-center rounded-xl border text-[13px] font-bold ${avatarTone[member.tone]}`}
            >
              {member.initial}
            </div>
            <div className="min-w-0">
              <div className="flex items-center gap-1.5">
                <h3 className="truncate text-[13.5px] font-semibold text-white">{member.name}</h3>
                <span
                  className={`shrink-0 rounded border px-1.5 py-0.5 text-[10px] ${roleBadgeTone[member.tone]}`}
                >
                  {member.role}
                </span>
              </div>
              <div className="mt-1 text-[11.5px] text-white/40">{member.trait}</div>
            </div>
          </div>
          <button
            type="button"
            onClick={audition}
            title="试听"
            className={`shrink-0 rounded-lg p-2 transition-colors ${auditionTone[member.tone]}`}
          >
            <Volume2 className="h-4 w-4" />
          </button>
        </div>

        <div className="mt-4 space-y-1.5 rounded-xl border border-hairline bg-ink-850/70 p-3 text-[11.5px]">
          {member.facts.map((fact) => (
            <div key={fact.label} className="flex justify-between gap-2">
              <span className="shrink-0 text-white/35">{fact.label}:</span>
              <span
                className={`truncate ${fact.mono ? "font-mono" : ""} ${
                  fact.emphasis === "dim"
                    ? "text-white/50"
                    : fact.emphasis === "tone"
                      ? factTone[member.tone]
                      : "text-white/75"
                }`}
              >
                {fact.value}
              </span>
            </div>
          ))}
        </div>
      </div>

      <div className="mt-4 flex items-center justify-between gap-2 border-t border-hairline pt-3 text-[12px]">
        {member.footerNote ? (
          <span className="truncate font-mono text-[10.5px] text-accent-400/80">
            {member.footerNote}
          </span>
        ) : (
          <span
            className={`flex items-center gap-1.5 ${
              member.state === "已就绪" ? "text-emerald-300/80" : "text-amber-300"
            }`}
          >
            <span
              className={`h-1.5 w-1.5 rounded-full ${
                member.state === "已就绪" ? "bg-emerald-400" : "bg-amber-400"
              }`}
            />
            {member.state === "已就绪" ? "已就绪" : "待确认音色"}
          </span>
        )}

        <div className="flex shrink-0 items-center gap-3 whitespace-nowrap">
          {member.links.map((link) =>
            link.href ? (
              <Link
                key={link.label}
                href={link.href}
                className="text-white/45 transition-colors hover:text-white"
              >
                {link.label}
              </Link>
            ) : (
              <button
                key={link.label}
                type="button"
                onClick={() => show(link.toast ?? "", "success")}
                className="text-accent-400 hover:underline"
              >
                {link.label}
              </button>
            ),
          )}
        </div>
      </div>
    </article>
  );
}
