<script setup lang="ts">
/**
 * 更新页 —— 全屏页面，不再是弹窗。
 *
 * ── 为什么是独立页而不是「设置页里的一栏」 ──
 * ① 更新器有 8 个状态（检查中 / 已最新 / 检查失败 / 有新版本 / 下载中 /
 *    已就绪 / 安装中 / 失败+跳过入口），其中 4 个需要**独占**的主操作区；
 *    塞进设置页的一栏，用户得先滚到那一栏才看得见自己刚点的「立即更新」，
 *    而下载进度是那种「必须一直看得到」的东西。
 * ② 它有三个入口（启动检查 / 设置页 / 更新提示条），并且**要求**「不再是
 *    dialog」；作为页面，它天然享受 App.vue 的整页转场与返回栈，
 *    而弹窗每次都要自己处理拦截点击、ESC、层级。
 * ③ 与设置页共用同一块地面（StageBackdrop）与同一套返回钮，切过来像
 *    「走进了另一屏」，不是「弹出了一个框」。
 *
 * ── 排版约定（本文件刻意遵守的项目规范）──
 * · 颜色只取自 shared.css 的令牌；层级靠字号 / 字距 / 留白，字重上限 600。
 * · 中文标题**不加负字距**（满 em 方块字会被挤笔画）；负字距只出现在
 *   版本号这类纯拉丁数字上。
 * · 分隔用「行 + 发丝线」，不用玻璃卡片堆叠、不用 backdrop-filter。
 * · 动效全部交给 App.vue 的整页 stagger 与全局 `body.motion-reduced`，
 *   本文件不自行判断动效偏好（唯一例外是转圈，见 .up-spinner 的注释）。
 */
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { openUrl } from "@tauri-apps/plugin-opener";
import StageBackdrop from "./shell/StageBackdrop.vue";
import { useAppInfo } from "../composables/useAppInfo";
import {
  useUpdater,
  toUpdateError,
  skippableSha,
  type UpdateCheckResult,
  type UpdateError,
} from "../composables/useUpdater";
import { renderMarkdown } from "../utils/markdown";

const { t, te } = useI18n();
const { version: appVersion } = useAppInfo();

const props = withDefaults(
  defineProps<{
    /**
     * 调用方已经拿到的检查结果（启动检查 / 设置页）。
     * 给了就直接进结果态，不再多打一次网络请求；没给就自己查。
     */
    updateResult?: UpdateCheckResult | null;
  }>(),
  { updateResult: null },
);

const emit = defineEmits<{ "switch-page": [page: string] }>();

const {
  download: session,
  beginDownload,
  cancelDownload,
  resetDownloadSession,
  getChannel,
  checkForUpdate,
  installUpdate,
} = useUpdater();

/**
 * 一个 phase 覆盖全部界面状态。
 *
 * 「下载中 / 校验中」不拆成两个 phase：校验是后端在同一次 invoke 里做的，
 * 前端只能从进度事件推断「字节收完了」。拆两个状态就得在两个地方同步它，
 * 而它们共用同一条进度条与同一个取消按钮 —— 用一个 phase + 一个派生的
 * `isVerifying` 更不容易错。
 */
type Phase =
  | "checking"
  | "latest"
  | "checkFailed"
  | "found"
  | "downloading"
  | "ready"
  | "installing"
  | "failed";

const phase = ref<Phase>("checking");
const result = ref<UpdateCheckResult | null>(null);
const checkError = ref<UpdateError | null>(null);
const flowError = ref<UpdateError | null>(null);
const failedStage = ref<"check" | "download" | "install">("download");
const checkedAt = ref<string>("");
const channel = ref("master");
const showDetail = ref(false);
const showSkipConfirm = ref(false);
const copied = ref(false);

const latest = computed(() => result.value?.latest ?? null);
const currentVersion = computed(
  () => result.value?.currentVersion || appVersion.value || "—",
);
/** 下载阶段：字节收完但 invoke 还没返回 —— 后端此刻在校验哈希 */
const isVerifying = computed(
  () => phase.value === "downloading" && session.phase === "verifying",
);

const channelLabel = computed(() => {
  if (channel.value === "unstable") return t("update.channelUnstable");
  if (channel.value === "both") return t("update.channelBoth");
  return t("update.channelStable");
});
const versionTag = computed(() => `v${currentVersion.value}`);

/// Beta- / UnStable- 前缀的 release 属于测试版更新
const isBetaRelease = computed(() => {
  const tag = latest.value?.tagName?.toLowerCase() ?? "";
  return tag.startsWith("beta-") || tag.startsWith("unstable-");
});

/// 强制更新：Safe-tag 或 major 升位（后端已把 major 的 priority 提升为 safe）
const isRequired = computed(() => latest.value?.priority === "safe");
const isDismissable = computed(() => !isRequired.value);

/**
 * 强制更新在「有新版本 / 安装中」两个阶段不给退路。
 *
 * 这是改造前弹窗的既有语义（`isDismissable = false` 时连关闭按钮都没有）。
 * 换成一整页之后，如果顶上还留着一个「返回」，强制更新就名存实亡 ——
 * 点一下就走了，而 Safe-tag / major 的整点正是「这次必须装」。
 *
 * 失败态**仍然给返回**：强制约束的是「有可用更新时不许跳过」，
 * 不是「连更新都失败了也不许离开」。把用户锁在一张报错页上没有意义。
 */
const hideBack = computed(
  () => isRequired.value && (phase.value === "found" || phase.value === "installing"),
);

const priorityLabel = computed(() =>
  isRequired.value ? t("update.prioritySafe") : t("update.priorityOptional"),
);

const bumpKindLabel = computed(() => {
  const bump = result.value?.bumpKind;
  if (bump === "major") return t("update.bumpMajor");
  if (bump === "minor") return t("update.bumpMinor");
  if (bump === "patch") return t("update.bumpPatch");
  return "";
});

