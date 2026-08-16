<template>
  <div class="conversion-page">
    <div class="header-section">
      <button class="back-btn" @click="goBack" :aria-label="t('common.backToHome')">
        <i class="ri-arrow-left-line back-icon" aria-hidden="true"></i>
        <span>{{ t('common.back') }}</span>
      </button>
      <div class="title-group">
        <h1 class="page-title">{{ t('conversion.title') }}</h1>
        <p class="page-subtitle">{{ t('conversion.subtitle') }}</p>
      </div>
    </div>

    <div class="panel-grid">
      <div class="card card-mode">
        <div class="card-title">{{ t('conversion.importMode') }}</div>
        <div class="import-tabs">
          <button class="tab-btn" :class="{ active: importMode === 'file' }" @click="importMode = 'file'">{{ t('conversion.fileMode') }}</button>
          <button class="tab-btn" :class="{ active: importMode === 'folder' }" @click="importMode = 'folder'">{{ t('conversion.folderMode') }}</button>
        </div>
        <p class="card-hint">{{ t('conversion.dropHint') }}</p>
      </div>

      <div class="card card-version">
        <div class="card-title">{{ t('conversion.targetVersion') }}</div>
        <button
          class="version-picker"
          :class="{ open: showVersionPicker }"
          @click="showVersionPicker = true"
        >
          <div class="version-picker-main">
            <span class="version-picker-label">{{ selectedVersionEntry.label }}</span>
            <span class="version-picker-range">{{ selectedVersionEntry.range }}</span>
          </div>
          <div class="version-picker-meta">
            <span class="version-picker-format">{{ t('conversion.packFormat', { n: selectedVersionEntry.packFormat }) }}</span>
            <i class="ri-arrow-down-s-line version-picker-chevron" aria-hidden="true"></i>
          </div>
        </button>
        <label class="switch-line">
          <span class="switch-title">
            {{ t('conversion.fixAlphaLabel') }}
            <span
              class="help-icon"
              :title="t('conversion.fixAlphaHelp')"
              :aria-label="t('conversion.fixAlphaLabel')"
            >?</span>
          </span>
          <label class="switch">
            <input type="checkbox" v-model="fixAlphaLayers" />
            <span class="slider"></span>
          </label>
        </label>
        <p class="card-hint">{{ t('conversion.fixAlphaHint') }}</p>
      </div>

      <div class="card card-progress">
        <div class="card-title">{{ t('conversion.progressTitle') }}</div>
        <div class="control-steps">
          <div class="step" :class="{ active: hasItems }">
            <span class="step-dot"></span>
            <span class="step-text">{{ t('conversion.stepImport') }}</span>
          </div>
          <div class="step" :class="{ active: showProgress || isConverting }">
            <span class="step-dot"></span>
            <span class="step-text">{{ t('conversion.stepProcessing') }}</span>
          </div>
          <div class="step" :class="{ active: conversionResults.some(r => r && r.status === 'success') }">
            <span class="step-dot"></span>
            <span class="step-text">{{ t('conversion.stepOutput') }}</span>
          </div>
        </div>
        <div v-if="showProgress || isConverting" class="progress-section">
          <div class="progress-info">
            <i v-if="isConverting" class="ri-loader-4-line spin" aria-hidden="true"></i>
            <span class="status-text">{{ progressText }}</span>
          </div>
          <div v-if="manyFilesWarning" class="many-files-warning">
            <i class="ri-time-line" aria-hidden="true"></i>
            {{ t('conversion.manyFilesWarning') }}
          </div>
          <button
            v-if="isConverting"
            class="cancel-btn"
            @click="cancelConversion"
            :disabled="isCancelling"
          >
            <i class="ri-close-circle-line" aria-hidden="true"></i>
            {{ isCancelling ? t('conversion.cancelling') : t('conversion.cancel') }}
          </button>
        </div>
        <div v-else class="idle-state">
          {{ t('conversion.idleState') }}
        </div>
      </div>

      <div class="card card-drop">
        <div class="drop-header">
          <div class="drop-title">{{ t('conversion.dropTitle') }}</div>
        </div>

        <div
          class="drag-drop-frame"
          :class="{ 'is-dragover': isDragging, 'has-file': hasItems }"
          @dragover.prevent="isDragging = true"
          @dragleave.prevent="isDragging = false"
          @drop.prevent="onDrop"
          @click="triggerPicker"
        >
          <div class="drop-icon-container">
            <i v-if="!hasItems" class="ri-inbox-2-line drop-icon" aria-hidden="true"></i>
            <i v-else class="ri-checkbox-circle-line drop-icon success" aria-hidden="true"></i>
          </div>
          <div class="drop-text">
            <h3>{{ importMode === 'file' ? t('conversion.dropFilePrompt') : t('conversion.dropFolderPrompt') }}</h3>
            <p>{{ t('conversion.dropSubPrompt') }}</p>
          </div>
        </div>

        <div class="selected-items-list" v-if="hasItems">
          <div class="list-header">
            <span>{{ t('conversion.queueTitle', { count: selectedItems.length }) }}</span>
            <div class="list-actions">
              <button class="view-all-btn" @click="showItemsDialog = true">{{ t('conversion.viewFullList') }}</button>
              <button class="clear-btn" @click="clearSelection">{{ t('conversion.clearAll') }}</button>
            </div>
          </div>
          <div class="items-scroll">
            <div v-for="(item, idx) in previewItems" :key="`${item.path}-${idx}`" class="item-row" @mousemove="updateMousePosition">
              <div class="item-info">
                <span class="item-icon">
                  <i class="ri-file-3-line" aria-hidden="true"></i>
                </span>
                <span class="item-name">{{ item.name }}</span>
              </div>
              <div class="item-actions">
                <span class="item-size">{{ item.size }}</span>
                <button class="remove-item-btn" @click.stop="removeItem(selectedItems.indexOf(item))">×</button>
              </div>
            </div>
            <div v-if="selectedItems.length > previewLimit" class="more-row">
              {{ t('conversion.moreItems', { count: selectedItems.length - previewLimit }) }}
            </div>
          </div>
        </div>

        <div class="action-bar">
          <button class="start-conversion-button" @click="startConversion" :disabled="!hasItems || isConverting">
            <template v-if="!isConverting">{{ t('conversion.startConvert') }}</template>
            <template v-else>{{ t('conversion.processing') }}</template>
          </button>
        </div>
      </div>
    </div>

    <transition name="dialog-pop">
    <div class="dialog-overlay" v-if="showResultModal" @click="showResultModal = false">
      <div class="dialog-content" @click.stop>
        <div class="dialog-header">
          <h3>{{ conversionResults && conversionResults.length > 0 ? t('conversion.resultSuccess') : t('conversion.resultFailed') }}</h3>
          <button class="close-btn" @click="showResultModal = false">×</button>
        </div>
        <div class="dialog-body">
          <div v-if="conversionResults && conversionResults.length > 0">
            <div class="result-summary">
              <div class="result-item">
                <span class="result-label">{{ t('conversion.totalLabel') }}</span>
                <span class="result-value">{{ conversionResults.length }} 个</span>
              </div>
              <div class="result-item">
                <span class="result-label">{{ t('conversion.successLabel') }}</span>
                <span class="result-value success">{{ conversionResults.filter(r => r.status === 'success').length }} 个</span>
              </div>
              <div class="result-item">
                <span class="result-label">{{ t('conversion.failLabel') }}</span>
                <span class="result-value error">{{ conversionResults.filter(r => r.status !== 'success').length }} 个</span>
              </div>
            </div>
            <div v-if="conversionResults.filter(r => r.status !== 'success').length > 0" class="error-details">
              <h4>{{ t('conversion.errorDetails') }}</h4>
              <ul class="error-list">
                <li v-for="(result, index) in conversionResults.filter(r => r.status !== 'success')" :key="index">
                  {{ result.fileName || selectedItems[index]?.name || t('conversion.errorFile') }}: {{ result.error || t('common.unknownError') }}
                </li>
              </ul>
            </div>
          </div>
          <div v-else>
            <p>{{ t('conversion.severeError') }}</p>
          </div>
        </div>
        <div class="dialog-footer">
          <button class="dialog-button" @click="openOutputFolder" :disabled="!conversionResults || conversionResults.filter(r => r.status === 'success').length === 0">
            {{ t('conversion.openOutputDir') }}
          </button>
          <button class="dialog-button secondary" @click="exportLogsToFile" :disabled="logMessages.length === 0">
            {{ t('conversion.exportLog') }}
          </button>
          <button class="dialog-button secondary" @click="showResultModal = false">{{ t('common.close') }}</button>
        </div>
      </div>
    </div>
    </transition>

    <transition name="sidebar-overlay-fade">
      <div
        v-if="showVersionPicker"
        class="sidebar-overlay"
        @click="showVersionPicker = false"
        @keydown.esc="showVersionPicker = false"
      ></div>
    </transition>

    <transition
      :css="false"
      @before-enter="onSidebarBeforeEnter"
      @enter="onSidebarEnter"
      @after-enter="onSidebarAfterEnter"
      @before-leave="onSidebarBeforeLeave"
      @leave="onSidebarLeave"
      @after-leave="onSidebarAfterLeave"
    >
      <aside
        v-if="showVersionPicker"
        class="sidebar-content version-sidebar"
        ref="versionSidebar"
        @click.stop
        tabindex="-1"
      >
        <div class="sidebar-header">
          <div class="sidebar-header-text">
            <h3>{{ t('conversion.selectVersion') }}</h3>
            <p class="sidebar-hint">{{ selectedVersionEntry.label }} · {{ t('conversion.packFormat', { n: selectedVersionEntry.packFormat }) }}</p>
          </div>
          <button class="sidebar-close" @click="showVersionPicker = false" :aria-label="t('common.close')">
            <i class="ri-close-line" aria-hidden="true"></i>
          </button>
        </div>
        <div class="sidebar-body">
          <div
            v-for="(group, gi) in versionsByEra"
            :key="group.era"
            class="version-era"
          >
            <div class="version-era-header">
              <span class="version-era-name">{{ t(`conversion.versionEras.${group.era}`) }}</span>
              <span class="version-era-count">{{ group.items.length }}</span>
            </div>
            <div class="version-list">
              <button
                v-for="(v, i) in group.items"
                :key="v.label"
                class="version-row"
                :class="{
                  active: v.label === selectedVersion,
                  'has-status': v.status,
                }"
                :style="{ '--card-delay': `${(gi * 50) + (i * 25)}ms` }"
                @click="selectedVersion = v.label; showVersionPicker = false"
              >
                <div class="version-row-main">
                  <span class="version-row-label">{{ v.label }}</span>
                  <span class="version-row-meta">{{ t('conversion.packFormat', { n: v.packFormat }) }}</span>
                </div>
                <div class="version-row-tail">
                  <span v-if="v.status" class="version-status" :class="`status-${v.status}`">
                    {{ t(`conversion.versionStatus.${v.status}`) }}
                  </span>
                  <i v-if="v.label === selectedVersion" class="ri-check-line version-row-check" aria-hidden="true"></i>
                </div>
              </button>
            </div>
          </div>
        </div>
      </aside>
    </transition>

    <transition name="dialog-pop">
      <div v-if="showItemsDialog" class="dialog-overlay" @click="showItemsDialog = false">
        <div class="dialog-content items-dialog" @click.stop>
          <div class="dialog-header">
            <h3>{{ t('conversion.fullImportList', { count: selectedItems.length }) }}</h3>
            <button class="dialog-close" @click="showItemsDialog = false" :aria-label="t('common.close')">×</button>
          </div>
          <div class="items-dialog-list">
            <div v-for="(item, idx) in selectedItems" :key="`${item.path}-${idx}`" class="item-row">
              <div class="item-info">
                <span class="item-icon"><i class="ri-file-3-line" aria-hidden="true"></i></span>
                <span class="item-name">{{ item.name }}</span>
              </div>
              <div class="item-actions">
                <span class="item-size">{{ item.size }}</span>
                <button class="remove-item-btn" @click.stop="removeItem(idx)">×</button>
              </div>
            </div>
          </div>
          <div class="dialog-footer">
            <button class="dialog-button secondary" @click="showItemsDialog = false">{{ t('common.close') }}</button>
          </div>
        </div>
      </div>
    </transition>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { open, save, ask } from '@tauri-apps/plugin-dialog';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useNotification } from '../composables/useNotification';
