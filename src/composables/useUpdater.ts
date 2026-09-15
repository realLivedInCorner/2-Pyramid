import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

export interface ReleaseInfo {
  tagName: string;
  version: string;
  name: string;
  body: string;
  publishedAt: string;
  htmlUrl: string;
  assets: Array<{ name: string; browserDownloadUrl: string; size: number; contentType: string }>;
  priority: "safe" | "optional";
  isPrerelease: boolean;
}

export type VersionBumpKind = "none" | "patch" | "minor" | "major";

export interface UpdateCheckResult {
  hasUpdate: boolean;
  currentVersion: string;
  /** major.minor.patch 升高位：none | patch | minor | major */
  bumpKind: VersionBumpKind;
  latest: ReleaseInfo | null;
}

export function useUpdater() {
  const updateAvailable = ref(false);
  const updateInfo = ref<UpdateCheckResult | null>(null);
  const isChecking = ref(false);

  async function getChannel(): Promise<string> {
    try {
      return await invoke<string>("get_update_channel");
    } catch {
      return "master";
    }
  }

  async function checkForUpdate(channel?: string): Promise<UpdateCheckResult> {
    isChecking.value = true;
    try {
      const result = await invoke<UpdateCheckResult>("check_for_update", {
        channel: channel ?? null,
      });
      updateAvailable.value = result.hasUpdate;
      updateInfo.value = result;
      return result;
    } finally {
      isChecking.value = false;
    }
  }

  async function checkStartupUpdate(): Promise<{
    hasUpdate: boolean;
    /** "safe" = 强制（Safe-tag 或 major）；"optional" = 用户自愿 */
    priority: "safe" | "optional";
    result: UpdateCheckResult | null;
  }> {
    try {
      const channel = await getChannel();
      const result = await checkForUpdate(channel);
      // 后端已按 bump 覆盖 priority：Safe-tag / major → safe，其余 optional
      const priority = result.latest?.priority ?? "optional";
      return {
        hasUpdate: result.hasUpdate,
        priority,
        result: result.hasUpdate ? result : null,
      };
    } catch {
      return { hasUpdate: false, priority: "optional", result: null };
    }
  }

  async function downloadUpdate(tagName: string): Promise<string> {
    return await invoke<string>("download_update", { tagName });
  }

  async function installUpdate(installerPath: string, newVersion: string): Promise<void> {
    await invoke("install_update", { installerPath, newVersion });
  }

  async function checkUpdateMarker(): Promise<string | null> {
    try {
      return await invoke<string | null>("check_update_marker");
    } catch {
      return null;
    }
  }

  return {
    updateAvailable,
    updateInfo,
    isChecking,
    getChannel,
    checkForUpdate,
    checkStartupUpdate,
    downloadUpdate,
    installUpdate,
    checkUpdateMarker,
  };
}
