import { redirect } from "next/navigation";

// 根路径必然阻塞（直接跳转），显式退出 instant 校验
export const instant = false;

// 占位根路径：真正的首屏后续由产品侧自行实现，届时直接改写本文件。
// 剧目库列表页固定挂在 /project 下，不要搬回根路径（路由约定见 README）。
export default function Home() {
  redirect("/project");
}