// dialog enter/leave 走全局 CSS class 模式(`<transition name="dialog-pop">` +
// `name="dialog-pop-fast">`,见 App.vue 全局 .dialog-pop-* / .dialog-pop-fast-*
// 规则)。Vue 3 在 element insert 时直接加 enter-from class,跟 element 同一个
// commit,第一帧 paint 一定看到 from 状态 → 杜绝「打开瞬间闪一下」。
// version picker leave 700ms 走 .dialog-pop-fast-leave-active class 内部
// 700ms transition-duration 规则(配合内部 .version-card spring leave)。

const emit = defineEmits(['switch-page']);
const props = defineProps<{
  sourceHandling?: 'ask' | 'delete' | 'keep';
  openOutputAfterConvert?: boolean;
}>();
const { t } = useI18n();
const { notify, registerToastAction, unregisterToastAction } = useNotification();

const importMode = ref<'file' | 'folder'>('file');
const isDragging = ref(false);
let dragUnlisten: (() => void) | null = null;
const isConverting = ref(false);
const isCancelling = ref(false);
const showProgress = ref(false);
const progressText = ref(t('conversion.ready'));
// Warn when a batch is large enough that it may take a while.
const MANY_FILES_THRESHOLD = 10;
const manyFilesWarning = ref(false);
const selectedVersion = ref('1.21-1.21.1');
const fixAlphaLayers = ref(false);
const selectedItems = ref<any[]>([]);
const showResultModal = ref(false);
const conversionResults = ref<any[]>([]);
const logMessages = ref<string[]>([]);
const outputMode = ref<'follow' | 'fixed'>('follow');
const outputPath = ref('');
const showVersionPicker = ref(false);
const versionSidebar = ref<HTMLElement | null>(null);
const showItemsDialog = ref(false);
const previewLimit = 3;

