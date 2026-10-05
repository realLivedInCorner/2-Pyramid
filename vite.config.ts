import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import { fileURLToPath } from "url";
import { dirname, resolve } from "path";
import { existsSync } from "fs";

// `import.meta.url` gives us the config file's location at runtime,
// which lets us derive `__dirname` without depending on the Node
// global (which TS needs `@types/node` for). Works under both
// `vite.config.ts` (loaded via tsx/esbuild at dev time) and the
// compiled `vite.config.js` emitted by `tsc -b`.
const __dirname = dirname(fileURLToPath(import.meta.url));

const host = process.env.TAURI_DEV_HOST;

// 依赖目录：向上找到真正装着 `node_modules` 的那一层。
//
// 为什么需要它：`git worktree` 的 `node_modules` 是**空的**（只有 `.vite` 缓存），
// 所有依赖（vite / vue / remixicon …）由 Node 沿目录链**向上**解析到主 checkout。
// 而 Vite 默认把 `server.fs.allow` 限制在「项目根 + vite 自带的 client 目录」，
// 于是 worktree 里跑 dev 时字体落在允许列表之外：
//
//   The request url ".../code/node_modules/remixicon/fonts/remixicon.woff2"
//   is outside of Vite serving allow list.
//
// 后果不是报错而是**静默降级**：字体 404 ⇒ 图标渲染成方框（看起来像"图标库没了"）。
//
// 注意**不能写死 ".."**：worktree 的深度不固定（`code/.worktrees/astray` 的依赖在
// 上两层）。这里按 Node 自己的规则向上找第一个含 `node_modules` 的目录——
// 主 checkout 里它就在项目根内，找到了也只是重复放行一次。
function depsRoots(from: string): string[] {
  const found: string[] = [];
  let dir = resolve(from);
  for (let i = 0; i < 6; i += 1) {
    const parent = resolve(dir, "..");
    if (parent === dir) break;
    dir = parent;
    if (existsSync(resolve(dir, "node_modules"))) found.push(dir);
  }
  return found;
}
const depsRoot = depsRoots(__dirname);

// 构建渠道：stable（正式版）/ beta（测试版）。
// 发布流水线通过环境变量 VITE_CHANNEL 注入；这里补齐默认值，
// 保证 index.html 中的 %VITE_BETA_MARK% 占位符（以及任何读取
// import.meta.env 的代码）在 dev / 手动构建时也能被替换。
if (!process.env.VITE_CHANNEL) {
  process.env.VITE_CHANNEL = "stable";
}
process.env.VITE_BETA_MARK =
  process.env.VITE_CHANNEL === "beta" ? " (Beta)" : "";

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
    fs: {
      // 见上方 `depsRoot`：worktree 的依赖在上层，必须显式放行，否则图标字体 404。
      // 同时放行项目根（`fs.allow` 覆盖默认值，不会自动包含根目录）。
      allow: [resolve(__dirname), ...depsRoot],
    },
  },
}));