const bumpHint = computed(() => {
  const bump = result.value?.bumpKind;
  if (bump === "major") return t("update.bumpMajorHint");
  if (bump === "minor") return t("update.bumpMinorHint");
  if (bump === "patch") return t("update.bumpPatchHint");
  return "";
});

/// 更新说明：Release body 以 Markdown 渲染（安全渲染器，见 utils/markdown.ts）
const renderedBody = computed(() =>
  latest.value?.body ? renderMarkdown(latest.value.body) : "",
);
const publishedDate = computed(() => latest.value?.publishedAt?.slice(0, 10) ?? "");

const assetSize = computed(() => {
  // 只统计安装包本体（.sha256 只有几十字节，算进去会显得「更新很小」）
  const asset = latest.value?.assets.find(
    (a) => a.name.endsWith(".exe") || a.name.endsWith(".msi"),
  );
  return asset ? formatBytes(asset.size) : t("update.unknown");
});

// ── 数字格式化 ───────────────────────────────────────────────

function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return "0 KB";
  const mb = bytes / (1024 * 1024);
  if (mb >= 1) return `${mb.toFixed(1)} MB`;
  return `${Math.max(1, Math.round(bytes / 1024))} KB`;
}

/** 剩余时间用 mm:ss —— 与语言无关，也不会因为「秒/分」的单位词而变长 */
function formatEta(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds <= 0) return "0:00";
  const total = Math.round(seconds);
  const m = Math.floor(total / 60);
  const s = total % 60;
  return `${m}:${String(s).padStart(2, "0")}`;
}

function nowClock(): string {
  const d = new Date();
  return [d.getHours(), d.getMinutes(), d.getSeconds()]
    .map((n) => String(n).padStart(2, "0"))
    .join(":");
}

const percent = computed(() => {
  if (session.total <= 0) return 0;
  return Math.min(100, Math.round((session.downloaded / session.total) * 100));
});
const speedText = computed(() =>
  session.rate > 0 ? `${formatBytes(session.rate)}/s` : "—",
);
const etaText = computed(() => {
  if (session.rate <= 0 || session.total <= 0) return "";
  return formatEta(Math.max(0, (session.total - session.downloaded) / session.rate));
});

// ── 失败文案 ─────────────────────────────────────────────────
//
// 后端的 `detail` 是**故意不翻译**的技术原文（HTTP 码、IO 报错、期望/实际哈希），
// 它出现在「技术细节」里供复制排查；界面上这句话必须是用户能判断「该不该
// 重试」的人话，所以按 kind 从 i18n 取，未知 kind 落到 other 的通用措辞。
const activeError = computed<UpdateError | null>(() =>
  phase.value === "checkFailed" ? checkError.value : flowError.value,
);

const errorCopy = computed(() => {
  const kind = activeError.value?.kind ?? "other";
  const key = `update.errors.${kind}`;
  if (te(`${key}.title`)) {
    return { title: t(`${key}.title`), hint: t(`${key}.hint`) };
  }
  return {
    title: t("update.errors.other.title"),
    hint: t("update.errors.other.hint"),
  };
});

/**
 * 跳过校验的入口**只**在这一种失败下出现：拿不到 `.sha256`
 * （资产缺失 / 取不到）。哈希对不上不出现 —— 那是「字节不对」的证据，
 * 绕过它等于把完整性校验整个作废；正确补救是重新下载。
 * 判据与后端 `UpdateErrorKind::allows_unverified_sha256` 逐条对齐。
 */
const canSkipSha = computed(() => {
  if (failedStage.value !== "download") return false;
  const kind = flowError.value?.kind;
  return !!kind && skippableSha(kind);
});

async function copyDetail() {
  const text = activeError.value?.detail ?? "";
  if (!text) return;
  let ok = false;
  try {
    await navigator.clipboard.writeText(text);
    ok = true;
  } catch {
    // WebView 里 clipboard API 可能没有权限（非 secure context）——
    // 落回 execCommand，这条在 Tauri 的 webview 里一直是可用的
    try {
      const ta = document.createElement("textarea");
      ta.value = text;
      ta.style.position = "fixed";
      ta.style.opacity = "0";
      document.body.appendChild(ta);
      ta.select();
      ok = document.execCommand("copy");
      ta.remove();
    } catch {
      ok = false;
    }
  }
  if (ok) {
    copied.value = true;
    window.setTimeout(() => { copied.value = false; }, 1600);
  }
}

// ── 流程 ─────────────────────────────────────────────────────

async function runCheck() {
  phase.value = "checking";
  checkError.value = null;
  showSkipConfirm.value = false;
  try {
    channel.value = await getChannel();
    const r = await checkForUpdate(channel.value);
    result.value = r;
    checkedAt.value = nowClock();
    phase.value = r.hasUpdate && r.latest ? "found" : "latest";
  } catch (e) {
    checkError.value = toUpdateError(e);
    failedStage.value = "check";
    phase.value = "checkFailed";
  }
}

async function startDownload(allowUnverifiedSha256 = false) {
  if (!latest.value) return;
  showSkipConfirm.value = false;
  showDetail.value = false;
  flowError.value = null;
  failedStage.value = "download";
  phase.value = "downloading";

  const ok = await beginDownload(latest.value.tagName, { allowUnverifiedSha256 });
  if (ok) {
    phase.value = "ready";
    return;
  }
  const err = session.error;
  // 取消不是失败：回到「有新版本」，用户可以再点一次下载
  if (err?.kind === "cancelled") {
    phase.value = "found";
    return;
  }
  flowError.value = err ?? toUpdateError("unknown");
  phase.value = "failed";
}

async function doInstall() {
  phase.value = "installing";
  flowError.value = null;
  try {
    await installUpdate(
      session.path,
      latest.value?.version ?? result.value?.latest?.version ?? "",
    );
  } catch (e) {
    flowError.value = toUpdateError(e);
    failedStage.value = "install";
    phase.value = "failed";
  }
  // 正常路径下 `install_update` 会在 1.5s 后 app.exit(0)，调用根本不会返回 ——
  // 所以「安装中」就是本页最后的画面，不需要（也等不到）一个「已完成」态。
}

