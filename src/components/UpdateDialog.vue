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
  // Safe 标签优先；其次按 major.minor.patch 升高位标注更新程度
  if (latest.value.priority === "safe") return t('update.prioritySafe');
  const bump = props.updateResult?.bumpKind;
  if (bump === "major") return t('update.bumpMajor');
  if (bump === "minor") return t('update.bumpMinor');
  if (bump === "patch") return t('update.bumpPatch');
  return t('update.priorityOptional');
});

const priorityClass = computed(() => {
  if (!latest.value) return "";
  const bump = props.updateResult?.bumpKind;
  if (latest.value.priority === "safe") return "priority-safe";
  if (bump === "major") return "priority-major";
  if (bump === "minor") return "priority-minor";
  if (bump === "patch") return "priority-patch";
  return `priority-${latest.value.priority}`;
});

const bumpKindLabel = computed(() => {
  const bump = props.updateResult?.bumpKind;
  if (bump === "major") return t('update.bumpMajor');
  if (bump === "minor") return t('update.bumpMinor');
  if (bump === "patch") return t('update.bumpPatch');
  return "";
});

const bumpHint = computed(() => {
  const bump = props.updateResult?.bumpKind;
  if (bump === "major") return t('update.bumpMajorHint');
  if (bump === "minor") return t('update.bumpMinorHint');
  if (bump === "patch") return t('update.bumpPatchHint');
  return "";
});

/// 强制更新：Safe-tag 或 major 升位（后端已把 major 的 priority 提升为 safe）
const isRequired = computed(() => latest.value?.priority === "safe");

const isDismissable = computed(() => !isRequired.value);
const isSkippable = computed(() => latest.value?.priority === "optional" && !isRequired.value);

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
          <p class="ud-sub">{{ t('update.checking') }}</p>
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
          <div class="ud-actions dialog-footer">
            <button class="btn-text primary" @click="emit('close')">{{ t('update.ok') }}</button>
          </div>
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
          <div v-if="bumpKindLabel" class="ud-bump-line">
            <span class="ud-bump-kind" :class="priorityClass">{{ bumpKindLabel }}</span>
            <span v-if="bumpHint" class="ud-bump-hint">{{ bumpHint }}</span>
          </div>
          <div class="ud-meta">
            <span class="ud-size"><i class="ri-download-line"></i> {{ assetSize }}</span>
            <!-- 有更新日志时日期上移到日志标题行，同一条信息不出现两次 -->
            <span v-if="latest?.publishedAt && !latest?.body" class="ud-date">{{ latest?.publishedAt?.slice(0, 10) }}</span>
          </div>
          <div v-if="latest?.body" class="ud-notes">
            <div class="ud-notes-head">
              <span class="ud-notes-title">{{ latest?.name || latest?.version }}</span>
              <span v-if="latest?.publishedAt" class="ud-notes-date">{{ latest?.publishedAt?.slice(0, 10) }}</span>
            </div>
            <div class="ud-notes-scroll">
              <div class="ud-md" v-html="renderedBody" @click="onMdClick"></div>
            </div>
          </div>
          <div class="ud-actions dialog-footer">
            <button v-if="isSkippable" class="btn-text" @click="emit('close')">{{ t('update.skipVersion') }}</button>
            <button v-if="isDismissable" class="btn-text" @click="emit('close')">{{ t('update.remindLater') }}</button>
            <button class="btn-text primary" @click="startDownload">{{ t('update.updateNow') }}</button>
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
          <div class="ud-actions dialog-footer">
            <button class="btn-text" @click="emit('close')">{{ t('update.cancel') }}</button>
          </div>
        </div>
      </template>

      <!-- Ready：打开安装向导 -->
      <template v-else-if="state === 'ready'">
        <div class="ud-state ready">
          <div class="ud-icon-box success">
            <i class="ri-check-line"></i>
          </div>
          <h3>{{ t('update.readyTitle') }}</h3>
          <p class="ud-sub">{{ t('update.readyDesc') }}</p>
          <p class="ud-warn">{{ t('update.readyWarn') }}</p>
          <div class="ud-actions dialog-footer">
            <button class="btn-text" @click="emit('close')">{{ t('update.installLater') }}</button>
            <button class="btn-text primary" @click="doInstall">{{ t('update.installRestart') }}</button>
          </div>
        </div>
      </template>

      <!-- Installing：启动安装向导 -->
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
          <div class="ud-actions dialog-footer">
            <button class="btn-text" @click="emit('close')">{{ t('common.close') }}</button>
            <button class="btn-text primary" @click="retry">{{ t('update.retry') }}</button>
          </div>
        </div>
      </template>
    </div>
  </div>
</template>

