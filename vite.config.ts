import { defineConfig } from "vite";

export default defineConfig({
  // Vite options
  root: "src",
  clearScreen: false,
  build: {
    outDir: "../dist",
    emptyOutDir: true,
  },
  // Tauri dev server
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
});