function retry() {
  showDetail.value = false;
  if (failedStage.value === "check") return void runCheck();
  if (failedStage.value === "install") return void doInstall();
  // 重新下载：后端在写文件前会先删掉上次的半成品，前端复位进度与错误
  resetDownloadSession();
  return void startDownload(false);
}

function goBack() {
  emit("switch-page", "home");
}

/// 更新日志里的链接交给系统浏览器（仅 http/https，渲染器已校验）
function onMdClick(ev: MouseEvent) {
  const target = ev.target as HTMLElement | null;
  const link = target?.closest?.("a[data-ext-link]") as HTMLAnchorElement | null;
  if (!link) return;
  ev.preventDefault();
  const href = link.getAttribute("href");
  if (href && /^https?:\/\//i.test(href)) {
    void openUrl(href).catch(() => {});
  }
}

function openReleasePage() {
  const url = latest.value?.htmlUrl;
  if (url && /^https?:\/\//i.test(url)) void openUrl(url).catch(() => {});
}

onMounted(async () => {
  try {
    channel.value = await getChannel();
  } catch {
    // 读不到通道就用默认值：它只影响标识，不影响能不能更新
  }

  // 从别的页切走再回来：会话状态还在，接着显示，不要退回「检查中」
  if (session.running) {
    phase.value = "downloading";
    return;
  }
  if (session.path) {
    // 上一轮已经下好了（用户选了「稍后安装」）
    phase.value = "ready";
    return;
  }
  if (session.error && session.error.kind !== "cancelled") {
    flowError.value = session.error;
    failedStage.value = "download";
    phase.value = "failed";
    return;
  }
  if (props.updateResult) {
    result.value = props.updateResult;
    checkedAt.value = nowClock();
    phase.value =
      props.updateResult.hasUpdate && props.updateResult.latest ? "found" : "latest";
    return;
  }
  await runCheck();
});
</script>

<template>
  <div class="update-page">
    <StageBackdrop />

    <header class="up-head">
      <div class="up-head-main">
        <button
          v-if="!hideBack"
          class="back-btn"
          type="button"
          :aria-label="t('common.backToHome')"
          @click="goBack"
        >
          <i class="ri-arrow-left-line back-icon" aria-hidden="true"></i>
          <span>{{ t("common.back") }}</span>
        </button>
        <div class="up-title-group">
          <h1 class="up-title">{{ t("update.pageTitle") }}</h1>
          <p class="up-subtitle">{{ t("update.pageSubtitle") }}</p>
        </div>
      </div>

      <!--
        右上只放不可点的信息（通道 / 当前版本）：窗口控制按钮是 fixed 的
        36×3 + 间隙，占住右上角 y 10–54 与右侧约 140px —— 任何按钮放进来
        都会被压在下面，点了像是没反应。可点的「重新检查」因此都在正文里。
      -->
      <div class="up-head-meta">
        <span class="up-channel">{{ channelLabel }}</span>
        <span class="up-version">{{ versionTag }}</span>
      </div>
    </header>

    <main class="up-body" aria-live="polite">
      <div class="up-column">
        <!-- :key 一变就重挂载，入场动画重播；动画本身只写 backwards 填充，
             避免动画结束后留下 translate 让本页成为包含块（Workbench 记录过这个坑） -->
        <section class="up-state" :key="phase" :data-phase="phase">

          <!-- ── 检查中 ── -->
          <template v-if="phase === 'checking'">
            <div class="up-status">
              <div class="up-spinner" aria-hidden="true"></div>
              <p class="up-status-title">{{ t("update.checking") }}</p>
              <p class="up-status-hint">{{ t("update.connecting") }}</p>
            </div>
          </template>

          <!-- ── 已是最新 ── -->
          <template v-else-if="phase === 'latest'">
            <div class="up-status">
              <div class="up-glyph ok" aria-hidden="true">
                <i class="ri-check-line"></i>
              </div>
              <p class="up-status-title">{{ t("update.noUpdate") }}</p>
              <p class="up-status-hint">
                {{ t("update.noUpdateDesc", { version: currentVersion }) }}
              </p>
              <p v-if="checkedAt" class="up-status-meta">
                {{ t("update.checkedAt", { time: checkedAt }) }}
              </p>
            </div>
          </template>

          <!-- ── 检查失败 ── -->
          <template v-else-if="phase === 'checkFailed'">
            <div class="up-status">
              <div class="up-glyph danger" aria-hidden="true">
                <i class="ri-close-line"></i>
              </div>
              <p class="up-status-title">{{ t("update.checkFailedTitle") }}</p>
              <p class="up-status-hint">{{ errorCopy.title }}</p>
              <p class="up-status-hint dim">{{ errorCopy.hint }}</p>
            </div>
          </template>

          <!-- ── 有新版本 ── -->
          <template v-else-if="phase === 'found'">
            <div class="up-chips">
              <span class="up-chip" :class="isRequired ? 'required' : 'optional'">
                <i :class="isRequired ? 'ri-shield-check-line' : 'ri-information-line'" aria-hidden="true"></i>
                {{ priorityLabel }}
              </span>
              <span v-if="bumpKindLabel" class="up-chip bump">{{ bumpKindLabel }}</span>
              <span v-if="isBetaRelease" class="up-chip beta">{{ t("update.betaTag") }}</span>
            </div>

            <!-- 版本对比：本页的主角，字号压过下面每一行 -->
            <div class="up-versions">
              <div class="up-ver">
                <span class="up-ver-label">{{ t("update.currentLabel") }}</span>
                <span class="up-ver-num">{{ currentVersion }}</span>
              </div>
              <i class="ri-arrow-right-line up-ver-arrow" aria-hidden="true"></i>
              <div class="up-ver">
                <span class="up-ver-label">{{ t("update.latestLabel") }}</span>
                <span class="up-ver-num fresh">{{ latest?.version }}</span>
              </div>
            </div>
            <p v-if="bumpHint" class="up-bump-hint">{{ bumpHint }}</p>

            <dl class="up-rows">
              <div class="up-row">
                <dt>{{ t("update.channelRow") }}</dt>
                <dd>{{ channelLabel }}</dd>
              </div>
              <div v-if="publishedDate" class="up-row">
                <dt>{{ t("update.publishedRow") }}</dt>
                <dd>{{ publishedDate }}</dd>
              </div>
              <div class="up-row">
                <dt>{{ t("update.sizeRow") }}</dt>
                <dd>{{ assetSize }}</dd>
              </div>
              <div v-if="latest?.htmlUrl" class="up-row">
                <dt>{{ t("update.releaseRow") }}</dt>
                <dd>
                  <button class="up-link" type="button" @click="openReleasePage">
                    {{ t("update.viewOnGithub") }}
                    <i class="ri-external-link-line" aria-hidden="true"></i>
                  </button>
                </dd>
              </div>
            </dl>

            <section v-if="renderedBody" class="up-notes">
              <div class="up-notes-head">
                <h2 class="up-notes-title">{{ t("update.releaseNotes") }}</h2>
                <span v-if="publishedDate" class="up-notes-date">{{ publishedDate }}</span>
              </div>
              <div class="up-notes-body">
                <div class="up-md" v-html="renderedBody" @click="onMdClick"></div>
              </div>
            </section>

            <div class="up-actions">
              <p v-if="isDismissable" class="up-hint">{{ t("update.remindLaterHint") }}</p>
              <button v-if="isDismissable" class="ghost-btn" type="button" @click="goBack">
                {{ t("update.remindLater") }}
              </button>
              <button class="primary-btn" type="button" @click="startDownload(false)">
                <i class="ri-download-2-line" aria-hidden="true"></i>
                {{ t("update.updateNow") }}
              </button>
            </div>
          </template>

          <!-- ── 下载中 / 校验中 ── -->
          <template v-else-if="phase === 'downloading'">
            <div class="up-progress">
              <div class="up-progress-head">
                <p class="up-progress-title">
                  {{ isVerifying ? t("update.verifying") : t("update.downloading") }}
                </p>
                <p class="up-progress-percent">
                  <template v-if="isVerifying">{{ t("update.verifyingShort") }}</template>
                  <template v-else>{{ session.total > 0 ? percent + "%" : "" }}</template>
                </p>
              </div>

              <div class="up-bar" :class="{ indeterminate: session.total <= 0 && !isVerifying }">
                <div
                  class="up-bar-fill"
                  :class="{ verifying: isVerifying }"
                  :style="{ width: session.total > 0 ? percent + '%' : '30%' }"
                ></div>
              </div>

              <div v-if="!isVerifying" class="up-progress-meta">
                <span class="up-mono">{{ formatBytes(session.downloaded) }} / {{ formatBytes(session.total) }}</span>
                <i aria-hidden="true">·</i>
                <span class="up-mono">{{ speedText }}</span>
                <template v-if="etaText">
                  <i aria-hidden="true">·</i>
                  <span>{{ t("update.remaining", { time: etaText }) }}</span>
                </template>
              </div>
              <p v-else class="up-progress-meta up-verifying-note">{{ t("update.verifyingDesc") }}</p>

              <p v-if="session.skippedSha" class="up-warn-line">
                <i class="ri-error-warning-line" aria-hidden="true"></i>
                {{ t("update.skippedNotice") }}
              </p>

              <div class="up-actions">
                <p class="up-hint">{{ t("update.cancelHint") }}</p>
                <button
                  class="ghost-btn"
                  type="button"
                  :disabled="session.cancelRequested || isVerifying"
                  @click="cancelDownload"
                >
                  {{ session.cancelRequested ? t("update.cancelling") : t("update.cancel") }}
                </button>
              </div>
            </div>
          </template>

          <!-- ── 已就绪：启动安装向导 ── -->
          <template v-else-if="phase === 'ready'">
            <div class="up-status">
              <div class="up-glyph ok" aria-hidden="true">
                <i class="ri-check-line"></i>
              </div>
              <p class="up-status-title">{{ t("update.readyTitle") }}</p>
              <p class="up-status-hint">{{ t("update.readyDesc") }}</p>
              <p class="up-status-meta">
                <i v-if="!session.skippedSha" class="ri-shield-check-line" aria-hidden="true"></i>
                {{ session.skippedSha ? t("update.shaSkippedBadge") : t("update.verifiedOk") }}
              </p>
              <p class="up-warn">{{ t("update.readyWarn") }}</p>
            </div>

            <div class="up-actions">
              <button class="ghost-btn" type="button" @click="goBack">
                {{ t("update.installLater") }}
              </button>
              <button class="primary-btn" type="button" @click="doInstall">
                <i class="ri-restart-line" aria-hidden="true"></i>
                {{ t("update.installRestart") }}
              </button>
            </div>
          </template>

          <!-- ── 安装中 ── -->
          <template v-else-if="phase === 'installing'">
            <div class="up-status">
              <div class="up-spinner" aria-hidden="true"></div>
              <p class="up-status-title">{{ t("update.installing") }}</p>
              <p class="up-status-hint">{{ t("update.installingDesc") }}</p>
            </div>
          </template>

          <!-- ── 失败（下载 / 校验 / 安装）── -->
          <template v-else-if="phase === 'failed'">
            <div class="up-status tight">
              <div class="up-glyph danger" aria-hidden="true">
                <i class="ri-close-line"></i>
              </div>
              <p class="up-status-title">{{ errorCopy.title }}</p>
              <p class="up-status-hint">{{ errorCopy.hint }}</p>
            </div>

            <!-- 技术细节：默认折叠，但一定在。用户复制给维护者的就是这一段 -->
            <div class="up-detail">
              <div class="up-detail-head">
                <button class="up-detail-toggle" type="button" @click="showDetail = !showDetail">
                  <i :class="showDetail ? 'ri-arrow-down-s-line' : 'ri-arrow-right-s-line'" aria-hidden="true"></i>
                  {{ t("update.technicalDetail") }}
                </button>
                <button class="up-detail-copy" type="button" @click="copyDetail">
                  {{ copied ? t("update.copied") : t("update.copyDetail") }}
                </button>
              </div>
              <pre v-if="showDetail" class="up-detail-body">{{ activeError?.detail }}</pre>
            </div>

            <!-- 跳过校验：入口 → 代价说明 → 二次确认。两步都必须发生 -->
            <div v-if="canSkipSha" class="up-skip">
              <template v-if="!showSkipConfirm">
                <button class="up-skip-open" type="button" @click="showSkipConfirm = true">
                  {{ t("update.skipShaAction") }}
                  <i class="ri-arrow-right-s-line" aria-hidden="true"></i>
                </button>
              </template>
              <template v-else>
                <!--
                  免责声明。它必须**被读到并理解**，所以给了整块边框、左侧 3px 竖线、
                  两段正文（代价 / 责任）和比正文更大的行距。
                  但**不用红色**：红色在本项目里只表示「已经出错了」；这里还没出错，
                  是用户在主动选一条保护更少的路 —— 用红等于把一个选择渲染成事故。
                -->
                <div class="up-skip-box">
                  <p class="up-skip-eyebrow">{{ t("update.skipShaEyebrow") }}</p>
                  <p class="up-skip-title">{{ t("update.skipShaTitle") }}</p>
                  <p class="up-skip-cost">{{ t("update.skipShaCost") }}</p>
                  <p class="up-skip-responsibility">{{ t("update.skipShaResponsibility") }}</p>
                  <div class="up-skip-actions">
                    <button class="ghost-btn" type="button" @click="showSkipConfirm = false">
                      {{ t("update.skipShaCancel") }}
                    </button>
                    <button
                      class="ghost-btn up-skip-confirm"
                      type="button"
                      @click="startDownload(true)"
                    >
                      {{ t("update.skipShaConfirm") }}
                    </button>
                  </div>
                </div>
              </template>
            </div>

            <div class="up-actions">
              <button class="ghost-btn" type="button" @click="goBack">{{ t("update.backHome") }}</button>
              <button class="primary-btn" type="button" @click="retry">
                <i class="ri-refresh-line" aria-hidden="true"></i>
                {{ t("update.retry") }}
              </button>
            </div>
          </template>
        </section>

        <!-- 检查失败态在这里复用同一套「技术细节 + 重试」，
             不塞进上面的 up-status 里 —— 那里是给一句话结论的地方 -->
        <div v-if="phase === 'checkFailed'" class="up-checkfail-extra">
          <div class="up-detail">
            <div class="up-detail-head">
              <button class="up-detail-toggle" type="button" @click="showDetail = !showDetail">
                <i :class="showDetail ? 'ri-arrow-down-s-line' : 'ri-arrow-right-s-line'" aria-hidden="true"></i>
                {{ t("update.technicalDetail") }}
              </button>
              <button class="up-detail-copy" type="button" @click="copyDetail">
                {{ copied ? t("update.copied") : t("update.copyDetail") }}
              </button>
            </div>
            <pre v-if="showDetail" class="up-detail-body">{{ activeError?.detail }}</pre>
          </div>
          <div class="up-actions">
            <button class="ghost-btn" type="button" @click="goBack">{{ t("update.backHome") }}</button>
            <button class="primary-btn" type="button" @click="runCheck">
              <i class="ri-refresh-line" aria-hidden="true"></i>
              {{ t("update.checkAgain") }}
            </button>
          </div>
        </div>

        <!-- 「已是最新」也留一个重新检查；放在状态块外，取消/安装中不出现 -->
        <div v-if="phase === 'latest'" class="up-checkfail-extra">
          <div class="up-actions">
            <button class="ghost-btn" type="button" @click="goBack">{{ t("update.backHome") }}</button>
            <button class="ghost-btn" type="button" @click="runCheck">
              <i class="ri-refresh-line" aria-hidden="true"></i>
              {{ t("update.checkAgain") }}
            </button>
          </div>
        </div>
      </div>
    </main>
  </div>
</template>

<style scoped>
.update-page {
  width: 100%;
  height: 100%;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  position: relative;
  color: var(--ink);
  font-family: "Microsoft YaHei UI", "Microsoft YaHei", "PingFang SC", "Hiragino Sans GB",
    "Noto Sans CJK SC", "Source Han Sans SC", -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
}

/* ── 页头 ──
   右侧留 150px：窗口控制按钮是 fixed 的 3×36px + 间隙，压在 y 10–54 上。
   这里只放不可点的信息，所以「留白」就够了，不需要把控件顶下去。 */
.up-head {
  position: relative;
  z-index: 1;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 24px;
  padding: 30px 150px 18px 48px;
}
.up-head-main {
  display: flex;
  align-items: center;
  gap: 20px;
  min-width: 0;
}
.up-title-group {
  display: flex;
  flex-direction: column;
  min-width: 0;
}
/*
 * 中文标题**不加**负字距。
 * 负字距是拉丁大标题的技巧（"2-Pyramid" 那种字母对能挤紧一点），
 * 中文是满 em 方块字，挤了就是把笔画压到一起 —— 实测 -0.02em 时
 * 「更新」两字的「攵」和「丨」会粘成一条线。
 */
.up-title {
  margin: 0;
  font-size: clamp(22px, 3vw, 28px);
  font-weight: 600;
  line-height: 1.18;
  letter-spacing: 0;
  color: var(--ink);
}
/* 副标题走 --ink-3：深色下它在渐变底上实测 4.06–4.45，达不到正文 4.5，
   够 3:1 的「辅助文字」门槛，所以只用在这里，不用在任何整句说明上 */
.up-subtitle {
  margin: 6px 0 0;
  font-size: 13px;
  color: var(--ink-3);
  letter-spacing: 0.01em;
}
.up-head-meta {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-shrink: 0;
}
.up-channel {
  padding: 3px 9px;
  border-radius: 6px;
  border: 1px solid rgba(var(--warm-rgb), 0.2);
  color: var(--ink-2);
  font-size: 11.5px;
  letter-spacing: 0.04em;
  white-space: nowrap;
}
.up-version {
  font-size: 12px;
  color: var(--ink-3);
  font-variant-numeric: tabular-nums;
  letter-spacing: 0.02em;
}

/* ── 正文 ── */
.up-body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  position: relative;
  z-index: 1;
  display: flex;
  justify-content: center;
  padding: 10px 48px 44px;
}
.up-body::-webkit-scrollbar { width: 8px; }
.up-body::-webkit-scrollbar-track { background: transparent; }
.up-body::-webkit-scrollbar-thumb {
  background: rgba(var(--warm-rgb), 0.18);
  border-radius: 4px;
}
.up-body::-webkit-scrollbar-thumb:hover { background: rgba(var(--warm-rgb), 0.32); }

