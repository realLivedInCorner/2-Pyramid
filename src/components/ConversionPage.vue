<template>
  <div class="conversion-page">
    <div class="header-section">
      <button class="back-button" @click="goBack">
        <i class="ri-arrow-left-line back-icon" aria-hidden="true"></i>
        {{ t('common.back') }}
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
          <div class="step" :class="{ active: progressLabel === 100 }">
            <span class="step-dot"></span>
            <span class="step-text">{{ t('conversion.stepOutput') }}</span>
          </div>
        </div>
        <div v-if="showProgress || isConverting" class="progress-section">
          <div class="progress-info">
            <span class="status-text">{{ progressText }}</span>
            <span class="percent-num">{{ progressLabel }}%</span>
          </div>
          <div class="progress-bar-container">
            <div class="progress-bar" :style="{ width: progressBarPercentage + '%' }"></div>
          </div>
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

    <transition name="dialog-pop">
      <div v-if="showVersionPicker" class="dialog-overlay" @click="showVersionPicker = false">
        <div class="dialog-content version-dialog" @click.stop>
          <div class="dialog-header">
            <h3>{{ t('conversion.selectVersion') }}</h3>
            <button class="dialog-close" @click="showVersionPicker = false" :aria-label="t('common.close')">×</button>
          </div>
          <div class="version-dialog-body">
            <div
              v-for="(group, gi) in versionsByEra"
              :key="group.era"
              class="version-era"
            >
              <div class="version-era-header">
                <span class="version-era-name">{{ t(`conversion.versionEras.${group.era}`) }}</span>
                <span class="version-era-count">{{ group.items.length }}</span>
              </div>
              <div class="version-cards">
                <button
                  v-for="(v, i) in group.items"
                  :key="v.label"
                  class="version-card"
                  :class="{
                    active: v.label === selectedVersion,
                    'has-status': v.status,
                  }"
                  :style="{ '--card-delay': `${(gi * 80) + (i * 50)}ms` }"
                  @click="selectedVersion = v.label; showVersionPicker = false"
                >
                  <span v-if="v.status" class="version-status" :class="`status-${v.status}`">
                    {{ t(`conversion.versionStatus.${v.status}`) }}
                  </span>
                  <div class="version-card-main">
                    <span class="version-card-label">{{ v.label }}</span>
                    <span class="version-card-meta">
                      {{ t('conversion.packFormat', { n: v.packFormat }) }}
                    </span>
                  </div>
                  <i v-if="v.label === selectedVersion" class="ri-check-line version-card-check" aria-hidden="true"></i>
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
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
import { ref, computed, onMounted, onUnmounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { open, save } from '@tauri-apps/plugin-dialog';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useNotification } from '../composables/useNotification';

const emit = defineEmits(['switch-page']);
const { t } = useI18n();
const { notify } = useNotification();

const importMode = ref<'file' | 'folder'>('file');
const isDragging = ref(false);
let dragUnlisten: (() => void) | null = null;
const isConverting = ref(false);
const showProgress = ref(false);
const progressPercentage = ref(0);
const progressText = ref(t('conversion.ready'));
const progressBarPercentage = computed(() => {
  const raw = Number(progressPercentage.value);
  if (!Number.isFinite(raw)) return 0;
  return Math.min(100, Math.max(0, raw));
});
const progressLabel = computed(() => Math.round(progressBarPercentage.value));
const selectedVersion = ref('1.21-1.21.1');
const fixAlphaLayers = ref(false);
const selectedItems = ref<any[]>([]);
const showResultModal = ref(false);
const conversionResults = ref<any[]>([]);
const logMessages = ref<string[]>([]);
const outputMode = ref<'follow' | 'fixed'>('follow');
const outputPath = ref('');
const showVersionPicker = ref(false);
const showItemsDialog = ref(false);
const previewLimit = 3;
let fakeProgressTimer: ReturnType<typeof setInterval> | null = null;

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

