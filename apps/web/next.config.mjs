/** @type {import('next').NextConfig} */
const nextConfig = {
  reactStrictMode: true,
  poweredByHeader: false,
  // Fully static build so the web app can be served from a free static host that never sleeps.
  output: "export",
  trailingSlash: true,
  images: { unoptimized: true },
};
export default nextConfig;