<style scoped>
/* 弹窗外壳（.dialog-overlay / .dialog-content）全项目只在 shared.css 有一份 ——
   页面级的定义会因 scoped 的 data-v 属性拿到更高特异度，留着就会盖掉全局那份，
   所以本文件不再定义它们。原来跟着它俩一起写的入场动画
   （@keyframes overlay-fade / panel-rise）也一并删掉了：没有 animation 再引用它们。
   同理，本文件不再覆盖外壳的内边距与宽度，那是全局约定。 */

/* ── 居中状态（检查中 / 无更新 / 下载中 / 已就绪 / 安装中 / 失败） ── */
.ud-state {
  display: flex; flex-direction: column; align-items: center; text-align: center; gap: 12px;
}
/* 状态标题与 shared.css 的 .dialog-header h3 同级：15px/600。
   层级由字号和留白建立 —— 原来这里用 18px/700，比弹窗标题还重。 */
.ud-state h3 { margin: 0; font-size: 15px; font-weight: 600; letter-spacing: -0.01em; color: var(--ink); }
.ud-sub { margin: 0; font-size: 13px; line-height: 1.6; color: var(--ink-2); }

/* 转圈：底环不能太浅，否则静止时看不出是个环（动画被减动效关掉时尤其明显） */
.ud-spinner {
  width: 36px; height: 36px;
  border: 3px solid rgba(var(--warm-rgb), 0.14);
  border-top-color: var(--ink);
  border-radius: 50%;
  animation: ud-spin 0.7s linear infinite;
}
@keyframes ud-spin { to { transform: rotate(360deg); } }

/* 状态图标：语义色 + 同色淡底。原来的薄荷绿 / 浅红冷底压在暖白弹窗上发脏。 */
.ud-icon-box {
  width: 44px; height: 44px; border-radius: 50%;
  display: flex; align-items: center; justify-content: center; font-size: 22px;
}
.ud-icon-box.success { background: rgba(5, 150, 105, 0.1); color: var(--ok); }
.ud-icon-box.danger { background: rgba(179, 38, 30, 0.08); color: var(--danger); }

/* ── 优先级标签 ──
   颜色只用来回答「必须装吗」：safe / major 是强制更新，用危险色；
   可选更新降为安静态，不再和它一样抢眼（原来两者都是主题蓝，看不出差别）。 */
.ud-priority-badge {
  display: inline-flex; align-items: center; gap: 5px;
  padding: 4px 10px; border-radius: 6px;
  border: 1px solid transparent;
  font-size: 11.5px; font-weight: 600;
}
.priority-safe, .priority-major {
  background: rgba(179, 38, 30, 0.08); color: var(--danger); border-color: rgba(179, 38, 30, 0.22);
}
.priority-minor, .priority-optional {
  background: rgba(var(--warm-rgb), 0.06); color: var(--ink-2); border-color: rgba(var(--warm-rgb), 0.14);
}
.priority-patch {
  background: rgba(5, 150, 105, 0.09); color: var(--ok); border-color: rgba(5, 150, 105, 0.2);
}

.ud-bump-line {
  display: flex; align-items: center; justify-content: center;
  gap: 8px; flex-wrap: wrap; margin-top: -4px;
}
.ud-bump-kind {
  display: inline-flex; align-items: center;
  padding: 3px 9px; border-radius: 6px; border: 1px solid transparent;
  font-size: 11.5px; font-weight: 600;
}
.ud-bump-hint { font-size: 12px; color: var(--ink-3); }

/* ── 版本对比：本次更新的主角，字号压过下面每一行 ── */
.ud-version-compare { display: flex; align-items: center; gap: 10px; }
.v-old { font-size: 15px; color: var(--ink-4); }
.v-arrow { font-size: 16px; color: var(--ink-4); }
.v-new {
  display: inline-flex; align-items: center; gap: 8px;
  font-size: 18px; font-weight: 600; letter-spacing: -0.02em; color: var(--ink);
}

.ud-beta-tag {
  font-style: normal; padding: 2px 7px; border-radius: 6px;
  font-size: 10.5px; font-weight: 600;
  background: rgba(180, 83, 9, 0.1); color: var(--warn);
  vertical-align: 2px;
}

.ud-meta { display: flex; gap: 14px; font-size: 12px; color: var(--ink-4); }
.ud-meta i { font-size: 13px; }

/* ── 更新日志 ──
   暖色淡面 + 发丝描边，不用投影（它不是浮层）；
   标题行固定、只有正文滚动 —— 长日志滚到一半也知道自己在读哪个版本。 */
