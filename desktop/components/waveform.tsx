const BARS = 42;
const VIEWPORT_WIDTH = 320;
const VIEWPORT_HEIGHT = 120;

/** 与原型一致的线性同余伪随机，保证服务端渲染与客户端水合波形相同 */
function randomSeries(seed: number) {
  let state = seed;
  return () => {
    state = (state * 9301 + 49297) % 233280;
    return state / 233280;
  };
}

export function Waveform({
  progress,
  seed,
  className,
}: {
  progress: number;
  seed: number;
  className?: string;
}) {
  const gap = VIEWPORT_WIDTH / BARS;
  const barWidth = gap * 0.34;
  const filled = Math.round((BARS * progress) / 100);
  const random = randomSeries(seed);

  const bars = Array.from({ length: BARS }, (_, index) => {
    const height = 12 + random() * 48;
    return {
      index,
      x: index * gap + (gap - barWidth) / 2,
      y: (VIEWPORT_HEIGHT / 2 - height / 2).toFixed(1),
      height: height.toFixed(1),
      played: index < filled,
    };
  });

  return (
    <svg
      viewBox={`0 0 ${VIEWPORT_WIDTH} ${VIEWPORT_HEIGHT}`}
      preserveAspectRatio="none"
      aria-hidden="true"
      className={className}
    >
      {bars.map((bar) => (
        <rect
          key={bar.index}
          x={bar.x.toFixed(1)}
          y={bar.y}
          width={barWidth.toFixed(1)}
          height={bar.height}
          rx={(barWidth / 2).toFixed(1)}
          className={bar.played ? "fill-accent/60" : "fill-white/13"}
        />
      ))}
    </svg>
  );
}
