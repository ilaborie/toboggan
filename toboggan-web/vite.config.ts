import { defineConfig } from "vite";

export default defineConfig({
  root: ".",
  build: {
    outDir: "dist",
    rollupOptions: {
      input: {
        main: "./index.html",
      },
    },
  },
  server: {
    port: 8000,
    proxy: {
      "/assets": "http://localhost:8080",
    },
  },
});
