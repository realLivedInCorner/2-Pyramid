<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from "vue";
import { useI18n } from "vue-i18n";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useUpdater, type UpdateCheckResult, type ReleaseInfo } from "../composables/useUpdater";
import { isTauri as tauriIsAvailable } from "@tauri-apps/api/core";
import { renderMarkdown } from "../utils/markdown";

const { t } = useI18n();

const props = defineProps<{
  updateResult: UpdateCheckResult;
}>();

const emit = defineEmits<{
  close: [];
  installed: [];
}>();

const { downloadUpdate, installUpdate } = useUpdater();

type DialogState = "checking" | "no-update" | "found" | "downloading" | "ready" | "installing" | "error";

const state = ref<DialogState>("checking");
const downloadProgress = ref(0);
const downloadedSize = ref("0");
const totalSize = ref("0");
const downloadedPath = ref("");
const errorMessage = ref("");
let unlistenProgress: UnlistenFn | null = null;

const latest = computed<ReleaseInfo | null>(() => props.updateResult?.latest ?? null);

/// 更新日志：Release body 以 Markdown 渲染（安全渲染器）
const renderedBody = computed(() =>
  latest.value?.body ? renderMarkdown(latest.value.body) : ""
);

/// 更新日志里的链接：交给系统默认浏览器打开（仅 http/https，渲染器已校验）
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

/// Beta- / UnStable- 前缀的 release 属于测试版更新
const isBetaRelease = computed(() => {
  const tag = latest.value?.tagName?.toLowerCase() ?? "";
  return tag.startsWith("beta-") || tag.startsWith("unstable-");
});

const priorityLabel = computed(() => {
  if (!latest.value) return "";
  return latest.value.priority === "safe" ? t('update.prioritySafe') : t('update.priorityOptional');
});

const priorityClass = computed(() => {
  if (!latest.value) return "";
  return `priority-${latest.value.priority}`;
});

const isDismissable = computed(() => latest.value?.priority !== "safe");
const isSkippable = computed(() => latest.value?.priority === "optional");

const assetSize = computed(() => {
  if (!latest.value) return t('update.unknown');
  const asset = latest.value.assets.find((a) => a.name.endsWith(".msi") || a.name.endsWith(".exe"));
  if (!asset) return t('update.unknown');
  const mb = asset.size / (1024 * 1024);
  return mb >= 1 ? `${mb.toFixed(1)} MB` : `${(asset.size / 1024).toFixed(0)} KB`;
});

function formatBytes(bytes: number): string {
  const mb = bytes / (1024 * 1024);
  return mb >= 1 ? `${mb.toFixed(1)} MB` : `${(bytes / 1024).toFixed(0)} KB`;
}

async function startDownload() {
  if (!latest.value) return;
  state.value = "downloading";
  downloadProgress.value = 0;

  try {
    // Listen for progress events
    if (tauriIsAvailable()) {
      unlistenProgress = await listen<{ downloaded: number; total: number }>(
        "update-download-progress",
        (event) => {
          const { downloaded, total } = event.payload;
          if (total > 0) {
            downloadProgress.value = Math.round((downloaded / total) * 100);
          }
          downloadedSize.value = formatBytes(downloaded);
          totalSize.value = formatBytes(total);
        }
      );
    }

    downloadedPath.value = await downloadUpdate(latest.value.tagName);
    state.value = "ready";
  } catch (e: any) {
    errorMessage.value = typeof e === "string" ? e : (e?.message ?? t('update.downloadFailed'));
    state.value = "error";
  }
}

async function doInstall() {
  if (!downloadedPath.value) return;
  state.value = "installing";
  try {
    await installUpdate(downloadedPath.value, latest.value?.version ?? "");
  } catch (e: any) {
    errorMessage.value = typeof e === "string" ? e : (e?.message ?? t('update.installFailed'));
    state.value = "error";
  }
}

function retry() {
  state.value = "found";
  errorMessage.value = "";
}

onMounted(() => {
  if (props.updateResult) {
    state.value = "found";
  } else {
    state.value = "no-update";
  }
});

onUnmounted(() => {
  if (unlistenProgress) unlistenProgress();
});
</script>