.up-column {
  width: 100%;
  max-width: 760px;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

/*
 * 状态块入场。
 * fill-mode 只能是 backwards：both / forwards 会让 `translate` 常驻，
 * 元素因此成为包含块，将来的浮层（fixed）会错位 —— Workbench.vue 记过同一个坑。
 * 基线里不写 opacity: 0：动画结束后会回落到基线，那样内容会消失。
 */
.up-state {
  display: flex;
  flex-direction: column;
  animation: up-in 320ms var(--ease-out, cubic-bezier(0.22, 1, 0.36, 1)) backwards;
}
@keyframes up-in {
  from { opacity: 0; translate: 0 10px; }
  to { opacity: 1; translate: 0 0; }
}

/* ── 居中状态（检查中 / 已最新 / 安装中）── */
.up-status {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  gap: 10px;
  padding: 56px 0 8px;
}
.up-status.tight { padding: 32px 0 4px; }

.up-status-title {
  margin: 0;
  font-size: 18px;
  font-weight: 600;
  letter-spacing: 0.01em;
  color: var(--ink);
}
.up-status-hint {
  margin: 0;
  max-width: 46ch;
  font-size: 13px;
  line-height: 1.7;
  color: var(--ink-2);
}
.up-status-hint.dim { color: var(--ink-3); }
.up-status-meta {
  margin: 4px 0 0;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: var(--ink-3);
  font-variant-numeric: tabular-nums;
}

/*
 * 状态图标。
 * 语义色只用在**底色**上，线条统一用墨色：--ok 的 #10b981 在浅色纸面上
 * 只有约 2.4:1（深色 #3fb950 约 5.4:1），拿它画一个有含义的图形过不了
 * 非文本 3:1 的门槛；底色是 12% 的淡染，不影响可读性，含义由标题文字承载。
 */
.up-glyph {
  width: 44px;
  height: 44px;
  border-radius: 50%;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  font-size: 22px;
  color: var(--ink);
}
.up-glyph.ok { background: color-mix(in srgb, var(--ok) 14%, transparent); }
.up-glyph.danger { background: color-mix(in srgb, var(--danger) 12%, transparent); color: var(--danger); }

/*
 * 转圈：底环不能太淡，否则动画被减动效关掉时看不出这是个「环」。
 * 全局的 `body.motion-reduced [class*="spin"]` 会把它保住（加载态例外），
 * 本文件不再单独写 media query —— 两处都写会互相盖掉。
 */
.up-spinner {
  width: 30px;
  height: 30px;
  border: 2.5px solid rgba(var(--warm-rgb), 0.18);
  border-top-color: var(--ink);
  border-radius: 50%;
  animation: up-spin 0.75s linear infinite;
}
@keyframes up-spin { to { transform: rotate(360deg); } }

/* ── 标签 ── */
.up-chips {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}
.up-chip {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 3px 9px;
  border-radius: 6px;
  border: 1px solid transparent;
  font-size: 11.5px;
  font-weight: 600;
  letter-spacing: 0.02em;
}
.up-chip i { font-size: 13px; line-height: 1; }
.up-chip.required {
  color: var(--danger);
  background: color-mix(in srgb, var(--danger) 9%, transparent);
  border-color: color-mix(in srgb, var(--danger) 26%, transparent);
}
.up-chip.optional {
  color: var(--ink-2);
  background: rgba(var(--warm-rgb), 0.07);
  border-color: rgba(var(--warm-rgb), 0.16);
}
.up-chip.bump {
  color: var(--ink-2);
  border-color: rgba(var(--warm-rgb), 0.2);
}
.up-chip.beta {
  color: var(--warn);
  background: var(--warn-tint);
  border-color: var(--warn-line);
}

/* ── 版本对比 ── */
.up-versions {
  display: flex;
  align-items: flex-end;
  gap: 18px;
  margin-top: 18px;
  flex-wrap: wrap;
}
.up-ver {
  display: flex;
  flex-direction: column;
  gap: 7px;
  min-width: 0;
}
.up-ver-label {
  font-size: 11.5px;
  letter-spacing: 0.1em;
  color: var(--ink-3);
}
/* 版本号是纯拉丁数字，是全文件唯一允许负字距的地方 */
.up-ver-num {
  font-size: 34px;
  font-weight: 600;
  line-height: 1;
  letter-spacing: -0.025em;
  font-variant-numeric: tabular-nums;
  color: var(--ink-3);
}
.up-ver-num.fresh { color: var(--ink); }
.up-ver-arrow {
  font-size: 20px;
  line-height: 1;
  margin-bottom: 6px;
  color: var(--ink-3);
}
.up-bump-hint {
  margin: 12px 0 0;
  font-size: 12.5px;
  line-height: 1.6;
  color: var(--ink-2);
}

/* ── 明细行：靠发丝线分隔，不做卡片 ── */
.up-rows {
  margin: 24px 0 0;
  display: flex;
  flex-direction: column;
}
.up-row {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 20px;
  padding: 10px 0;
  border-bottom: 1px solid rgba(var(--warm-rgb), 0.14);
}
.up-row:first-child { border-top: 1px solid rgba(var(--warm-rgb), 0.14); }
.up-row dt {
  font-size: 12px;
  letter-spacing: 0.03em;
  color: var(--ink-3);
  flex-shrink: 0;
}
.up-row dd {
  margin: 0;
  font-size: 13px;
  color: var(--ink);
  font-variant-numeric: tabular-nums;
  text-align: right;
}
.up-link {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 0;
  border: none;
  background: none;
  font-family: inherit;
  font-size: 13px;
  color: var(--ink);
  cursor: pointer;
  border-bottom: 1px solid rgba(var(--warm-rgb), 0.32);
  transition: border-color 0.15s ease;
}
.up-link:hover { border-bottom-color: var(--ink); }
.up-link i { font-size: 13px; }

/* ── 更新说明 ── */
.up-notes {
  margin-top: 26px;
  display: flex;
  flex-direction: column;
  min-height: 0;
}
.up-notes-head {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 12px;
  padding-bottom: 9px;
  border-bottom: 1px solid rgba(var(--warm-rgb), 0.14);
}
.up-notes-title {
  margin: 0;
  font-size: 13px;
  font-weight: 600;
  letter-spacing: 0.03em;
  color: var(--ink);
}
.up-notes-date {
  font-size: 11.5px;
  color: var(--ink-3);
  font-variant-numeric: tabular-nums;
}
/* 日志正文限高滚动：长日志不该把「立即更新」顶到看不见的地方 */
.up-notes-body {
  max-height: 236px;
  overflow-y: auto;
  padding: 10px 0 2px;
}
.up-notes-body::-webkit-scrollbar { width: 6px; }
.up-notes-body::-webkit-scrollbar-thumb {
  background: rgba(var(--warm-rgb), 0.2);
  border-radius: 6px;
}

/*
 * Markdown 渲染视图（v-html）。
 * ⚠ 内部节点来自 v-html，**不带 data-v 属性** —— 普通后代选择器会被编译成
 * `.up-md h2[data-v-x]` 而永远匹配不上，必须走 :deep()。UpdateDialog 当年
 * 在这里丢过 15 条规则（日志变成一堵浏览器默认样式的墙）。
 */
.up-md {
  font-size: 12.5px;
  line-height: 1.72;
  color: var(--ink-2);
}
.up-md :deep(h1),
.up-md :deep(h2),
.up-md :deep(h3),
.up-md :deep(h4) {
  margin: 16px 0 6px;
  padding-top: 12px;
  border-top: 1px solid rgba(var(--warm-rgb), 0.14);
  font-size: 13px;
  font-weight: 600;
  line-height: 1.4;
  letter-spacing: 0.02em;
  color: var(--ink);
}
.up-md :deep(h1:first-child),
.up-md :deep(h2:first-child),
.up-md :deep(h3:first-child),
.up-md :deep(h4:first-child) {
  margin-top: 0;
  padding-top: 0;
  border-top: none;
}
.up-md :deep(h1) { font-size: 14px; }
.up-md :deep(p) { margin: 6px 0; }
.up-md :deep(ul),
.up-md :deep(ol) { margin: 6px 0; padding-left: 18px; }
.up-md :deep(li) { margin: 4px 0; }
/* 列表符号退一级：条目文字才是要读的 */
.up-md :deep(li)::marker { color: var(--ink-3); }
.up-md :deep(strong) { color: var(--ink); font-weight: 600; }
.up-md :deep(a) {
  color: var(--ink);
  text-decoration: none;
  border-bottom: 1px solid rgba(var(--warm-rgb), 0.32);
}
.up-md :deep(a:hover) { border-bottom-color: var(--ink); }
.up-md :deep(code) {
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 11.5px;
  padding: 1px 5px;
  border-radius: 5px;
  background: rgba(var(--warm-rgb), 0.1);
  color: var(--ink);
}
.up-md :deep(pre) {
  margin: 8px 0;
  padding: 10px 12px;
  border-radius: 8px;
  overflow-x: auto;
  background: rgba(var(--warm-rgb), 0.07);
  border: 1px solid rgba(var(--warm-rgb), 0.14);
}
.up-md :deep(pre code) { padding: 0; background: transparent; color: var(--ink); }
.up-md :deep(blockquote) {
  margin: 8px 0;
  padding: 6px 12px;
  border-left: 2px solid rgba(var(--warm-rgb), 0.32);
  color: var(--ink-2);
}
.up-md :deep(hr) {
  margin: 14px 0;
  border: none;
  border-top: 1px solid rgba(var(--warm-rgb), 0.14);
}

/* ── 下载进度 ── */
.up-progress { display: flex; flex-direction: column; padding-top: 34px; }
.up-progress-head {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 16px;
}
.up-progress-title {
  margin: 0;
  font-size: 18px;
  font-weight: 600;
  letter-spacing: 0.01em;
  color: var(--ink);
}
.up-progress-percent {
  margin: 0;
  font-size: 26px;
  font-weight: 600;
  line-height: 1;
  letter-spacing: -0.02em;
  font-variant-numeric: tabular-nums;
  color: var(--ink);
}
.up-bar {
  position: relative;
  height: 4px;
  margin-top: 18px;
  border-radius: 2px;
  background: rgba(var(--warm-rgb), 0.18);
  overflow: hidden;
}
.up-bar-fill {
  height: 100%;
  border-radius: 2px;
  background: var(--ink);
  transition: width 220ms var(--ease-out, ease-out);
}
/* 校验阶段：进度已满，改用一条更淡的满条 —— 「字节收完」不等于「完成」 */
.up-bar-fill.verifying { background: var(--ink-3); }
/* 拿不到 total（无 content-length）时不知道百分比，跑一条来回扫的块 */
.up-bar.indeterminate .up-bar-fill { animation: up-sweep 1.3s var(--ease-in-out, ease-in-out) infinite; }
@keyframes up-sweep {
  from { transform: translateX(-110%); }
  to { transform: translateX(360%); }
}
.up-progress-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
  margin: 12px 0 0;
  font-size: 12px;
  color: var(--ink-3);
  font-variant-numeric: tabular-nums;
}
.up-progress-meta i { font-style: normal; color: var(--ink-3); }
.up-progress-meta.up-verifying-note { color: var(--ink-2); font-variant-numeric: normal; }
.up-mono { font-family: ui-monospace, SFMono-Regular, Consolas, monospace; font-size: 11.5px; }

