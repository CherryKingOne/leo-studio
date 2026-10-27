/**
 * 暗色滚动条：透明轨道 + 内缩细胶囊滑块，等价于原型的 ::-webkit-scrollbar 处理。
 * 不加这段时 macOS「始终显示滚动条」会渲染 15px 浅灰经典滚动条，压在悬浮工具栏下方。
 */
export const inkScrollbar = [
  "[&::-webkit-scrollbar]:h-2.5",
  "[&::-webkit-scrollbar]:w-2.5",
  "[&::-webkit-scrollbar-track]:bg-transparent",
  "[&::-webkit-scrollbar-thumb]:rounded-full",
  "[&::-webkit-scrollbar-thumb]:border-[3px]",
  "[&::-webkit-scrollbar-thumb]:border-transparent",
  "[&::-webkit-scrollbar-thumb]:bg-ink-600",
  "[&::-webkit-scrollbar-thumb]:[background-clip:content-box]",
  "[&::-webkit-scrollbar-thumb:hover]:bg-[#3a3a42]",
].join(" ");