<template>
  <div class="dialog-overlay" @click.self="isDismissable && emit('close')">
    <div class="dialog-content update-dialog">
      <!-- Checking -->
      <template v-if="state === 'checking'">
        <div class="ud-state checking">
          <div class="ud-spinner"></div>
          <p>{{ t('update.checking') }}</p>
        </div>
      </template>

      <!-- No update -->
      <template v-else-if="state === 'no-update'">
        <div class="ud-state no-update">
          <div class="ud-icon-box success">
            <i class="ri-check-line"></i>
          </div>
          <h3>{{ t('update.noUpdate') }}</h3>
          <p class="ud-sub">2-Pyramid {{ updateResult?.currentVersion ?? "" }}</p>
          <button class="ud-btn primary" @click="emit('close')">{{ t('update.ok') }}</button>
        </div>
      </template>

      <!-- Update found -->
      <template v-else-if="state === 'found'">
        <div class="ud-state found">
          <div class="ud-header">
            <div class="ud-priority-badge" :class="priorityClass">
              <i v-if="latest?.priority === 'safe'" class="ri-shield-check-line"></i>
              <i v-else class="ri-information-line"></i>
              {{ priorityLabel }}
            </div>
          </div>
          <div class="ud-version-compare">
            <span class="v-old">{{ updateResult?.currentVersion }}</span>
            <i class="ri-arrow-right-line v-arrow"></i>
            <span class="v-new">{{ latest?.version }}<em v-if="isBetaRelease" class="ud-beta-tag">{{ t('update.betaTag') }}</em></span>
          </div>
          <div class="ud-meta">
            <span class="ud-size"><i class="ri-download-line"></i> {{ assetSize }}</span>
            <span v-if="latest?.publishedAt" class="ud-date">{{ latest?.publishedAt?.slice(0, 10) }}</span>
          </div>
          <div v-if="latest?.body" class="ud-body">
            <div class="ud-md" v-html="renderedBody" @click="onMdClick"></div>
          </div>
          <div class="ud-actions">
            <button v-if="isSkippable" class="ud-btn ghost" @click="emit('close')">{{ t('update.skipVersion') }}</button>
            <button v-if="isDismissable" class="ud-btn ghost" @click="emit('close')">{{ t('update.remindLater') }}</button>
            <button class="ud-btn primary" @click="startDownload">{{ t('update.updateNow') }}</button>
          </div>
        </div>
      </template>

      <!-- Downloading -->
      <template v-else-if="state === 'downloading'">
        <div class="ud-state downloading">
          <div class="ud-spinner"></div>
          <h3>{{ t('update.downloading') }}</h3>
          <div class="ud-progress-bar">
            <div class="ud-progress-fill" :style="{ width: downloadProgress + '%' }"></div>
          </div>
          <p class="ud-progress-text">
            {{ downloadProgress }}%
            <span v-if="downloadedSize">· {{ downloadedSize }} / {{ totalSize }}</span>
          </p>
          <button class="ud-btn ghost" @click="emit('close')">{{ t('update.cancel') }}</button>
        </div>
      </template>

      <!-- Ready -->
      <template v-else-if="state === 'ready'">
        <div class="ud-state ready">
          <div class="ud-icon-box success">
            <i class="ri-check-line"></i>
          </div>
          <h3>{{ t('update.readyTitle') }}</h3>
          <p class="ud-sub">{{ t('update.readyDesc') }}</p>
          <p class="ud-warn">{{ t('update.readyWarn') }}</p>
          <div class="ud-actions">
            <button class="ud-btn ghost" @click="emit('close')">{{ t('update.installLater') }}</button>
            <button class="ud-btn primary" @click="doInstall">{{ t('update.installRestart') }}</button>
          </div>
        </div>
      </template>

      <!-- Installing -->
      <template v-else-if="state === 'installing'">
        <div class="ud-state installing">
          <div class="ud-spinner"></div>
          <h3>{{ t('update.installing') }}</h3>
          <p class="ud-sub">{{ t('update.installingDesc') }}</p>
        </div>
      </template>

      <!-- Error -->
      <template v-else-if="state === 'error'">
        <div class="ud-state error">
          <div class="ud-icon-box danger">
            <i class="ri-close-line"></i>
          </div>
          <h3>{{ t('update.failed') }}</h3>
          <p class="ud-sub">{{ errorMessage }}</p>
          <div class="ud-actions">
            <button class="ud-btn ghost" @click="emit('close')">{{ t('common.close') }}</button>
            <button class="ud-btn primary" @click="retry">{{ t('update.retry') }}</button>
          </div>
        </div>
      </template>
    </div>
  </div>
</template>

<style scoped>
.dialog-overlay {
  position: fixed; inset: 0; z-index: 2000;
  background: rgba(15, 23, 42, 0.35);
  display: flex; align-items: center; justify-content: center;
  backdrop-filter: blur(8px);
  animation: overlay-fade 0.2s ease;
}
.dialog-content {
  background: #fff; border-radius: 20px;
  border: 1px solid rgba(0, 0, 0, 0.06);
  box-shadow: 0 24px 60px rgba(0, 0, 0, 0.18);
  padding: 0; width: 440px; max-width: 92vw;
  animation: panel-rise 0.25s ease;
  overflow: hidden;
}
@keyframes overlay-fade { from { opacity: 0; } to { opacity: 1; } }
@keyframes panel-rise { from { opacity: 0; transform: translateY(12px) scale(0.98); } to { opacity: 1; transform: translateY(0) scale(1); } }