type VersionStatus = 'latest' | 'stable';
type VersionEra =
  | 'classic'      // 1.6 – 1.12
  | 'modern'       // 1.13 – 1.16  (Update Aquatic, Village & Pillage, Nether)
  | 'cavesCliffs'  // 1.17 – 1.19  (Caves & Cliffs I/II, Wild)
  | 'trailsTales'  // 1.20
  | 'trickyTrials' // 1.21
  | 'bravery';     // 26.1+ (Bundles of Bravery)

interface VersionEntry {
  label: string;        // 唯一 key,跟 versionMap 对齐
  range: string;        // 显示的版本区间,如 "1.6 → 1.8"
  packFormat: number;   // pack_format 数值
  era: VersionEra;
  status?: VersionStatus;
}

const versions: VersionEntry[] = [
  { label: '1.6-1.8',         range: '1.6 → 1.8',         packFormat: 1,  era: 'classic' },
  { label: '1.9-1.10',        range: '1.9 → 1.10',        packFormat: 2,  era: 'classic' },
  { label: '1.11-1.12',       range: '1.11 → 1.12',       packFormat: 3,  era: 'classic' },
  { label: '1.13-1.14',       range: '1.13 → 1.14',       packFormat: 4,  era: 'modern' },
  { label: '1.15-1.16.1',     range: '1.15 → 1.16.1',     packFormat: 5,  era: 'modern' },
  { label: '1.16.2-1.16.5',   range: '1.16.2 → 1.16.5',   packFormat: 6,  era: 'modern' },
  { label: '1.17',            range: '1.17',              packFormat: 7,  era: 'cavesCliffs' },
  { label: '1.18',            range: '1.18',              packFormat: 8,  era: 'cavesCliffs' },
  { label: '1.19-1.19.2',     range: '1.19 → 1.19.2',     packFormat: 9,  era: 'cavesCliffs' },
  { label: '1.19.3',          range: '1.19.3',            packFormat: 12, era: 'cavesCliffs' },
  { label: '1.19.4',          range: '1.19.4',            packFormat: 13, era: 'cavesCliffs' },
  { label: '1.20-1.20.1',     range: '1.20 → 1.20.1',     packFormat: 15, era: 'trailsTales' },
  { label: '1.20.2',          range: '1.20.2',            packFormat: 18, era: 'trailsTales' },
  { label: '1.20.3-1.20.4',   range: '1.20.3 → 1.20.4',   packFormat: 22, era: 'trailsTales' },
  { label: '1.20.5-1.20.6',   range: '1.20.5 → 1.20.6',   packFormat: 32, era: 'trailsTales' },
  { label: '1.21-1.21.1',     range: '1.21 → 1.21.1',     packFormat: 34, era: 'trickyTrials' },
  { label: '1.21.2-1.21.3',   range: '1.21.2 → 1.21.3',   packFormat: 42, era: 'trickyTrials' },
  { label: '1.21.4',          range: '1.21.4',            packFormat: 46, era: 'trickyTrials' },
  { label: '1.21.5',          range: '1.21.5',            packFormat: 55, era: 'trickyTrials' },
  { label: '1.21.6',          range: '1.21.6',            packFormat: 63, era: 'trickyTrials' },
  { label: '1.21.7-1.21.8',   range: '1.21.7 → 1.21.8',   packFormat: 64, era: 'trickyTrials' },
  { label: '1.21.9-1.21.10',  range: '1.21.9 → 1.21.10',  packFormat: 69, era: 'trickyTrials' },
  { label: '1.21.11',         range: '1.21.11',           packFormat: 75, era: 'trickyTrials', status: 'stable' },
  { label: '26.1-26.1.2',     range: '26.1 → 26.1.2',     packFormat: 84, era: 'bravery',    status: 'latest' },
  { label: '26.2',            range: '26.2',              packFormat: 88, era: 'bravery',    status: 'latest' },
];

// Era order for the picker dialog (chronological, oldest first).
const eraOrder: VersionEra[] = [
  'classic', 'modern', 'cavesCliffs', 'trailsTales', 'trickyTrials', 'bravery',
];

const selectedVersionEntry = computed(
  () => versions.find(v => v.label === selectedVersion.value) ?? versions[versions.length - 1]
);

const versionsByEra = computed(() => {
  const groups: Record<VersionEra, VersionEntry[]> = {
    classic: [], modern: [], cavesCliffs: [], trailsTales: [], trickyTrials: [], bravery: [],
  };
  for (const v of versions) groups[v.era].push(v);
  return eraOrder
    .filter(era => groups[era].length > 0)
    .map(era => ({ era, items: groups[era] }));
});

const hasItems = computed(() => selectedItems.value.length > 0);
const previewItems = computed(() => selectedItems.value.slice(0, previewLimit));

const goBack = () => emit('switch-page', 'home');

const loadOutputSettings = async () => {
  const savedMode = localStorage.getItem('outputMode');
  const savedPath = localStorage.getItem('outputPath');
  if (savedMode === 'follow' || savedMode === 'fixed') outputMode.value = savedMode;
  if (savedPath) outputPath.value = savedPath;
  try {
    const cfg = await invoke<any>('get_config');
    if (cfg?.output_mode === 'follow' || cfg?.output_mode === 'fixed') {
      outputMode.value = cfg.output_mode;
    }
    if (typeof cfg?.output_path === 'string' && cfg.output_path.length > 0) {
      outputPath.value = cfg.output_path;
    }
  } catch {
    // ignore
  }
};

onMounted(async () => {
  void loadOutputSettings();

  // Register the toast-action handler for “Open output folder”. The
  // toast page emits `conv:open-output` via Rust when the user clicks
  // the action button on the conversion-complete toast.
  registerToastAction('conv:open-output', () => {
    void openOutputFolder();
  });

  // 注册 Tauri 窗口级拖拽事件（获取真实文件路径）
  try {
    const win = getCurrentWindow();
    dragUnlisten = await win.onDragDropEvent((event) => {
      if (event.payload.type === 'over') {
        isDragging.value = true;
      } else if (event.payload.type === 'leave') {
        isDragging.value = false;
      } else if (event.payload.type === 'drop') {
        isDragging.value = false;
        const paths = event.payload.paths || [];
        if (paths.length === 0) return;
        for (const p of paths) {
          const fileName = p.split('/').pop() || p.split('\\').pop() || p;
          const ext = fileName.toLowerCase().split('.').pop() || '';
          if (importMode.value === 'file') {
            if (ext !== 'zip' && ext !== 'mcpack') continue;
            selectedItems.value.push({ name: fileName, size: '', path: p, isDir: false });
          } else {
            selectedItems.value.push({ name: fileName, size: 'Folder', path: p, isDir: true });
          }
        }
      }
    });
  } catch (e) {
    console.warn('Drag-drop not available, falling back to file picker only:', e);
  }
});

onUnmounted(() => {
  if (dragUnlisten) {
    dragUnlisten();
    dragUnlisten = null;
  }
  window.removeEventListener('keydown', handleVersionPickerKey);
  // Tear down the toast action handlers we registered on mount so a
  // later conversion (or page) doesn't accidentally fire them.
  unregisterToastAction('conv:open-output');
});

