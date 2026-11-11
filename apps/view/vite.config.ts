import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Feste Portnummer, passend zu devUrl in src-tauri/tauri.conf.json
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] },
  },
});
