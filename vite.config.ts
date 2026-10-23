import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import path from "node:path";
import process from "node:process";

const host = process.env.TAURI_DEV_HOST;

export default defineConfig(() => ({
  plugins: [react(), tailwindcss()],

  resolve: {
    alias: { "@": path.resolve(import.meta.dirname, "./src") },
  },

  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    // Принудительно IPv4 loopback — лучше всего работает с VPN
    host: "127.0.0.1",
    hmr: {
      protocol: "ws",
      host: "127.0.0.1",
      port: 1421,
    },
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
}));