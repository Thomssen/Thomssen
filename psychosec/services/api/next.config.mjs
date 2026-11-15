/** @type {import('next').NextConfig} */
export default {
  transpilePackages: ["@psychosec/shared"],
  poweredByHeader: false,
  productionBrowserSourceMaps: false,
  // API only: no pages are served.
};
