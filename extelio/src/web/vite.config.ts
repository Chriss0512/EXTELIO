import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Kapitel 12: Die Auslieferung muss ohne Inline-Skripte und Inline-Styles
// auskommen, damit die Content Security Policy ohne 'unsafe-inline' greift.
export default defineConfig({
  plugins: [react()],
  build: {
    outDir: "dist",
    emptyOutDir: true,
    target: "es2022",
    cssCodeSplit: false,
    sourcemap: false,
    rollupOptions: {
      output: {
        entryFileNames: "assets/[name]-[hash].js",
        chunkFileNames: "assets/[name]-[hash].js",
        assetFileNames: "assets/[name]-[hash][extname]",
      },
    },
  },
  server: {
    port: 5173,
    proxy: { "/api": "http://127.0.0.1:8080" },
  },
});
