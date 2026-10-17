import { reactive } from "vue";
import { invoke, isTauri as tauriIsAvailable } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

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

/**
 * 失败分类。**必须与 Rust 侧 `UpdateErrorKind` 的 serde camelCase 字面量一致** ——
 * 后端 `error_serializes_with_camel_case_kind` 那条单测就是钉这个契约的。
 *
 * 分类在前端几乎全部用于「选一句人话」；唯一的**分支**用途是
 * `skippableSha()`（见下）：只有「拿不到校验文件」才允许跳过校验。
 */
export type UpdateErrorKind =
  | "network"
  | "timeout"
  | "http"
  | "disk"
  | "shaMissing"
  | "shaUnreachable"
  | "shaMismatch"
  | "asset"
  | "blocked"
  | "busy"
  | "cancelled"
  | "install"
  | "other";

export interface UpdateError {
  kind: UpdateErrorKind;
  /** 原始技术细节（英文错误链 / HTTP 状态码 / 期望与实际哈希）。故意不翻译。 */
  detail: string;
}

/**
 * 把 invoke 的 rejection 归一成 UpdateError。
 *
 * Tauri 的 Err 会以「结构化对象」穿过 IPC（我们后端返回的是 Serialize 结构体），
 * 但历史调用点、浏览器里没有 Tauri 时的 JS 异常、以及第三方抛的 Error 都可能是
 * 字符串或 Error —— 三种形态都必须有归宿，否则界面上会冒出 `[object Object]`。
 */
export function toUpdateError(e: unknown): UpdateError {
  if (e && typeof e === "object") {
    const anyErr = e as { kind?: unknown; detail?: unknown; message?: unknown };
    if (typeof anyErr.kind === "string") {
      return {
        kind: anyErr.kind as UpdateErrorKind,
        detail: typeof anyErr.detail === "string" ? anyErr.detail : "",
      };
    }
    if (typeof anyErr.message === "string") return { kind: "other", detail: anyErr.message };
  }
  if (typeof e === "string") return { kind: "other", detail: e };
  return { kind: "other", detail: String(e ?? "") };
}

/**
 * 「这次失败能否用跳过校验绕过」—— 前端这份必须与后端
 * `UpdateErrorKind::allows_unverified_sha256` **逐条对齐**。
 *
 * 为什么两边各写一份而不是让后端给个布尔：后端给布尔等于把判断权交给不可信
 * 的一侧（这本来就是一条安全边界，宁可多写四行也不接受「后端说能跳就跳」
 * 这种形状）。两份实现由测试与注释互相钉住。
 */
export function skippableSha(kind: UpdateErrorKind): boolean {
  return kind === "shaMissing" || kind === "shaUnreachable";
}

// ── 下载会话（进程级单例）─────────────────────────────────────
//
// 为什么不放在页面组件的 `ref` 里：
// 下载是**进程级**动作，页面的挂载/卸载不该影响它。之前弹窗版把进度、
// 监听器、错误全部挂在组件上，关掉弹窗再打开就「忘了」还有下载在跑 ——
// 用户点第二次时后端已经在下第一份，界面上却是从 0% 开始，两路还写同一个
// 临时文件（后端现在也有互斥，但前端不该主动制造这种局面）。
//
// 于是会话状态提到模块作用域：离开更新页再回来，进度条接着走。
export interface DownloadSession {
  /** 有一次下载正在进行（含请求还没回来的阶段） */
  running: boolean;
  /** downloading：正在收字节；verifying：字节收完了，等后端校验返回 */
  phase: "idle" | "downloading" | "verifying";
  downloaded: number;
  total: number;
  /** 平滑后的字节/秒；0 表示还测不出来 */
  rate: number;
  /** 下载起点（Date.now），用于「已用时间」与剩余时间 */
  startedAt: number;
  /** 本次是否跳过了 SHA-256（仅用于界面措辞与日志对账，不持久化） */
  skippedSha: boolean;
  /** 用户已点取消，等后端在中途停下 */
  cancelRequested: boolean;
  /** 下载完成后安装包的绝对路径 */
  path: string;
  error: UpdateError | null;
}

const session = reactive<DownloadSession>({
  running: false,
  phase: "idle",
  downloaded: 0,
  total: 0,
  rate: 0,
  startedAt: 0,
  skippedSha: false,
  cancelRequested: false,
  path: "",
  error: null,
});

let unlistenProgress: UnlistenFn | null = null;
let listenInFlight: Promise<void> | null = null;
/** 上一次采样点（不进 reactive：每 512KB 一次的事件不需要驱动渲染） */
let lastBytes = 0;
let lastAt = 0;

/**
 * 进度事件。后端每 ~512KB 发一次（`update-download-progress`），
 * 刻意不在每个 chunk 发 —— 那是每秒上千次 IPC，主线程会被自己的进度条噎住。
 */