/* === Sidebar enter/leave 钩子 =================================
   完全弃用 @keyframes 动画。改用 inline style + CSS transition 手动驱动:
   - onSidebarBeforeEnter: 锁初始 transform 100%,box-shadow none
   - onSidebarEnter:        raf 后设 transition + 终值(0 + 0.08),触发 CSS transition
   - onSidebarAfterEnter:   锁终态(0 + 0.08),防止 stylesheet 默认 transform 重新生效
   - onSidebarBeforeLeave:  直接设 transition + 终值 transform 100% + box-shadow none
                            (不依赖 raf,inline style 改动是同步的,浏览器自动触发 transition)
   - onSidebarLeave:        啥都不做,Vue 等 done() 调
   - onSidebarAfterLeave:   清空 inline style,v-if 移除元素

   enter/leave 都靠 inline style 触发 CSS transition 渐变 — 所有 transform /
   box-shadow / opacity 状态都在 inline style 里,不被 App.vue 全局 .page-shell
   > * > * 的 page-entry stagger 干扰。

   box-shadow 数值(距离/blur/opacity)跟 .sidebar-content 默认 CSS 保持一致。 */
function onSidebarBeforeEnter(el: Element) {
  const h = el as HTMLElement;
  h.style.transition = 'none';
  h.style.transform = 'translateX(100%)';
  h.style.boxShadow = 'none';
  h.style.opacity = '1';
}

function onSidebarEnter(el: Element, done: () => void) {
  const h = el as HTMLElement;
  // 强制 reflow 让 transition: none 先 commit
  h.offsetHeight;
  // raf 后设 transition + 终值,触发 CSS transition 渐变
  requestAnimationFrame(() => {
    h.style.transition = 'transform 450ms cubic-bezier(0.22, 1, 0.36, 1), box-shadow 450ms cubic-bezier(0.22, 1, 0.36, 1)';
    h.style.transform = 'translateX(0)';
    h.style.boxShadow = '-12px 0 36px rgba(0, 0, 0, 0.08)';
    setTimeout(done, 460);
  });
}

function onSidebarAfterEnter(el: Element) {
  const h = el as HTMLElement;
  // 锁终态:清掉 transition 防止后续状态变化被 transition 拦截
  h.style.transition = '';
  h.style.transform = 'translateX(0)';
  h.style.boxShadow = '-12px 0 36px rgba(0, 0, 0, 0.08)';
  h.style.opacity = '1';
}

function onSidebarBeforeLeave(el: Element) {
  const h = el as HTMLElement;
  // 当前 transform = 0(afterEnter 锁),box-shadow = 0.08
  // 直接设 transition + 终值(transform 100% + box-shadow none)
  // 浏览器看到 inline style 改动从 0 跳到 100%,自动触发 CSS transition 渐变
  h.style.transition = 'transform 400ms cubic-bezier(0.65, 0, 0.35, 1), box-shadow 400ms cubic-bezier(0.65, 0, 0.35, 1)';
  h.style.transform = 'translateX(100%)';
  h.style.boxShadow = 'none';
}

function onSidebarLeave(_el: Element, done: () => void) {
  // Vue 等 done() 调才 unmount,等 410ms 让 transition 跑完
  setTimeout(done, 410);
}

function onSidebarAfterLeave(el: Element) {
  const h = el as HTMLElement;
  h.style.transition = '';
  h.style.transform = '';
  h.style.boxShadow = '';
  h.style.opacity = '';
}

const handleVersionPickerKey = (e: KeyboardEvent) => {
  if (e.key === 'Escape' && showVersionPicker.value) {
    showVersionPicker.value = false;
  }
};

watch(showVersionPicker, (open) => {
  if (open) {
    // ESC 关闭 sidebar
    window.addEventListener('keydown', handleVersionPickerKey);
    // 自动 focus sidebar 容器(让内部 button 也能接收键盘事件)
    setTimeout(() => {
      versionSidebar.value?.focus?.();
    }, 50);
  } else {
    window.removeEventListener('keydown', handleVersionPickerKey);
  }
});

const triggerPicker = async () => {
  try {
    const result = await open({
      multiple: true,
      directory: importMode.value === 'folder',
      filters: importMode.value === 'file' ? [
        { name: t('conversion.resourcePackFilter'), extensions: ['zip', 'mcpack'] },
        { name: t('conversion.allFilesFilter'), extensions: ['*'] }
      ] : undefined
    });

    if (result) {
      const paths = Array.isArray(result) ? result : [result];
      for (const path of paths) {
        const fileName = path.split('/').pop() || path.split('\\').pop() || path;
        selectedItems.value.push({
          name: fileName,
          size: importMode.value === 'folder' ? 'Folder' : (Math.random() * 20 + 5).toFixed(1) + ' MB',
          path,
          isDir: importMode.value === 'folder'
        });
      }
    }
  } catch (error) {
    notify({
      title: t('conversion.fileSelectErrorTitle'),
      body: t('conversion.fileSelectErrorBody'),
      type: 'error',
      source: 'conversion'
    });
  }
};

const onDrop = () => {
  isDragging.value = false;
  // 实际文件处理由 Tauri onDragDropEvent 完成
};

const updateMousePosition = () => {};

const removeItem = (index: number) => {
  selectedItems.value.splice(index, 1);
};

const clearSelection = () => {
  selectedItems.value = [];
};