.up-warn-line {
  display: flex;
  align-items: flex-start;
  gap: 6px;
  margin: 16px 0 0;
  font-size: 12px;
  line-height: 1.6;
  color: var(--warn);
}
.up-warn-line i { font-size: 14px; line-height: 1.4; }
.up-warn {
  margin: 6px 0 0;
  font-size: 12px;
  color: var(--warn);
}

/* ── 失败：技术细节 ── */
.up-detail {
  margin-top: 22px;
  border-top: 1px solid rgba(var(--warm-rgb), 0.14);
  padding-top: 10px;
}
.up-detail-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}
.up-detail-toggle,
.up-detail-copy {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 4px 0;
  border: none;
  background: none;
  font-family: inherit;
  font-size: 12px;
  color: var(--ink-3);
  cursor: pointer;
  transition: color 0.15s ease;
}
.up-detail-toggle:hover,
.up-detail-copy:hover { color: var(--ink); }
.up-detail-toggle i { font-size: 15px; }
.up-detail-body {
  margin: 8px 0 0;
  padding: 10px 12px;
  max-height: 150px;
  overflow: auto;
  border-radius: 8px;
  background: rgba(var(--warm-rgb), 0.07);
  border: 1px solid rgba(var(--warm-rgb), 0.14);
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 11.5px;
  line-height: 1.6;
  color: var(--ink-2);
  white-space: pre-wrap;
  word-break: break-all;
  user-select: text;
}

