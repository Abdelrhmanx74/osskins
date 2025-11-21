/** @type {import('next').NextConfig} */
const nextConfig = {
  reactStrictMode: true,
  output: "export",
  images: {
    unoptimized: true,
  },
  distDir: "dist",
  webpack: (config, { buildId, dev, isServer, defaultLoaders, webpack }) => {
    // Disable parallel webpack builds to reduce memory usage
    config.parallelism = 1;
    return config;
  },
};

export default nextConfig;