.ud-notes {
  align-self: stretch; display: flex; flex-direction: column;
  margin-top: 2px; overflow: hidden; text-align: left;
  background: rgba(var(--warm-rgb), 0.06);
  border: 1px solid rgba(var(--warm-rgb), 0.14);
  border-radius: 8px;
}
.ud-notes-head {
  flex-shrink: 0; display: flex; align-items: baseline; justify-content: space-between; gap: 10px;
  padding: 9px 14px; border-bottom: 1px solid rgba(var(--warm-rgb), 0.14);
}
.ud-notes-title { font-size: 13px; font-weight: 600; color: var(--ink); }
.ud-notes-date { font-size: 11.5px; color: var(--ink-4); font-variant-numeric: tabular-nums; }
.ud-notes-scroll { max-height: 200px; overflow-y: auto; padding: 2px 14px 10px; }
.ud-notes-scroll::-webkit-scrollbar { width: 6px; }
.ud-notes-scroll::-webkit-scrollbar-thumb { background: rgba(var(--warm-rgb), 0.2); border-radius: 6px; }

/* Markdown 渲染视图。
   ⚠ .ud-md 内部节点来自 v-html，**不带 data-v 属性** ——
   普通后代选择器会被编译成 `.ud-md h2[data-v-x]` 而永远匹配不上
   （改造前 15 条日志样式全是死规则，日志因此是一堵浏览器默认样式的墙，必须走 :deep()）。 */
.ud-md { font-size: 12.5px; line-height: 1.7; color: var(--ink-2); }
/* 分组标题：整条发丝线 + 上方留白，把日志切成可扫读的几段 */
.ud-md :deep(h1), .ud-md :deep(h2), .ud-md :deep(h3), .ud-md :deep(h4) {
  margin: 14px 0 6px; padding-top: 12px;
  border-top: 1px solid rgba(var(--warm-rgb), 0.14);
  font-size: 13px; font-weight: 600; line-height: 1.4; letter-spacing: 0.01em;
  color: var(--ink);
}
/* 首个分组紧贴面板顶部，不该再顶一条横线 */
.ud-md :deep(h1:first-child), .ud-md :deep(h2:first-child),
.ud-md :deep(h3:first-child), .ud-md :deep(h4:first-child) {
  margin-top: 8px; padding-top: 0; border-top: none;
}
.ud-md :deep(h1) { font-size: 14px; }
.ud-md :deep(p) { margin: 6px 0; }
.ud-md :deep(ul), .ud-md :deep(ol) { margin: 6px 0; padding-left: 18px; }
.ud-md :deep(li) { margin: 4px 0; }
/* 列表符号退到三级灰：条目文字才是要读的东西 */
.ud-md :deep(li)::marker { color: var(--ink-4); }
.ud-md :deep(strong) { color: var(--ink); font-weight: 600; }
.ud-md :deep(a) {
  color: var(--ink); font-weight: 500; text-decoration: none;
  border-bottom: 1px solid rgba(var(--warm-rgb), 0.3);
}
.ud-md :deep(a:hover) { border-bottom-color: var(--ink); }
.ud-md :deep(code) {
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 11.5px; padding: 1px 5px; border-radius: 6px;
  background: rgba(var(--warm-rgb), 0.09); color: var(--ink);
}
.ud-md :deep(pre) {
  margin: 8px 0; padding: 10px 12px; border-radius: 8px; overflow-x: auto;
  background: rgba(var(--warm-rgb), 0.06);
  border: 1px solid rgba(var(--warm-rgb), 0.14);
}
.ud-md :deep(pre code) { padding: 0; background: transparent; color: var(--ink); }
.ud-md :deep(blockquote) {
  margin: 8px 0; padding: 6px 12px;
  border-left: 2px solid rgba(var(--warm-rgb), 0.3);
  border-radius: 0 8px 8px 0;
  background: rgba(var(--warm-rgb), 0.06);
  color: var(--ink-3);
}
.ud-md :deep(hr) { margin: 12px 0; border: none; border-top: 1px solid rgba(var(--warm-rgb), 0.14); }

/* ── 下载进度 ── */
.ud-progress-bar {
  width: 100%; height: 6px; border-radius: 6px; overflow: hidden;
  background: rgba(var(--warm-rgb), 0.14);
}
.ud-progress-fill { height: 100%; border-radius: 6px; background: var(--ink); transition: width 0.2s ease; }
/* 百分比每跳一次整行宽度就变一次；表格数字等宽后不再抖 */
.ud-progress-text { margin: 0; font-size: 12px; color: var(--ink-3); font-variant-numeric: tabular-nums; }

.ud-warn { margin: 0; font-size: 12px; color: var(--warn); }

/* ── 动作行 ──
   直接挂全局 .dialog-footer（发丝线 + 上边距 + 靠右）与 .btn-text 系列，不再自造一套按钮；
   只覆盖对齐：状态内容整体居中，按钮单独靠右会和正文脱节。 */
.ud-actions { align-self: stretch; justify-content: center; gap: 8px; }

@media (prefers-reduced-motion: reduce) {
  .ud-spinner { animation: none; }
  .ud-progress-fill { transition: none; }
}
</style>