const startConversion = async () => {
  if (!hasItems.value || isConverting.value) return;
  if (outputMode.value === 'fixed' && (!outputPath.value || outputPath.value.trim().length === 0)) {
    notify({
      title: t('conversion.outputNotSetTitle'),
      body: t('conversion.outputNotSetBody'),
      type: 'error',
      source: 'conversion'
    });
    return;
  }

  isConverting.value = true;
  isCancelling.value = false;
  showProgress.value = true;
  progressText.value = t('conversion.converting');
  logMessages.value = [];

  // Warn when converting a lot of files at once.
  manyFilesWarning.value = selectedItems.value.length >= MANY_FILES_THRESHOLD;

  notify({
    title: t('conversion.convStartTitle'),
    body: t('conversion.convStartBody', { count: selectedItems.value.length }),
    type: 'info',
    source: 'conversion'
  });


  const versionMap: Record<string, number> = Object.fromEntries(
    versions.map(v => [v.label, v.packFormat])
  );
  const targetFormat = versionMap[selectedVersion.value] || 75;

  const filePaths = selectedItems.value.map(item => item.path);
  const outputDirs = selectedItems.value.map(() => {
    if (outputMode.value === 'fixed') return outputPath.value || '';
    return '';
  });

  let conversionError = false;

  // (No fake/interpolated progress: the Rust scheduler reports real
  //  module progress (`Progress: X/N`), which we aggregate across all
  //  packs below. The bar moves only when modules actually complete.)

  try {
    conversionResults.value = await invoke<any[]>('convert_resource_packs_batch', {
      filePaths,
      targetFormat,
      outputDirs,
      fixAlphaLayers: fixAlphaLayers.value,
    });

    for (let i = 0; i < selectedItems.value.length; i++) {
      const result = conversionResults.value[i];
      if (result && result.status === 'success') {
        selectedItems.value[i].output_path = result.output;
      }
    }

    const successCount = conversionResults.value.filter(r => r.status === 'success').length;
    const totalCount = conversionResults.value.length || selectedItems.value.length;
    progressText.value = t('conversion.convEnd', { success: successCount, total: totalCount });

    // Build the toast actions: an “Open Folder” button whenever we
    // can resolve an output directory, so the user can jump straight
    // there without digging through the result modal.
    const actions: { id: string; label: string; icon: string }[] = [];
    if (resolveOutputDir()) {
      actions.push({
        id: 'conv:open-output',
        label: t('openBtn'),
        icon: 'ri-folder-open-line',
      });
    }

    notify({
      title: t('conversion.convCompleteTitle'),
      body: t('conversion.convCompleteBody', { count: successCount }),
      type: 'success',
      source: 'conversion',
      actions,
    });
  } catch (err) {
    conversionError = true;
    progressText.value = t('conversion.severeError');
    conversionResults.value = [];
    notify({
      title: t('conversion.severeErrorTitle'),
      body: t('conversion.severeErrorBody'),
      type: 'error',
      source: 'conversion'
    });
  } finally {
    isConverting.value = false;
    isCancelling.value = false;
    manyFilesWarning.value = false;
    // Collapse the converting panel back to the idle state.
    showProgress.value = false;
    console.log('[conv] finished: results=', conversionResults.value);
    if (!conversionError) {
      const failureCount = conversionResults.value.filter(r => r.status !== 'success').length;
      const successCount = conversionResults.value.filter(r => r.status === 'success').length;
      // Cancelled: the user pressed Cancel — show a neutral state.
      const cancelledCount = conversionResults.value.filter(r => r.status === 'cancelled').length;
      if (cancelledCount > 0) {
        progressText.value = t('conversion.cancelledText');
        return;
      }
      if (failureCount === 0) {
        showResultModal.value = false;
        const totalCount = conversionResults.value.length || selectedItems.value.length;

        // Post-conversion side effects driven by Settings:
        //   * Auto-open the output folder (if enabled)
        //   * Apply source-pack handling policy (ask/delete/keep)
        if (props.openOutputAfterConvert !== false && resolveOutputDir()) {
          // Fire-and-forget; we don't want a failed open_folder call
          // to break the rest of the success path.
          void openOutputFolder();
        }
        applySourceHandling();

        // Same actions as the “running” notify above.
        const actions: { id: string; label: string; icon: string }[] = [];
        if (resolveOutputDir()) {
          actions.push({
            id: 'conv:open-output',
            label: t('openBtn'),
            icon: 'ri-folder-open-line',
          });
        }
        notify({
          title: t('conversion.convCompleteTitle'),
          body: t('conversion.convAllSuccessBody', { success: successCount, total: totalCount }),
          type: 'success',
          source: 'conversion',
          actions,
        });
      } else {
        showResultModal.value = true;
      }
    }
  }
};

/**
 * Ask the Rust backend to abort the running batch conversion. The
 * backend stops before the next file; already-completed files stay
 * intact. We show a transient “cancelling…” state on the button.
 */
const cancelConversion = async () => {
  if (!isConverting.value || isCancelling.value) return;
  isCancelling.value = true;
  try {
    await invoke('cancel_conversion');
  } catch (e) {
    console.error('[cancel] cancel_conversion failed:', e);
    isCancelling.value = false;
  }
};

const openOutputFolder = async () => {
  if (selectedItems.value.length === 0) return;
  try {
    const successResult = Array.isArray(conversionResults.value)
      ? conversionResults.value.find(r => r && r.status === 'success' && r.output)
      : null;
    const samplePath = successResult?.output
      || selectedItems.value[0].output_path
      || (outputMode.value === 'fixed' ? outputPath.value : selectedItems.value[0].path);
    const dirPath = samplePath.replace(/[^\/\\]+$/, '');
    await invoke('open_folder', { path: dirPath });
  } catch (e) {
    notify({
      title: t('conversion.openOutputErrorTitle'),
      body: t('conversion.openOutputErrorBody'),
      type: 'error',
      source: 'conversion'
    });
  }
};

/**
 * Resolve the directory we should pop open after a successful
 * conversion. Returns "" when we don't have a usable path (e.g. the
 * user is in `follow` mode and the conversion never produced an
 * output). Shared by the auto-open and toast-button paths so they
 * always agree on what "the output folder" means.
 */
const resolveOutputDir = (): string => {
  const successResult = Array.isArray(conversionResults.value)
    ? conversionResults.value.find(r => r && r.status === 'success' && r.output)
    : null;
  const samplePath = successResult?.output
    || (selectedItems.value[0]?.output_path ?? '')
    || (outputMode.value === 'fixed' ? outputPath.value : (selectedItems.value[0]?.path ?? ''));
  if (!samplePath) return '';
  return samplePath.replace(/[^\\/]+$/, '');
};

/**
 * Remove the source packs from disk. We invoke a Rust command instead
 * of fs.rm from JS so the OS actually frees the files (Tauri sandboxes
 * direct fs access in production). Only runs when at least one source
 * path is recorded AND the user has opted in via `sourceHandling`.
 */
const deleteSourcePacks = async (): Promise<void> => {
  const paths = selectedItems.value
    .map((it) => it.path)
    .filter((p): p is string => typeof p === 'string' && p.length > 0);
  if (paths.length === 0) return;
  try {
    await invoke('delete_paths', { paths });
    notify({
      title: t('common.delete'),
      body: t('settings.sourceHandling.delete'),
      type: 'success',
      source: 'conversion',
    });
  } catch (e) {
    notify({
      title: t('settings.sourceHandling.deleteFailedTitle'),
      body: String(e),
      type: 'error',
      source: 'conversion',
    });
  }
};

/**
 * Apply the user's `sourceHandling` setting after a successful batch.
 *   * "ask"    → async confirmation via the Tauri dialog plugin.
 *                IMPORTANT: never use `window.confirm` here — in
 *                WebView2 it blocks the JS main thread (freezing ping
 *                heartbeats, window controls, and state updates).
 *   * "delete" → delete unconditionally
 *   * "keep"   → no-op
 */
const applySourceHandling = async (): Promise<void> => {
  const policy = props.sourceHandling ?? 'ask';
  if (policy === 'keep') return;
  if (policy === 'delete') {
    await deleteSourcePacks();
    return;
  }
  try {
    const ok = await ask(t('settings.sourceHandling.deleteBody'), {
      title: t('settings.sourceHandling.deleteTitle'),
      kind: 'warning',
    });
    if (ok) await deleteSourcePacks();
  } catch {
    // Dialog unavailable — skip silently.
  }
};

const exportLogsToFile = async () => {
  if (logMessages.value.length === 0) {
    notify({
      title: t('conversion.noLogTitle'),
      body: t('conversion.noLogBody'),
      type: 'info',
      source: 'conversion'
    });
    return;
  }

  try {
    const savePath = await save({
      filters: [
        { name: t('conversion.logFileFilter'), extensions: ['log'] },
        { name: t('conversion.allFilesFilter'), extensions: ['*'] }
      ],
      defaultPath: `conversion_log_${new Date().toISOString().replace(/[:.]/g, '-')}.log`
    });

    if (savePath) {
      const logContent = logMessages.value.join('\n');
      await invoke('write_file', { path: savePath, content: logContent });
      notify({
        title: t('conversion.logExportSuccessTitle'),
        body: t('conversion.logExportSuccessBody', { path: savePath }),
        type: 'success',
        source: 'conversion'
      });
    }
  } catch {
    notify({
      title: t('conversion.logExportFailedTitle'),
      body: t('conversion.logExportFailedBody'),
      type: 'error',
      source: 'conversion'
    });
  }
};
</script>