const progressMode = computed<'single' | 'batch'>(() => {
  const hasFolder = selectedItems.value.some(i => i.isDir);
  if (hasFolder) return 'batch';
  if (selectedItems.value.length > 1) return 'batch';
  return 'single';
});

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
  showProgress.value = true;
  progressPercentage.value = 0;
  progressText.value = t('conversion.engineStarting');
  logMessages.value = [];

  notify({
    title: t('conversion.convStartTitle'),
    body: t('conversion.convStartBody', { count: selectedItems.value.length }),
    type: 'info',
    source: 'conversion'
  });

  const mode = progressMode.value;
  if (fakeProgressTimer) {
    clearInterval(fakeProgressTimer);
    fakeProgressTimer = null;
  }

  const logPollingInterval = setInterval(async () => {
    try {
      const logs = await invoke<string>('get_logs', {});
      if (logs) {
        const newLines = logs.split('\n');
        const currentCount = logMessages.value.length;
        if (newLines.length > currentCount) {
          logMessages.value = newLines;
          for (let i = currentCount; i < newLines.length; i++) {
            const line = newLines[i];
            
            // Match the progress pattern: Progress: X/Y (P%) - TaskName/Extracting/Repacking/Committing textures
            const moduleProgressMatch = line.match(/.*Progress: (\d+)\/(\d+) \((\d+)%\)(?: - (.*))?$/);
            if (moduleProgressMatch) {
              const current = parseInt(moduleProgressMatch[1]);
              const total = parseInt(moduleProgressMatch[2]);
              const percentage = parseInt(moduleProgressMatch[3]);
              const taskInfo = (moduleProgressMatch[4] || '').trim();
              
              if (mode === 'batch') {
                // Batch: only process lines without task info (overall file progress)
                if (taskInfo) continue;
                progressText.value = t('conversion.batchProgress', { current, total });
                progressPercentage.value = Math.max(progressPercentage.value, percentage);
              } else {
                // Single: main progress from module execution (task count), I/O phases only a small share
                if (!taskInfo) continue;
                let mappedPercentage = percentage;
                if (taskInfo === 'Extracting') {
                  // 0-5%
                  mappedPercentage = Math.round(percentage * 0.05);
                  progressText.value = t('conversion.extracting', { percent: percentage });
                } else if (taskInfo === 'Committing textures') {
                  // 90-95%
                  mappedPercentage = Math.round(90 + (percentage * 0.05));
                  progressText.value = t('conversion.committingTextures', { percent: percentage });
                } else if (taskInfo === 'Repacking') {
                  // 95-100%
                  mappedPercentage = Math.round(95 + (percentage * 0.05));
                  progressText.value = t('conversion.repacking', { percent: percentage });
                } else {
                  // 5-90%: module progress
                  mappedPercentage = Math.round(5 + (percentage * 0.85));
                  progressText.value = t('conversion.moduleProgress', { current, total, task: taskInfo });
                }
                progressPercentage.value = Math.max(progressPercentage.value, mappedPercentage);
              }
            }
          }
        }
      }
    } catch {
      // ignore
    }
  }, 300);

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

  if (filePaths.length === 1 && mode === 'single') {
    // We now have real progress reporting for all phases (extract, convert, repack).
    // The fake progress should be very conservative and only move if real progress is slow.
    fakeProgressTimer = setInterval(() => {
      if (!isConverting.value) return;
      const current = progressPercentage.value;
      if (current >= 95) return;
      // Only bump slowly if it's been a while without real progress
      const bump = 0.1;
      progressPercentage.value = Math.min(95, current + bump);
    }, 500);
  }

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
    progressPercentage.value = 100;
    const totalCount = conversionResults.value.length || selectedItems.value.length;
    progressText.value = t('conversion.convEnd', { success: successCount, total: totalCount });

    notify({
      title: t('conversion.convCompleteTitle'),
      body: t('conversion.convCompleteBody', { count: successCount }),
      type: 'success',
      source: 'conversion'
    });
  } catch (err) {
    conversionError = true;
    progressPercentage.value = 100;
    progressText.value = t('conversion.severeError');
    conversionResults.value = [];
    notify({
      title: t('conversion.severeErrorTitle'),
      body: t('conversion.severeErrorBody'),
      type: 'error',
      source: 'conversion'
    });
  } finally {
    clearInterval(logPollingInterval);
    if (fakeProgressTimer) {
      clearInterval(fakeProgressTimer);
      fakeProgressTimer = null;
    }
    isConverting.value = false;
    progressPercentage.value = 100;
    if (!conversionError) {
      const failureCount = conversionResults.value.filter(r => r.status !== 'success').length;
      const successCount = conversionResults.value.filter(r => r.status === 'success').length;
      if (failureCount === 0) {
        showResultModal.value = false;
        const totalCount = conversionResults.value.length || selectedItems.value.length;
        notify({
          title: t('conversion.convCompleteTitle'),
          body: t('conversion.convAllSuccessBody', { success: successCount, total: totalCount }),
          type: 'success',
          source: 'conversion'
        });
      } else {
        showResultModal.value = true;
      }
    }
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
  background: linear-gradient(180deg, #ffffff 0%, #f7f9ff 100%);
  color: #1d1d1f;
  overflow: hidden;
  position: relative;
  display: flex;
  flex-direction: column;
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", "PingFang SC", "Microsoft YaHei", sans-serif;
  background-size: 200% 200%;
  animation: bg-breathe 18s ease-in-out infinite;
}
.conversion-page::before {
  content: "";
  position: fixed;
  inset: 0;
  background:
    linear-gradient(135deg, rgba(120,160,255,0.04) 0%, rgba(120,160,255,0.01) 50%, transparent 100%),
    radial-gradient(55vw 45vh at 15% 20%, rgba(120,160,255,0.18), transparent 60%),
    radial-gradient(45vw 40vh at 85% 25%, rgba(120,160,255,0.14), transparent 55%),
    radial-gradient(60vw 55vh at 50% 80%, rgba(120,160,255,0.11), transparent 65%);
  opacity: 0.85;
  filter: blur(12px);
  animation: aurora-drift 26s ease-in-out infinite;
  pointer-events: none;
  z-index: 0;
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
.back-button {
  background: rgba(0, 0, 0, 0.05);
  border: none;
  padding: 10px 18px;
  border-radius: 14px;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  gap: 8px;
  font-weight: 600;
  transition: 0.3s;
}
.back-button:hover { background: rgba(0, 0, 0, 0.1); transform: translateX(-4px); }
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

.progress-section { margin-top: 4px; }
.progress-info { display: flex; justify-content: space-between; font-size: 12px; color: #6b7280; margin-bottom: 8px; }
.progress-bar-container { height: 8px; border-radius: 999px; background: rgba(0,0,0,0.06); overflow: hidden; }
.progress-bar { height: 100%; background: linear-gradient(90deg, var(--theme-color), color-mix(in srgb, var(--theme-color) 60%, #ffffff)); transition: width 0.3s ease; }
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

.dialog-overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.4); display: flex; align-items: center; justify-content: center; z-index: 100; }
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

/* === Version picker dialog (era-grouped cards) === */
.version-dialog {
  width: min(720px, 92vw);
  max-height: min(80vh, 720px);
  padding: 1.5rem 1.75rem 1.75rem;
  display: flex;
  flex-direction: column;
}
.version-dialog .dialog-header { margin-bottom: 1rem; }
.version-dialog-body {
  display: flex;
  flex-direction: column;
  gap: 1.1rem;
  overflow-y: auto;
  padding: 4px 4px 4px 0;
  margin-right: -4px;
}
.version-dialog-body::-webkit-scrollbar { width: 6px; }
.version-dialog-body::-webkit-scrollbar-thumb {
  background: rgba(0,0,0,0.12);
  border-radius: 3px;
}
.version-dialog-body::-webkit-scrollbar-thumb:hover { background: rgba(0,0,0,0.22); }

.version-era {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.version-era-header {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  padding: 0 2px;
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

.version-cards {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
  gap: 8px;
}

.version-card {
  position: relative;
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 10px 12px;
  border-radius: 12px;
  border: 1px solid rgba(0,0,0,0.06);
  background: rgba(255,255,255,0.85);
  font-family: inherit;
  cursor: pointer;
  text-align: left;
  overflow: hidden;
  transition:
    transform 0.18s cubic-bezier(.4,0,.2,1),
    border-color 0.18s ease,
    background 0.18s ease,
    box-shadow 0.18s ease;
  /* stagger entrance */
  animation: version-card-in 0.42s cubic-bezier(.2,.65,.3,1) both;
  animation-delay: var(--card-delay, 0ms);
}
@keyframes version-card-in {
  from { opacity: 0; transform: translateY(8px) scale(0.96); }
  to   { opacity: 1; transform: translateY(0) scale(1); }
}
.version-card:hover {
  transform: translateY(-2px);
  border-color: color-mix(in srgb, var(--theme-color) 45%, transparent);
  background: color-mix(in srgb, var(--theme-color) 6%, #ffffff);
  box-shadow: 0 6px 14px color-mix(in srgb, var(--theme-color) 14%, transparent);
}
.version-card:active { transform: translateY(0); }
.version-card.active {
  border-color: color-mix(in srgb, var(--theme-color) 60%, transparent);
  background: color-mix(in srgb, var(--theme-color) 14%, #ffffff);
  box-shadow:
    0 0 0 1px color-mix(in srgb, var(--theme-color) 60%, transparent) inset,
    0 4px 12px color-mix(in srgb, var(--theme-color) 22%, transparent);
}
.version-card.has-status { padding-top: 24px; }
.version-card-main {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}
.version-card-label {
  font-size: 13px;
  font-weight: 700;
  color: #1d1d1f;
  letter-spacing: -0.01em;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.version-card.active .version-card-label {
  color: color-mix(in srgb, var(--theme-color) 90%, #000);
}
.version-card-meta {
  font-size: 10px;
  color: #94a3b8;
  font-weight: 600;
  font-variant-numeric: tabular-nums;
  letter-spacing: 0.02em;
}
.version-card-check {
  position: absolute;
  top: 8px;
  right: 8px;
  font-size: 14px;
  color: var(--theme-color);
  animation: version-check-pop 0.32s cubic-bezier(.34,1.56,.64,1);
}
@keyframes version-check-pop {
  from { opacity: 0; transform: scale(0.4); }
  to   { opacity: 1; transform: scale(1); }
}

/* === Version status badges === */
.version-status {
  position: absolute;
  top: 6px;
  left: 6px;
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

/* Dialog enter/leave. Vue's `<transition>` only listens for
   transitionend / animationend on the **root** element of the
   transition (here, `.dialog-overlay`). So the overlay itself must
   carry a real transition — even if it's just an opacity fade —
   otherwise Vue thinks the leave finished instantly and unmounts the
   whole tree on the same frame, killing the content's fade/scale and
   the version-card stagger exit.

   We size the overlay's transition (0.27s) to fit the longest
   leave path on the version cards (90ms reverse-stagger delay + 180ms
   per-card animation = 270ms) so the last card finishes before
   Vue tears the tree down. */
.dialog-pop-enter-active,
.dialog-pop-leave-active {
  transition: opacity 0.27s ease;
}
.dialog-pop-enter-from,
.dialog-pop-leave-to {
  opacity: 0;
}

/* The actual white card: enters with a slight lift + scale, exits the
   same way. This rides on top of the overlay fade. */
.dialog-pop-enter-active .dialog-content,
.dialog-pop-leave-active .dialog-content {
  transition: opacity 0.22s ease, transform 0.22s cubic-bezier(.4,0,.2,1);
}
.dialog-pop-enter-from .dialog-content,
.dialog-pop-leave-to .dialog-content {
  opacity: 0;
  transform: translateY(12px) scale(0.96);
}

/* Version-picker card stagger exit: last era's cards leave first, so
   the eye follows the collapse from the bottom-right back to the
   top-left. Delays kept under the 0.27s overlay budget. */
.dialog-pop-leave-active .version-card {
  animation: version-card-out 0.18s cubic-bezier(.4,0,.2,1) both;
}
.dialog-pop-leave-active .version-era:nth-child(1) .version-card { animation-delay: 90ms; }
.dialog-pop-leave-active .version-era:nth-child(2) .version-card { animation-delay: 70ms; }
.dialog-pop-leave-active .version-era:nth-child(3) .version-card { animation-delay: 50ms; }
.dialog-pop-leave-active .version-era:nth-child(4) .version-card { animation-delay: 30ms; }
.dialog-pop-leave-active .version-era:nth-child(5) .version-card { animation-delay: 15ms; }
.dialog-pop-leave-active .version-era:nth-child(6) .version-card { animation-delay:  0ms; }
@keyframes version-card-out {
  from { opacity: 1; transform: translateY(0)    scale(1); }
  to   { opacity: 0; transform: translateY(6px)  scale(0.96); }
}

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
