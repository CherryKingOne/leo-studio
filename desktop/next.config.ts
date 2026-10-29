import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  cacheComponents: true,
  partialPrefetching: true,
  // 新建剧目走服务端动作上传小说正文，默认 1MB 会把几 MB 的 txt 挡在门外，
  // 这里与后端 LEO_MAX_UPLOAD_SIZE_MB=50 对齐并留出 multipart 边界开销
  experimental: {
    serverActions: {
      bodySizeLimit: "52mb",
    },
  },
  // 用局域网 IP 访问 dev server 时必须放行该来源，否则 /_next 开发资源被跨域拦截，页面不会水合、点击无响应
  allowedDevOrigins: ["172.16.70.104"],
  turbopack: {
    rules: {
      "*.css": {
        loaders: ["@tailwindcss/turbopack"],
        as: "*.css",
      },
    },
  },
};

export default nextConfig;