/* ── 失败：跳过校验 ──
 *
 * 整页唯一一处「用户主动降低安全等级」的地方。它的分量靠**结构**建立，
 * 不靠色相：
 *   · 整块 1px 暖色发丝框 + 左侧 3px --warn 竖线
 *     （--warn 是警示色，不是错误红；红色在本项目里只表示「已经出错了」，
 *      而这里还没出错 —— 用户在选一条保护更少的路，不该被渲染成事故现场）
 *   · 段落分开：眉标 → 标题 → 代价 → 责任，责任段再压一条发丝线
 *   · 正文 13px / 1.8 行距：页面正文是 13/1.7，这里故意再松一点，逼着读慢
 * 不用投影、不用 backdrop-filter —— 全项目用「行 + 发丝线」分隔，
 * 这里也只是页面里一个被强调的行块，不是浮起来的卡片。
 */
.up-skip-box {
  padding: 18px 20px 16px 22px;
  border: 1px solid rgba(var(--warm-rgb), 0.22);
  border-left: 3px solid var(--warn);
  border-radius: 4px;
  background: rgba(var(--warm-rgb), 0.05);
}
.up-skip-eyebrow {
  margin: 0 0 10px;
  font-size: 10.5px;
  font-weight: 400;
  letter-spacing: 0.18em;
  text-transform: uppercase;
  color: var(--ink-3);
}
.up-skip-title {
  margin: 0;
  font-size: 15px;
  font-weight: 600;
  /* 中文标题不加负字距：满 em 方块字会被挤到笔画相粘 */
  letter-spacing: 0;
  line-height: 1.35;
  color: var(--ink);
}
/* 代价段：必须读得清，所以是 --ink-2 而不是 --ink-3
   （深色下 --ink-3 在渐变底上只有 4.06–4.45，过不了正文 4.5 的门槛） */
