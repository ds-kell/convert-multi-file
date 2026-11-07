import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Settings follow the Tauri v2 + Vite guide:
// https://v2.tauri.app/start/frontend/vite/
const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [react()],
  // Keep Rust errors visible in the terminal.
  clearScreen: false,
  server: {
    // Tauri expects a fixed port; fail instead of picking another one.
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    // The Rust side is rebuilt by `cargo tauri dev`, not by Vite.
    watch: { ignored: ["**/src-tauri/**"] },
  },
  build: {
    // WebView2 on Windows is evergreen Chromium.
    target: "chrome120",
    outDir: "dist",
    emptyOutDir: true,
  },
});