<style scoped>
.conversion-page {
  width: 100%;
  height: 100%;
  min-height: 0;
  /* Background gradient + aurora ::before are provided by App.vue's
     `.page-shell > *` rule so all three pages render identically. */
  color: #1d1d1f;
  overflow: hidden;
  position: relative;
  display: flex;
  flex-direction: column;
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", "PingFang SC", "Microsoft YaHei", sans-serif;
}

.header-section {
  z-index: 5;
  padding: 24px 48px 8px;
  display: flex;
  align-items: center;
  gap: 20px;
  flex-shrink: 0;
}
.title-group { display: flex; flex-direction: column; }
.back-btn {
  background: rgba(0, 0, 0, 0.05);
  border: none;
  padding: 10px 18px;
  border-radius: 14px;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  gap: 8px;
  font-weight: 600;
  color: #111827;
  transition: 0.3s;
}
.back-btn:hover { background: rgba(0, 0, 0, 0.1); transform: translateX(-4px); }
.back-icon { font-size: 16px; line-height: 1; color: #111827; }
.page-title { font-size: 28px; font-weight: 800; letter-spacing: -1px; margin: 0; }
.page-subtitle { margin: 6px 0 0; color: #86868b; font-size: 13px; }

.switch-title {
  display: inline-flex;
  align-items: center;
  gap: 8px;
}
.help-icon {
  width: 18px;
  height: 18px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: 999px;
  font-size: 12px;
  font-weight: 800;
  color: #111827;
  background: rgba(0, 0, 0, 0.08);
  cursor: help;
  user-select: none;
}
.help-icon:hover {
  background: rgba(0, 0, 0, 0.12);
}

.panel-grid {
  flex: 1;
  min-height: 0;
  z-index: 2;
  padding: 14px 48px 24px;
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
  grid-template-rows: auto 1fr;
  grid-template-areas:
    "mode version"
    "drop progress";
  gap: 16px;
}

.card {
  background: rgba(255, 255, 255, 0.74);
  backdrop-filter: blur(20px);
  border: 1px solid rgba(0, 0, 0, 0.05);
  border-radius: 26px;
  padding: 16px 20px;
  box-shadow: 0 16px 34px rgba(0,0,0,0.06);
  display: flex;
  flex-direction: column;
  gap: 10px;
  min-height: 0;
  overflow: hidden;
}
.card-title { font-size: 14px; font-weight: 700; color: #1f2937; flex-shrink: 0; }
.card-hint { font-size: 12px; color: #94a3b8; margin: 0; flex-shrink: 0; }

.card-mode { grid-area: mode; }
.card-version { grid-area: version; }
.card-progress { grid-area: progress; }
.card-drop { grid-area: drop; gap: 12px; }

.import-tabs {
  display: inline-flex;
  background: rgba(0,0,0,0.04);
  border-radius: 999px;
  padding: 4px;
  gap: 4px;
}
.tab-btn {
  border: none;
  background: transparent;
  padding: 8px 16px;
  border-radius: 999px;
  font-size: 13px;
  font-weight: 600;
  color: #6b7280;
  cursor: pointer;
  transition: 0.2s;
}
.tab-btn.active { background: #fff; color: #1d1d1f; box-shadow: 0 6px 14px rgba(0,0,0,0.08); }

/* === Version picker (chip-style trigger) === */
.version-picker {
  display: inline-flex;
  align-items: center;
  gap: 14px;
  background: linear-gradient(135deg,
    color-mix(in srgb, var(--theme-color) 8%, #ffffff) 0%,
    color-mix(in srgb, var(--theme-color) 4%, #ffffff) 100%);
  border: 1px solid color-mix(in srgb, var(--theme-color) 22%, transparent);
  padding: 10px 14px;
  border-radius: 14px;
  font-family: inherit;
  cursor: pointer;
  transition: transform 0.18s ease, box-shadow 0.18s ease, border-color 0.18s ease, background 0.18s ease;
  box-shadow: 0 1px 0 rgba(255,255,255,0.7) inset, 0 1px 2px rgba(0,0,0,0.04);
  min-width: 220px;
  text-align: left;
}
.version-picker:hover {
  border-color: color-mix(in srgb, var(--theme-color) 42%, transparent);
  box-shadow: 0 4px 14px color-mix(in srgb, var(--theme-color) 18%, transparent),
              0 1px 0 rgba(255,255,255,0.7) inset;
  transform: translateY(-1px);
}
.version-picker:active { transform: translateY(0); }
.version-picker.open {
  border-color: color-mix(in srgb, var(--theme-color) 55%, transparent);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--theme-color) 18%, transparent);
}
.version-picker-main {
  display: flex;
  flex-direction: column;
  gap: 2px;
  flex: 1;
  min-width: 0;
}
.version-picker-label {
  font-size: 14px;
  font-weight: 700;
  color: #1d1d1f;
  letter-spacing: -0.01em;
  white-space: nowrap;
}
.version-picker-range {
  font-size: 11px;
  color: #64748b;
  font-weight: 500;
  white-space: nowrap;
}
.version-picker-meta {
  display: flex;
  align-items: center;
  gap: 6px;
}
.version-picker-format {
  font-size: 10px;
  font-weight: 600;
  color: color-mix(in srgb, var(--theme-color) 75%, #000);
  background: color-mix(in srgb, var(--theme-color) 12%, transparent);
  padding: 2px 6px;
  border-radius: 6px;
  font-variant-numeric: tabular-nums;
  letter-spacing: 0.02em;
}
.version-picker-chevron {
  font-size: 18px;
  color: #94a3b8;
  transition: transform 0.25s cubic-bezier(.4,0,.2,1), color 0.18s ease;
}
.version-picker:hover .version-picker-chevron { color: var(--theme-color); }
.version-picker.open .version-picker-chevron { transform: rotate(180deg); color: var(--theme-color); }

.switch-line { display: flex; align-items: center; justify-content: space-between; font-size: 12px; color: #64748b; }
.switch { position: relative; display: inline-block; width: 42px; height: 24px; }
.switch input { opacity: 0; width: 0; height: 0; }
.slider {
  position: absolute; inset: 0; cursor: pointer; background-color: #e9e9eb; transition: .4s; border-radius: 34px;
}
.slider:before {
  position: absolute; content: ""; height: 18px; width: 18px; left: 3px; bottom: 3px; background-color: white; transition: .4s; border-radius: 50%;
  box-shadow: 0 2px 4px rgba(0,0,0,0.1);
}
.switch input:checked + .slider { background-color: var(--theme-color); }
.switch input:checked + .slider:before { transform: translateX(18px); }

.control-steps { display: grid; gap: 10px; }
.step { display: flex; align-items: center; gap: 10px; font-size: 12px; color: #94a3b8; }
.step-dot { width: 8px; height: 8px; border-radius: 50%; background: rgba(148,163,184,0.6); box-shadow: 0 0 0 4px rgba(148,163,184,0.15); }
.step.active { color: #1d1d1f; }
.step.active .step-dot { background: var(--theme-color); box-shadow: 0 0 0 4px color-mix(in srgb, var(--theme-color) 30%, transparent); }

.progress-section { margin-top: 4px; display: flex; flex-direction: column; gap: 10px; }
.progress-info { display: flex; align-items: center; gap: 8px; font-size: 13px; color: #374151; }
.progress-info .spin { color: var(--theme-color); font-size: 16px; }
.status-text { font-weight: 600; }

/* “Many files” warning row. */
.many-files-warning {
  display: flex; align-items: center; gap: 6px;
  font-size: 12px; color: #b45309;
  background: #fffbeb; border: 1px solid #fde68a;
  border-radius: 10px; padding: 8px 12px;
}
.many-files-warning i { font-size: 14px; }

/* Cancel conversion button. */
.cancel-btn {
  display: inline-flex; align-items: center; gap: 6px;
  padding: 8px 16px;
  font-size: 13px; font-weight: 600; font-family: inherit;
  color: #b91c1c;
  background: #fef2f2; border: 1px solid #fecaca;
  border-radius: 10px; cursor: pointer;
  align-self: flex-start;
  transition: all 0.2s ease;
}
.cancel-btn:hover:not(:disabled) { background: #fee2e2; }
.cancel-btn:disabled { opacity: 0.6; cursor: not-allowed; }

@keyframes spin { to { transform: rotate(360deg); } }
.spin { display: inline-block; animation: spin 1s linear infinite; }
.idle-state { color: #94a3b8; font-size: 12px; }

.drop-header { display: flex; align-items: center; justify-content: space-between; }
.drop-title { font-size: 14px; font-weight: 700; color: #1f2937; }

.drag-drop-frame {
  border: 1px dashed rgba(15, 23, 42, 0.16);
  border-radius: 22px;
  padding: 26px;
  text-align: center;
  background: rgba(255,255,255,0.55);
  transition: 0.3s;
  cursor: pointer;
}
.drag-drop-frame.is-dragover { border-color: rgba(59,130,246,0.6); background: rgba(59,130,246,0.06); transform: scale(1.01); }
.drag-drop-frame.has-file { border-style: dashed; }
.drop-icon-container { display: flex; align-items: center; justify-content: center; margin-bottom: 8px; }
.drop-icon { font-size: 36px; color: #111827; line-height: 1; }
.drop-icon.success { color: #16a34a; }
.drop-text h3 { margin: 0; font-size: 14px; color: #1f2937; }
.drop-text p { margin: 6px 0 0; color: #94a3b8; font-size: 12px; }

.selected-items-list {
  background: rgba(255, 255, 255, 0.85);
  border-radius: 16px;
  border: 1px solid rgba(0,0,0,0.05);
  overflow: hidden;
}
.list-header {
  display: flex; align-items: center; justify-content: space-between;
  padding: 10px 14px; font-size: 12px; color: #64748b; background: rgba(0,0,0,0.02);
}
.list-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}
.view-all-btn {
  border: none;
  background: rgba(59,130,246,0.1);
  color: #2563eb;
  font-size: 12px;
  font-weight: 700;
  padding: 4px 8px;
  border-radius: 8px;
  cursor: pointer;
}
.view-all-btn:hover { background: rgba(59,130,246,0.18); }
.clear-btn { border: none; background: transparent; color: #ef4444; font-weight: 600; cursor: pointer; }
.items-scroll { max-height: 140px; overflow-y: auto; }
.item-row { display: flex; align-items: center; justify-content: space-between; padding: 10px 14px; border-top: 1px solid rgba(0,0,0,0.04); }
.item-info { display: flex; align-items: center; gap: 10px; }
.item-icon { width: 18px; height: 18px; color: #0f172a; display: inline-flex; align-items: center; justify-content: center; }
.item-icon i { font-size: 18px; line-height: 1; }
.item-name { font-size: 13px; }
.item-actions { display: flex; align-items: center; gap: 10px; }
.item-size { font-size: 12px; color: #94a3b8; }
.remove-item-btn { border: none; background: rgba(0,0,0,0.06); width: 22px; height: 22px; border-radius: 999px; cursor: pointer; }
.more-row {
  padding: 10px 14px;
  border-top: 1px solid rgba(0,0,0,0.04);
  color: #64748b;
  font-size: 12px;
}

.action-bar { margin-top: auto; }
.start-conversion-button {
  width: 100%;
  border: none;
  background: #111827;
  color: #fff;
  padding: 12px 16px;
  border-radius: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: 0.2s;
}
.start-conversion-button:disabled { opacity: 0.6; cursor: not-allowed; }
.start-conversion-button:hover:not(:disabled) { background: #000; }

.dialog-overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.4); display: flex; align-items: center; justify-content: center; z-index: 100; backdrop-filter: blur(6px); }
.dialog-content { background: white; padding: 2rem; border-radius: 24px; width: 500px; box-shadow: 0 20px 60px rgba(0,0,0,0.15); max-height: 80vh; overflow-y: auto; }
.dialog-header { display: flex; justify-content: space-between; margin-bottom: 1rem; }
.dialog-close { border: none; background: transparent; font-size: 20px; cursor: pointer; color: #64748b; }
.dialog-body { margin-bottom: 1.5rem; }
.result-summary { margin-bottom: 1rem; }
.result-item { display: flex; justify-content: space-between; margin-bottom: 0.5rem; padding: 0.5rem; border-radius: 8px; background: #f8fafc; }
.result-value { font-weight: 700; }
.result-value.success { color: #10b981; }
.result-value.error { color: #ef4444; }
.error-details { margin-top: 1rem; padding: 1rem; border-radius: 8px; background: #fef2f2; border: 1px solid #fee2e2; }
.error-details h4 { margin-top: 0; color: #dc2626; font-size: 1rem; }
.error-list { margin: 0.5rem 0 0 0; padding-left: 1.5rem; }
.error-list li { margin-bottom: 0.25rem; color: #b91c1c; font-size: 0.9rem; }
.dialog-footer { display: flex; gap: 12px; justify-content: flex-end; margin-top: 2rem; }
.dialog-button { padding: 8px 20px; border-radius: 10px; border: none; background: var(--theme-color); color: white; font-weight: 600; cursor: pointer; transition: all 0.2s; }
.dialog-button:hover:not(:disabled) { background: color-mix(in srgb, var(--theme-color) 85%, #000); transform: translateY(-1px); }
.dialog-button.secondary { background: #f1f5f9; color: #475569; }
.dialog-button.secondary:hover { background: #e2e8f0; }
.dialog-button:disabled { background: #cbd5e1; cursor: not-allowed; }

/* === Version picker sidebar (右侧抽屉,从右滑入) === */
.sidebar-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.28);
  z-index: 200;
  /* 不用 backdrop-filter blur — 避免 chromium 在 leave 时中断 animationend,
     跟 dialog-pop overlay 同样策略。 */
}
.sidebar-content {
  /* position: fixed + right: 0 — aside 跟 .sidebar-overlay 是 sibling,
     不嵌套在 overlay flex 容器里。aside 自己用 right:0 锚定在屏幕右侧,
     transform translateX(100%) = + 自身 420px width 推到屏幕外右侧;
     transform translateX(0) = 回到 right:0 位置(屏幕内最右 420px)。 */
  position: fixed;
  top: 0;
  right: 0;
  width: min(420px, 92vw);
  height: 100vh;
  background: white;
  /* 阴影淡一点 — 之前 -20px 0 60px rgba(0,0,0,0.18) 偏重,改 12px/36px/0.08
     保持层级感但不抢戏。enter 期间 inline style 会把 box-shadow 渐变到 0,
     leave 期间渐变到 0(全透明,exit 时无残留)。 */
  box-shadow: -12px 0 36px rgba(0, 0, 0, 0.08);
  display: flex;
  flex-direction: column;
  outline: none;
  z-index: 201;
  /* 默认状态 = enter-from(屏幕外右侧)— element insert 时浏览器 paint 看不见。
     enter/leave 完全由 inline style + CSS transition 驱动(见 onSidebarEnter /
     onSidebarLeave),transform / box-shadow 状态由 inline style 维护,这里
     只设兜底,让 element insert 那一帧就在屏幕外、不闪。

     opacity: 1 !important 显式覆盖 App.vue 全局 .page-shell > * > *
     (那条 page-entry stagger 规则给 aside 锁 opacity: 0 渐变到 1,
     enter 期间 sidebar 一直隐形)。 */
  transform: translateX(100%);
  opacity: 1 !important;
  will-change: transform, box-shadow;
}

.sidebar-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  padding: 1.5rem 1.5rem 1rem;
  border-bottom: 1px solid rgba(0, 0, 0, 0.06);
  flex-shrink: 0;
}
.sidebar-header-text {
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 0;
}
.sidebar-header-text h3 {
  margin: 0;
  font-size: 18px;
  font-weight: 700;
  color: #1d1d1f;
  letter-spacing: -0.02em;
}
.sidebar-hint {
  margin: 0;
  font-size: 12px;
  color: #64748b;
  font-weight: 500;
  font-variant-numeric: tabular-nums;
}
.sidebar-close {
  width: 32px;
  height: 32px;
  flex-shrink: 0;
  border: none;
  background: transparent;
  color: #64748b;
  font-size: 20px;
  line-height: 1;
  cursor: pointer;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background 0.15s ease, color 0.15s ease;
}
.sidebar-close:hover {
  background: rgba(0, 0, 0, 0.05);
  color: #1d1d1f;
}

.sidebar-body {
  flex: 1;
  overflow-y: auto;
  padding: 1rem 1.25rem 1.5rem;
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
}
.sidebar-body::-webkit-scrollbar { width: 6px; }
.sidebar-body::-webkit-scrollbar-thumb {
  background: rgba(0, 0, 0, 0.12);
  border-radius: 3px;
}
.sidebar-body::-webkit-scrollbar-thumb:hover { background: rgba(0, 0, 0, 0.22); }

.version-era {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.version-era-header {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  padding: 0 4px;
}
.version-era-name {
  font-size: 11px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: #64748b;
}
.version-era-count {
  font-size: 10px;
  color: #94a3b8;
  font-weight: 600;
  font-variant-numeric: tabular-nums;
}

.version-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.version-row {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  padding: 10px 12px;
  border-radius: 10px;
  border: 1px solid transparent;
  background: transparent;
  font-family: inherit;
  cursor: pointer;
  text-align: left;
  overflow: hidden;
  transition:
    background 0.15s ease,
    border-color 0.15s ease;
  /* stagger entrance — sidebar body 出现后,row 一个个滑入 */
  animation: version-row-in 0.36s cubic-bezier(.2, .65, .3, 1) both;
  animation-delay: var(--card-delay, 0ms);
}
@keyframes version-row-in {
  from { opacity: 0; transform: translateX(12px); }
  to   { opacity: 1; transform: translateX(0); }
}
.version-row:hover {
  background: rgba(0, 0, 0, 0.04);
  border-color: color-mix(in srgb, var(--theme-color) 25%, transparent);
}
.version-row:active {
  background: rgba(0, 0, 0, 0.06);
}
.version-row.active {
  background: color-mix(in srgb, var(--theme-color) 10%, #ffffff);
  border-color: color-mix(in srgb, var(--theme-color) 45%, transparent);
}
.version-row.has-status { padding-right: 8px; }

.version-row-main {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
  flex: 1;
}
.version-row-label {
  font-size: 14px;
  font-weight: 600;
  color: #1d1d1f;
  letter-spacing: -0.01em;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.version-row.active .version-row-label {
  color: color-mix(in srgb, var(--theme-color) 90%, #000);
}
.version-row-meta {
  font-size: 10px;
  color: #94a3b8;
  font-weight: 600;
  font-variant-numeric: tabular-nums;
  letter-spacing: 0.02em;
}

.version-row-tail {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
}
.version-row-check {
  font-size: 16px;
  color: var(--theme-color);
  animation: version-check-pop 0.32s cubic-bezier(.34, 1.56, .64, 1);
}
@keyframes version-check-pop {
  from { opacity: 0; transform: scale(0.4); }
  to   { opacity: 1; transform: scale(1); }
}

/* === Version status badges (inline 在 .version-row-tail 里) === */
.version-status {
  font-size: 9px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  padding: 2px 6px;
  border-radius: 999px;
  line-height: 1.3;
}
.status-latest {
  color: #fff;
  background: linear-gradient(135deg, var(--theme-color), color-mix(in srgb, var(--theme-color) 65%, #000));
  box-shadow: 0 1px 3px color-mix(in srgb, var(--theme-color) 40%, transparent);
}
.status-stable {
  color: color-mix(in srgb, var(--theme-color) 85%, #000);
  background: color-mix(in srgb, var(--theme-color) 12%, transparent);
  border: 1px solid color-mix(in srgb, var(--theme-color) 30%, transparent);
}

.items-dialog {
  width: min(860px, 92vw);
}
.items-dialog-list {
  max-height: 420px;
  overflow: auto;
  border: 1px solid rgba(0,0,0,0.06);
  border-radius: 12px;
}

/* Sidebar enter/leave 走全局 .sidebar-pop-* 规则(见 App.vue)。
   sidebar-content 默认状态 = enter-from(translateX 100%),element insert 时
   浏览器 paint 看不见 → 杜绝「打开瞬间闪一下」。
   内部 row 用 version-row-in 动画 stagger 出现,sidebar leave 时整个 sidebar
   一起滑出(无需 row spring,简化 close 视觉)。 */

@keyframes bg-breathe {
  0% { background-position: 0% 0%; }
  50% { background-position: 100% 50%; }
  100% { background-position: 0% 100%; }
}
@keyframes aurora-drift {
  0%, 100% { transform: translate3d(0, 0, 0); opacity: 0.7; }
  50% { transform: translate3d(30px, -18px, 0); opacity: 0.9; }
}

@media (max-width: 980px) {
  .panel-grid {
    grid-template-columns: 1fr;
    grid-template-rows: auto auto auto 1fr;
    grid-template-areas: "mode" "version" "drop" "progress";
    padding: 12px 20px 24px;
  }
  .header-section { padding: 20px 20px 8px; }
  .card-drop { min-height: 280px; }
  .card-progress { min-height: 200px; }
}
@media (min-width: 981px) and (max-width: 1200px) {
  .panel-grid { padding: 14px 28px 24px; }
  .header-section { padding: 24px 28px 8px; }
}
@media (min-width: 1600px) {
  .panel-grid { padding: 20px 64px 32px; gap: 24px; }
  .header-section { padding: 28px 64px 8px; }
}
</style>
