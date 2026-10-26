import { redirect } from "next/navigation";

// 占位根路径：真正的首屏后续由产品侧自行实现，届时直接改写本文件。
// 剧目库列表页固定挂在 /project 下，不要搬回根路径（路由约定见 README）。
export default function Home() {
  redirect("/project");
}
