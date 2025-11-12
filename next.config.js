/** @type {import('next').NextConfig} */
const nextConfig = {
  webpack: (config, { buildId, dev, isServer, defaultLoaders, webpack }) => {
    // Disable parallel webpack builds to reduce memory usage
    config.parallelism = 1;
    return config;
  },
};

export default nextConfig;