function applyProgress(downloaded: number, total: number) {
  session.downloaded = downloaded;
  if (total > 0) session.total = total;

  const now = performance.now();
  if (lastAt === 0) {
    lastBytes = downloaded;
    lastAt = now;
  } else {
    const dt = (now - lastAt) / 1000;
    // 采样窗口太短时 dt 抖动会把速度算成天文数字，等够 0.4s 再更新
    if (dt >= 0.4) {
      const instant = (downloaded - lastBytes) / dt;
      // 指数平滑：进度事件的间隔本身不均匀，瞬时值直接显示会跳得没法看
      session.rate = session.rate > 0 ? session.rate * 0.62 + instant * 0.38 : instant;
      lastBytes = downloaded;
      lastAt = now;
    }
  }

  // 只有「这次真的要比对哈希」才切到校验阶段。
  //
  // 踩过的坑：进度到 100% 就无条件写 verifying，跳过校验的那一次也会切过去 ——
  // 屏幕上一半写「正在校验安装包 / 通过后才允许安装」，另一半写「本次已跳过
  // 校验」，两句话互相否定；更糟的是用户会以为文件被验过。后端在跳过分支里
  // **根本不做比对**，所以这个阶段必须由「有没有 expected hash」决定，而不是
  // 由字节数决定。
  if (
    !session.skippedSha &&
    total > 0 &&
    downloaded >= total &&
    session.phase === "downloading"
  ) {
    session.phase = "verifying";
  }
}

async function ensureProgressListener() {
  if (!tauriIsAvailable() || unlistenProgress) return;
  if (listenInFlight) return listenInFlight;
  listenInFlight = (async () => {
    try {
      unlistenProgress = await listen<{ downloaded: number; total: number }>(
        "update-download-progress",
        (event) => applyProgress(event.payload.downloaded, event.payload.total),
      );
    } catch {
      // 订阅失败不该让下载失败：进度条只是反馈，后端仍在正常收字节
      unlistenProgress = null;
    }
  })();
  return listenInFlight;
}

/** 重试前复位：把上一轮的数字与错误清掉，否则新的一轮会从 47% 开始 */
export function resetDownloadSession() {
  if (session.running) return;
  session.phase = "idle";
  session.downloaded = 0;
  session.total = 0;
  session.rate = 0;
  session.startedAt = 0;
  session.skippedSha = false;
  session.cancelRequested = false;
  session.path = "";
  session.error = null;
  lastBytes = 0;
  lastAt = 0;
}

/**
 * 开始下载。返回是否成功（失败时错误在 `session.error` 里）。
 *
 * 不抛异常是刻意的：这是「按钮 → 状态机」的调用，页面只需要知道成没成；
 * 失败原因要就地渲染在页面里，而不是被 try/catch 吃掉。
 */
async function beginDownload(
  tagName: string,
  opts: { allowUnverifiedSha256?: boolean } = {},
): Promise<boolean> {
  // 并发下载的**第一道**闸（第二道在后端 AtomicBool）：这里挡住最常见的情形，
  // 后端的互斥挡住「页面切走后又点一次」这类绕过前端的路径。
  if (session.running) return false;

  const skip = opts.allowUnverifiedSha256 === true;
  session.running = true;
  session.phase = "downloading";
  session.downloaded = 0;
  session.total = 0;
  session.rate = 0;
  session.startedAt = Date.now();
  session.skippedSha = skip;
  session.cancelRequested = false;
  session.path = "";
  session.error = null;
  lastBytes = 0;
  lastAt = 0;

  try {
    await ensureProgressListener();
    session.path = await invoke<string>("download_update", {
      tagName,
      allowUnverifiedSha256: skip,
    });
    session.phase = "idle";
    return true;
  } catch (e) {
    session.error = toUpdateError(e);
    session.phase = "idle";
    return false;
  } finally {
    session.running = false;
  }
}

/**
 * 请求取消。只是举手，真正的停止发生在后端下一个数据块处 ——
 * 所以这里不把 `running` 置 false：那个标志由 beginDownload 的 finally 管。
 */
async function cancelDownload(): Promise<void> {
  if (!session.running || session.cancelRequested) return;
  session.cancelRequested = true;
  try {
    await invoke<boolean>("cancel_update_download");
  } catch {
    // 取消请求本身失败不该变成一条红色错误：单块超时后下载会自己继续或失败
  }
}

export function useUpdater() {
  async function getChannel(): Promise<string> {
    try {
      return await invoke<string>("get_update_channel");
    } catch {
      return "master";
    }
  }

  async function checkForUpdate(channel?: string): Promise<UpdateCheckResult> {
    return await invoke<UpdateCheckResult>("check_for_update", {
      channel: channel ?? null,
    });
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
    download: session,
    beginDownload,
    cancelDownload,
    resetDownloadSession,
    getChannel,
    checkForUpdate,
    checkStartupUpdate,
    installUpdate,
    checkUpdateMarker,
  };
}
