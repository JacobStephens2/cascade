import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import wasm from "vite-plugin-wasm";
import topLevelAwait from "vite-plugin-top-level-await";
import { VitePWA } from "vite-plugin-pwa";

export default defineConfig({
  plugins: [
    react(),
    wasm(),
    topLevelAwait(),
    VitePWA({
      registerType: "autoUpdate",
      includeAssets: ["sounds/waterfall.ogg"],
      manifest: {
        name: "Cascade",
        short_name: "Cascade",
        description: "Waterfall white-noise player for focus, prayer, and sleep.",
        theme_color: "#0b1a24",
        background_color: "#0b1a24",
        display: "standalone",
        orientation: "portrait",
        icons: [
          {
            src: "/icon-192.png",
            sizes: "192x192",
            type: "image/png",
          },
          {
            src: "/icon-512.png",
            sizes: "512x512",
            type: "image/png",
          },
          {
            src: "/icon-512.png",
            sizes: "512x512",
            type: "image/png",
            purpose: "maskable",
          },
        ],
      },
      workbox: {
        // The waterfall audio file is ~9MB OGG; precache it so the PWA works
        // offline. Bump the max file size accordingly.
        maximumFileSizeToCacheInBytes: 20 * 1024 * 1024,
        globPatterns: ["**/*.{js,css,html,wasm,svg,png,ico,ogg}"],
      },
    }),
  ],
  server: {
    host: true,
    port: 5173,
  },
  build: {
    target: "es2022",
  },
});
