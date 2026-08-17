import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";

/**
 * Shape returned by the Rust `get_app_info` command (mirrors
 * `src-tauri/src/commands/misc.rs::AppInfo`).
 */
export interface AppInfo {
  /** Semantic version, e.g. "2.0.0". */
  version: string;
  /** Raw build number from the repo-root `BUILD` file. */
  build: string;
  /** Pre-formatted `version+build.N` (or `version+dev`) for display. */
  full: string;
  /** True when compiled without `--release`. */
  isDev: boolean;
  /** 构建渠道：stable（正式版）/ beta（测试版）。 */
  channel: string;
}

// Module-level singletons so every component shares the same fetch/cache.
const appInfo = ref<AppInfo | null>(null);
const loadError = ref<string | null>(null);
const loading = ref(false);

async function load(): Promise<void> {
  if (loading.value || appInfo.value) return;
  loading.value = true;
  loadError.value = null;
  try {
    appInfo.value = await invoke<AppInfo>("get_app_info");
  } catch (e) {
    loadError.value = String(e);
    // Provide a graceful fallback so the UI never breaks even if the
    // command isn't registered yet (e.g. during early dev / HMR races).
    appInfo.value = {
      version: "0.0.0",
      build: "?",
      full: "0.0.0+?",
      isDev: true,
      channel: "stable",
    };
  } finally {
    loading.value = false;
  }
}

/**
 * Lightweight composable exposing the app's runtime version + build
 * number. The first call to `load()` (or any consumer) fetches from
 * Rust; subsequent calls hit the in-memory cache.
 */
export function useAppInfo() {
  // Kick off the fetch eagerly; components can also await `load()`.
  void load();

  const full = computed(() => appInfo.value?.full ?? "...");
  const version = computed(() => appInfo.value?.version ?? "0.0.0");
  const build = computed(() => appInfo.value?.build ?? "?");
  const isDev = computed(() => appInfo.value?.isDev ?? true);
  const channel = computed(() => appInfo.value?.channel ?? "stable");
  const isBeta = computed(() => channel.value === "beta");
  const error = computed(() => loadError.value);

  return { appInfo, full, version, build, isDev, channel, isBeta, error, load };
}