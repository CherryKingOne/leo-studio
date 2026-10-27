let ctx: AudioContext | null = null;

/** 剧本音频尚未接后端，用一段特征音模拟本段试听 */
export function playAudition(hz: number) {
  if (!ctx) ctx = new AudioContext();
  if (ctx.state === "suspended") void ctx.resume();

  const osc = ctx.createOscillator();
  const gain = ctx.createGain();
  osc.frequency.setValueAtTime(hz, ctx.currentTime);
  gain.gain.setValueAtTime(0.18, ctx.currentTime);
  gain.gain.linearRampToValueAtTime(0.01, ctx.currentTime + 1.2);
  osc.connect(gain);
  gain.connect(ctx.destination);
  osc.start();
  osc.stop(ctx.currentTime + 1.2);
}
