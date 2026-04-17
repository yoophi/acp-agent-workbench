import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

const devServerPort = Number.parseInt(process.env.VITE_DEV_SERVER_PORT ?? "1420", 10);

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: Number.isInteger(devServerPort) ? devServerPort : 1420,
    strictPort: true,
  },
  envPrefix: ["VITE_", "TAURI_"],
  build: {
    target: "es2020",
    minify: !process.env.TAURI_DEBUG,
    sourcemap: !!process.env.TAURI_DEBUG,
  },
});