.up-skip-cost {
  margin: 12px 0 0;
  font-size: 13px;
  line-height: 1.8;
  color: var(--ink-2);
}
/* 责任段：整段里最该被读的一句，用主墨色压过上一段。
   不再加粗 —— 本页字重上限 600，600 已经给了标题。 */
.up-skip-responsibility {
  margin: 12px 0 0;
  padding-top: 12px;
  border-top: 1px solid rgba(var(--warm-rgb), 0.16);
  font-size: 13px;
  line-height: 1.8;
  color: var(--ink);
}
.up-skip-actions {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 10px;
  margin-top: 16px;
}
/* 确认键不用红：它执行的是一个**已知后果**的选择，不是一个错误操作。
   用墨色描边让它比「不跳过」更实一档，但两者都还是安静态。 */
.up-skip-confirm {
  color: var(--ink);
  border-color: rgba(var(--warm-rgb), 0.34);
}
.up-skip-confirm:hover {
  color: var(--ink);
  background: rgba(var(--warm-rgb), 0.1);
}

/* ── 动作行 ── */
.up-actions {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 10px;
  flex-wrap: wrap;
  margin-top: 28px;
  padding-top: 16px;
  border-top: 1px solid rgba(var(--warm-rgb), 0.14);
}
.up-actions .up-hint {
  flex: 1;
  min-width: 160px;
  margin: 0;
  font-size: 12px;
  line-height: 1.5;
  color: var(--ink-3);
}
.up-checkfail-extra { display: flex; flex-direction: column; }

/* ── 窄窗 ── */
@media (max-width: 980px) {
  .up-head { padding: 26px 24px 14px; }
  .up-body { padding: 8px 24px 32px; }
}
@media (max-width: 720px) {
  .up-head {
    flex-direction: column;
    align-items: flex-start;
    gap: 12px;
    padding: 22px 16px 10px;
  }
  .up-body { padding: 6px 16px 28px; }
  .up-head-meta { padding-left: 0; }
  .up-ver-num { font-size: 28px; }
  .up-notes-body { max-height: 200px; }
  .up-actions { justify-content: stretch; }
  .up-actions > button { flex: 1; }
}
</style>