.update-dialog {
  padding: 36px 32px 28px;
}
.ud-state {
  display: flex; flex-direction: column; align-items: center; text-align: center; gap: 12px;
}
.ud-state h3 { font-size: 18px; font-weight: 700; color: #1d1d1f; margin: 0; }
.ud-sub { font-size: 13px; color: #6b7280; margin: 0; }

/* Spinner */
.ud-spinner {
  width: 40px; height: 40px;
  border: 3px solid #e9ecef;
  border-top-color: var(--theme-color);
  border-radius: 50%;
  animation: ud-spin 0.7s linear infinite;
}
@keyframes ud-spin { to { transform: rotate(360deg); } }

/* Icon box */
.ud-icon-box {
  width: 56px; height: 56px; border-radius: 50%;
  display: flex; align-items: center; justify-content: center; font-size: 28px;
}
.ud-icon-box.success { background: #ecfdf5; color: #10b981; }
.ud-icon-box.danger { background: #fef2f2; color: #ef4444; }

/* Priority badge */
.ud-priority-badge {
  display: inline-flex; align-items: center; gap: 6px;
  padding: 5px 14px; border-radius: 20px;
  font-size: 12px; font-weight: 700; letter-spacing: 0.3px;
}
.priority-safe { background: #fef2f2; color: #dc2626; border: 1px solid #fecaca; }
.priority-optional { background: color-mix(in srgb, var(--theme-color) 10%, transparent); color: var(--theme-color); border: 1px solid color-mix(in srgb, var(--theme-color) 20%, transparent); }

/* Version compare */
.ud-version-compare {
  display: flex; align-items: center; gap: 14px;
  font-size: 20px; font-weight: 800;
}
.v-old { color: #9ca3af; }
.v-arrow { color: #9ca3af; font-size: 18px; }
.v-new { color: #1d1d1f; display: inline-flex; align-items: center; gap: 8px; }

.ud-beta-tag {
  font-style: normal;
  font-size: 11px;
  font-weight: 800;
  padding: 2px 9px;
  border-radius: 999px;
  background: rgba(249, 115, 22, 0.14);
  color: #ea580c;
  vertical-align: 2px;
}

/* Meta */
.ud-meta {
  display: flex; gap: 16px; font-size: 12px; color: #9ca3af;
}
.ud-meta i { font-size: 14px; }

/* Body (changelog) —— Markdown 渲染视图 */
.ud-body {
  width: 100%; max-height: 240px; overflow-y: auto;
  background: #f9fafb; border-radius: 10px; padding: 14px 16px;
  text-align: left; margin-top: 4px;
}

.ud-md {
  font-size: 12.5px; color: #4b5563; line-height: 1.7;
}
.ud-md h1, .ud-md h2, .ud-md h3, .ud-md h4 {
  margin: 10px 0 6px; color: #1f2937; font-weight: 800; line-height: 1.4;
}
.ud-md h1 { font-size: 15px; }
.ud-md h2 { font-size: 14px; }
.ud-md h3, .ud-md h4 { font-size: 13px; }
.ud-md p { margin: 6px 0; }
.ud-md ul, .ud-md ol { margin: 6px 0 6px 18px; padding: 0; }
.ud-md li { margin: 3px 0; }
.ud-md blockquote {
  margin: 8px 0; padding: 6px 12px;
  border-left: 3px solid color-mix(in srgb, var(--theme-color) 55%, #e5e7eb);
  background: #f3f4f6; border-radius: 0 8px 8px 0;
  color: #6b7280;
}
.ud-md code {
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 11.5px;
  background: #eef0f3; border-radius: 4px; padding: 1px 5px;
  color: #111827;
}
.ud-md pre {
  margin: 8px 0; padding: 10px 12px;
  background: #111827; border-radius: 8px; overflow-x: auto;
}
.ud-md pre code { background: transparent; color: #e5e7eb; padding: 0; font-size: 11.5px; }
.ud-md a {
  color: var(--theme-color); text-decoration: none; font-weight: 700;
  border-bottom: 1px dashed color-mix(in srgb, var(--theme-color) 50%, transparent);
}
.ud-md a:hover { border-bottom-style: solid; }
.ud-md hr {
  border: none; border-top: 1px solid #e5e7eb; margin: 10px 0;
}
.ud-md strong { color: #111827; }

/* Progress bar */
.ud-progress-bar {
  width: 100%; height: 6px; background: #e9ecef; border-radius: 3px; overflow: hidden;
}
.ud-progress-fill {
  height: 100%; background: var(--theme-color); border-radius: 3px;
  transition: width 0.2s ease;
}
.ud-progress-text { font-size: 12px; color: #6b7280; margin: 0; }

/* Warning */
.ud-warn {
  font-size: 12px; color: #f59e0b; font-weight: 600;
}

/* Actions */
.ud-actions {
  display: flex; gap: 10px; margin-top: 8px;
}
.ud-btn {
  padding: 10px 22px; border-radius: 10px; border: none;
  font-size: 14px; font-weight: 600; cursor: pointer;
  transition: all 0.2s;
  font-family: inherit;
}
.ud-btn.primary {
  background: var(--theme-color); color: #fff;
}
.ud-btn.primary:hover { opacity: 0.88; transform: translateY(-1px); }
.ud-btn.ghost {
  background: transparent; color: #6b7280;
}
.ud-btn.ghost:hover { background: #f3f4f6; color: #1d1d1f; }
</style>
