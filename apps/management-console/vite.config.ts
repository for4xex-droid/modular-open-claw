/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import wasm from "vite-plugin-wasm";
import topLevelAwait from "vite-plugin-top-level-await";
import { resolve } from "path";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async ({ mode }) => ({
  plugins: [react(), wasm(), topLevelAwait()],
  assetsInclude: ['**/*.vrm', '**/*.wasm'],
  // Production: strip console.log / console.warn (keep console.error for diagnostics)
  esbuild: {
    pure: mode === 'production' ? ['console.log', 'console.warn'] : [],
  },
  build: {
    rollupOptions: {
      input: {
        main: resolve(__dirname, 'index.html'),
        biomePopup: resolve(__dirname, 'biome-popup.html'),
      },
      output: {
        manualChunks: {
          vendor: ['react', 'react-dom'],
          ui: ['framer-motion', 'lucide-react'],
          network: ['vis-data', 'vis-network'],
          mermaid: ['beautiful-mermaid'],
          biome: ['biome-engine']
        }
      }
    }
  },

  // WASM パッケージを Vite の依存関係最適化から除外
  optimizeDeps: {
    exclude: ['biome-engine'],
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
        protocol: "ws",
        host,
        port: 1421,
      }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
    fs: {
      // biome-engine パッケージが ../../libs/ にあるため、
      // ワークスペースルートまでファイル配信を許可する
      allow: [
        resolve(__dirname, '../..'),
      ],
    },
  },
}));
