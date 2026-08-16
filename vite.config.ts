import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import { fileURLToPath } from "url";
import { dirname, resolve } from "path";

// `import.meta.url` gives us the config file's location at runtime,
// which lets us derive `__dirname` without depending on the Node
// global (which TS needs `@types/node` for). Works under both
// `vite.config.ts` (loaded via tsx/esbuild at dev time) and the
// compiled `vite.config.js` emitted by `tsc -b`.
const __dirname = dirname(fileURLToPath(import.meta.url));

const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [vue()],

  // Multi-page setup: the main app lives at `index.html` and the
  // desktop top-level toast window lives at `toast.html`. Each page
  // has its own entry script so we can keep them tiny and independent
  // — the toast window in particular only needs a handful of KB.
  build: {
    rollupOptions: {
      input: {
        main: resolve(__dirname, "index.html"),
        toast: resolve(__dirname, "toast.html"),
      },
    },
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1421,
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
  },
}));
