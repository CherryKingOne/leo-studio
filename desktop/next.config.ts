import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  cacheComponents: true,
  partialPrefetching: true,
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
