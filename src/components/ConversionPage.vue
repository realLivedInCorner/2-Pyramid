/* .dialog-overlay / .dialog-content 已统一到 shared.css —— 见那里的说明。
   页面级的定义会因 scoped 的 data-v 属性获得更高特异度，留着会盖掉全局那份。 */
<template>
  <div class="conversion-page">
    <!-- 全应用共用的舞台背景，见 shell/StageBackdrop.vue -->
    <StageBackdrop />

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

    <Workbench :steps="steps" :label="t('conversion.flowLabel')" @go="goToStep">
      <!-- ① 源：导入模式 + 拖放 + 队列。全部与「选什么来转」有关 -->
      <div v-if="currentStep === 'source'" class="stage-panel">
        <div class="stage-head">
          <h2 class="stage-title">{{ t('conversion.stepSource') }}</h2>
          <p class="stage-desc">{{ t('conversion.stepSourceDesc') }}</p>
        </div>

        <div class="import-tabs">
          <button class="tab-btn" :class="{ active: importMode === 'file' }" @click="importMode = 'file'">{{ t('conversion.fileMode') }}</button>
          <button class="tab-btn" :class="{ active: importMode === 'folder' }" @click="importMode = 'folder'">{{ t('conversion.folderMode') }}</button>
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
            <p v-if="dropHint" class="drop-hint">{{ dropHint }}</p>
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

        <div class="stage-actions">
          <p class="stage-hint">{{ t('conversion.dropHint') }}</p>
          <button class="primary-btn" :disabled="!hasItems" @click="goToStep('target')">
            {{ t('conversion.nextTarget') }}
            <i class="ri-arrow-right-line" aria-hidden="true"></i>
          </button>
        </div>
      </div>

      <!-- ② 目标：版本 + 两项修复开关 + 主操作。三者互相依赖，放同一屏 -->
      <div v-else-if="currentStep === 'target'" class="stage-panel">
        <div class="stage-head">
          <h2 class="stage-title">{{ t('conversion.stepTarget') }}</h2>
          <p class="stage-desc">{{ t('conversion.stepTargetDesc') }}</p>
        </div>

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

        <div class="option-list">
          <div class="option-row">
            <div class="option-info">
              <div class="option-title">
                {{ t('conversion.fixAlphaLabel') }}
                <span
                  class="help-icon"
                  :title="t('conversion.fixAlphaHelp')"
                  :aria-label="t('conversion.fixAlphaLabel')"
                >?</span>
              </div>
              <p class="option-hint">{{ t('conversion.fixAlphaHint') }}</p>
            </div>
            <label class="switch">
              <input type="checkbox" v-model="fixAlphaLayers" />
              <span class="slider"></span>
            </label>
          </div>

          <div class="option-row">
            <div class="option-info">
              <div class="option-title">
                {{ t('conversion.adaptShadersLabel') }}
                <span
                  class="help-icon"
                  :title="t('conversion.adaptShadersHelp')"
                  :aria-label="t('conversion.adaptShadersLabel')"
                >?</span>
                <span class="exp-tag">{{ t('common.experimental') }}</span>
              </div>
              <p class="option-hint">{{ t('conversion.adaptShadersHint') }}</p>
            </div>
            <label class="switch">
              <input type="checkbox" v-model="adaptShaders" />
              <span class="slider"></span>
            </label>
          </div>
        </div>

        <div class="stage-actions">
          <button class="ghost-btn" @click="goToStep('source')">
            <i class="ri-arrow-left-line" aria-hidden="true"></i>
            {{ t('conversion.prev') }}
          </button>
          <button class="primary-btn" :disabled="!hasItems || isConverting" @click="startConversion">
            <i class="ri-play-fill" aria-hidden="true"></i>
            {{ t('conversion.startConvert') }}
          </button>
        </div>
      </div>

      <!-- ③ 转换：进度只在这一步出现，不再是常驻空卡片 -->
      <div v-else-if="currentStep === 'run'" class="stage-panel">
        <div class="stage-head">
          <h2 class="stage-title">{{ runTitle }}</h2>
          <p class="stage-desc">{{ progressText }}</p>
        </div>

        <!--
          三阶段进度。
          重写原因：旧版用「9px 圆点 + box-shadow 光环 + ::after 勾」，光环和伪元素
          都不占布局，在 flex 行里会把文字挤到不可预期的位置，出现被裁的现象。
          现在标记是一个固定 18px 的盒子，状态全部由它自己的边框/底色表达，
          不依赖任何不占位的装饰；文字是普通块级元素，宽度自适应，不设截断。
        -->
        <ol class="run-steps">
          <li v-for="s in runSteps" :key="s.id" class="run-step" :class="s.state">
            <span class="run-mark" aria-hidden="true"></span>
            <span class="run-label">{{ s.label }}</span>
          </li>
        </ol>

        <div class="progress-section">
          <div class="progress-info">
            <i v-if="isConverting" class="ri-loader-4-line spin" aria-hidden="true"></i>
            <span class="status-text">{{ progressText }}</span>
          </div>
          <div v-if="manyFilesWarning" class="many-files-warning">
            <i class="ri-time-line" aria-hidden="true"></i>
            {{ t('conversion.manyFilesWarning') }}
          </div>
        </div>

        <div class="stage-actions">
          <button
            v-if="isConverting"
            class="cancel-btn"
            @click="cancelConversion"
            :disabled="isCancelling"
          >
            <i class="ri-close-circle-line" aria-hidden="true"></i>
            {{ isCancelling ? t('conversion.cancelling') : t('conversion.cancel') }}
          </button>
          <button v-else class="primary-btn" @click="goToStep('result')">
            {{ t('conversion.viewResult') }}
            <i class="ri-arrow-right-line" aria-hidden="true"></i>
          </button>
        </div>
      </div>

      <!-- ④ 结果：原来这份内容在弹窗里，现在成为流程的终点 -->
      <div v-else class="stage-panel">
        <div class="stage-head">
          <h2 class="stage-title">{{ t('conversion.stepResult') }}</h2>
          <p class="stage-desc">{{ resultSummary }}</p>
        </div>

        <div class="result-list">
          <div
            v-for="(r, i) in conversionResults"
            :key="i"
            class="result-row"
            :class="r.status"
          >
            <i
              class="result-icon"
              :class="r.status === 'success' ? 'ri-checkbox-circle-line' : r.status === 'cancelled' ? 'ri-indeterminate-circle-line' : 'ri-error-warning-line'"
              aria-hidden="true"
            ></i>
            <div class="result-body">
              <span class="result-name">{{ r.name || selectedItems[i]?.name || '' }}</span>
              <span v-if="r.output" class="result-path">{{ r.output }}</span>
              <span v-else-if="r.error" class="result-path error">{{ r.error }}</span>
            </div>
          </div>
        </div>

        <div class="stage-actions">
          <button class="ghost-btn" @click="goToStep('source')">
            <i class="ri-refresh-line" aria-hidden="true"></i>
            {{ t('conversion.runAgain') }}
          </button>
          <!-- 详情弹窗保留：里面有逐项错误、日志条数与导出日志，结果页只放摘要 -->
          <button class="ghost-btn" @click="showResultModal = true">
            <i class="ri-list-check" aria-hidden="true"></i>
            {{ t('conversion.viewDetails') }}
          </button>
          <button class="primary-btn" @click="openOutputFolder">
            <i class="ri-folder-open-line" aria-hidden="true"></i>
            {{ t('openBtn') }}
          </button>
        </div>
      </div>
    </Workbench>

    <BedrockWarnDialog v-model="showBedrockWarn" @confirm="confirmBedrockPick" />
    <Java263WarnDialog v-model="show263Warn" @confirm="confirm263Pick" />

    <ConversionResultDialog
      v-model="showResultModal"
      :results="conversionResults"
      :file-names="selectedItems.map((i) => i.name)"
      :log-count="logMessages.length"
      @open-output="openOutputFolder"
      @export-logs="exportLogsToFile"
    />

    <DeleteSourceDialog v-model="showDeleteSourceDialog" @confirm="confirmDeleteSource" />

    <VersionSidebar
      v-model="showVersionPicker"
      :selected="selectedVersion"
      @pick="onVersionPick"
    />

    <ItemsListDialog
      v-model="showItemsDialog"
      :items="selectedItems"
      @remove="removeItem"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { pickFiles, pickDirectory, pickSavePath } from '../composables/useFilePicker';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useNotification } from '../composables/useNotification';
import BedrockWarnDialog from './conversion/BedrockWarnDialog.vue';
import Java263WarnDialog from './conversion/Java263WarnDialog.vue';
import ConversionResultDialog from './conversion/ConversionResultDialog.vue';
import DeleteSourceDialog from './conversion/DeleteSourceDialog.vue';
import ItemsListDialog from './conversion/ItemsListDialog.vue';
import VersionSidebar from './conversion/VersionSidebar.vue';
import Workbench, { type WorkbenchStep } from './shell/Workbench.vue';
import StageBackdrop from './shell/StageBackdrop.vue';
import {
  MINECRAFT_VERSIONS,
  DEFAULT_VERSION_LABEL,
  type VersionEntry,
} from '../data/minecraftVersions';

const emit = defineEmits(['switch-page']);
const props = defineProps<{
  sourceHandling?: 'ask' | 'delete' | 'keep';
  openOutputAfterConvert?: boolean;
}>();
const { t } = useI18n();
const { notify, registerToastAction, unregisterToastAction } = useNotification();

/**
 * ── 转换流程状态（一次一步）─────────────────────────────────────────────
 *
 * 原来「导入模式 / 目标版本 / 执行进度 / 拖放区」四项铺在一个 2×2 网格里常驻显示，
 * 代价是两张卡长期是空的（导入模式只有一个切换、进度空闲时只有三个灰点），
 * 而主操作「开始转换」被放在最后一张卡底部，离它依赖的版本设置隔了半个屏幕。
 *
 * 现在同一时刻只呈现一步；步骤轨负责回答「我在哪、前面选了什么」。
 * 可进入性由 canEnter 统一裁决，轨道上的锁定态与跳转用的是同一份判断，
 * 不会出现「按钮能点但进去是空的」。
 */
const STEP_IDS = ['source', 'target', 'run', 'result'] as const;
type StepId = (typeof STEP_IDS)[number];
const currentStep = ref<StepId>('source');

/** 是否跑过一轮（用于决定 run / result 能否进入） */
const hasRunOnce = ref(false);

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
// 记住上次的转换设置：目标版本与 Alpha 修复开关持久化到
// localStorage，重启后恢复。标签合法性在 onMounted 校验（版本列表
// 更新后旧标签可能失效）。
const selectedVersion = ref(localStorage.getItem('conversion.lastVersion') ?? DEFAULT_VERSION_LABEL);
const fixAlphaLayers = ref(localStorage.getItem('conversion.fixAlphaLayers') === 'true');
// 实验：着色器适配默认开；关闭则原样保留 shaders（可能在新版本失效）
const adaptShaders = ref(localStorage.getItem('conversion.adaptShaders') !== 'false');
const selectedItems = ref<any[]>([]);
const showResultModal = ref(false);
const conversionResults = ref<any[]>([]);
const logMessages = ref<string[]>([]);
const outputMode = ref<'follow' | 'fixed'>('follow');
const outputPath = ref('');
const showVersionPicker = ref(false);
const showBedrockWarn = ref(false);
const show263Warn = ref(false);
const showItemsDialog = ref(false);
const previewLimit = 3;
// 拖入不支持的扩展名时的提示（当前 zip/mcpack 均接受）
const dropHint = ref('');
let dropHintTimer: ReturnType<typeof setTimeout> | null = null;

const versions = MINECRAFT_VERSIONS;

const selectedVersionEntry = computed(
  () => versions.find(v => v.label === selectedVersion.value) ?? versions[versions.length - 1]
);

/// 版本选择：Bedrock 未完成 / 26.3 有官方材质包目录 bug，选中时先弹警示确认
const onVersionPick = (v: VersionEntry) => {
  if (v.packFormat === 1000) {
    showVersionPicker.value = false;
    showBedrockWarn.value = true;
    return;
  }
  if (v.packFormat === 97) {
    showVersionPicker.value = false;
    show263Warn.value = true;
    return;
  }
  selectedVersion.value = v.label;
  showVersionPicker.value = false;
};

const confirmBedrockPick = () => {
  showBedrockWarn.value = false;
  selectedVersion.value = 'Bedrock Latest';
};

const confirm263Pick = () => {
  show263Warn.value = false;
  selectedVersion.value = '26.3';
};

const hasItems = computed(() => selectedItems.value.length > 0);
const previewItems = computed(() => selectedItems.value.slice(0, previewLimit));

// ── 流程：可进入性 ────────────────────────────────────────────────────
/** 某一步当前能否进入。轨道锁定态与跳转共用这一份判断。 */
const canEnter = (id: StepId): boolean => {
  switch (id) {
    case 'source':
      // 转换跑完之后不再允许回到输入步骤：改了源或目标，结果页展示的就不再是
      // 那次转换的输入，界面上会出现「结果对不上设置」的矛盾状态。
      return !hasRunOnce.value;
    case 'target':
      return hasItems.value && !hasRunOnce.value;
    case 'run':
      return hasRunOnce.value || isConverting.value;
    case 'result':
      return hasRunOnce.value && conversionResults.value.length > 0;
  }
};

const goToStep = (id: string) => {
  const step = id as StepId;
  if (!STEP_IDS.includes(step)) return;
  if (step !== currentStep.value && !canEnter(step)) return;
  currentStep.value = step;
};

/** 本次是否至少有一个成功（决定「输出完成」点亮） */
const hasSuccess = computed(() => conversionResults.value.some((r) => r?.status === 'success'));

/**
 * 三阶段进度。
 *
 * 状态规则（互斥，同一个阶段只有一个）：
 *   done   已经走过 —— 导入完成、转换跑完、输出成功
 *   active 正在进行 —— 只有「转换处理中」在 isConverting 时
 *   todo   还没到
 *
 * 之前是三个独立的 :class 表达式各写各的，容易出现「转换处理中 和 输出完成
 * 同时不亮」或「两个都亮」的组合。集中到一个 computed 里，顺序和互斥一目了然。
 */
const runSteps = computed(() => {
  const items = [
    { id: 'import', label: t('conversion.stepImport'), done: hasItems.value, active: false },
    { id: 'process', label: t('conversion.stepProcessing'), done: hasRunOnce.value, active: isConverting.value },
    { id: 'output', label: t('conversion.stepOutput'), done: hasSuccess.value, active: false },
  ];
  // 当前正在跑的那一步，如果同时也满足 done 条件，以 active 为准
  return items.map((it) => ({
    id: it.id,
    label: it.label,
    state: it.active ? 'active' : it.done ? 'done' : 'todo',
  }));
});

/**
 * 步骤三的标题要跟着状态走。
 * 原来写死 t('conversion.stepRun')＝「正在转换」，
 * 于是转换结束后标题说「正在转换」、副标题说「转换结束: 成功 1/1」，自相矛盾。
 */
const runTitle = computed(() => {
  if (isConverting.value) return t('conversion.stepRun');
  if (hasRunOnce.value) return hasSuccess.value ? t('conversion.stepRunDone') : t('conversion.stepRunFailed');
  return t('conversion.stepRun');
});

/** 结果一句话概括，供结果页副标题用。 */
const resultSummary = computed(() => {
  const list = conversionResults.value;
  if (!list.length) return t('conversion.ready');
  const ok = list.filter((r) => r?.status === 'success').length;
  const cancelled = list.filter((r) => r?.status === 'cancelled').length;
  if (cancelled > 0) return t('conversion.cancelledText');
  if (ok === list.length) return t('conversion.convAllSuccessBody', { success: ok, total: list.length });
  return t('conversion.resultPartial', { success: ok, total: list.length });
});

/**
 * 步骤轨数据。
 * summary 只在「已经越过这一步」时才有意义 —— 未到达的步骤不留摘要，
 * 否则轨道会被前瞻性的空文本撑开。
 */
const steps = computed<WorkbenchStep[]>(() => {
  const cur = STEP_IDS.indexOf(currentStep.value);
  const labels: Record<StepId, string> = {
    source: t('conversion.stepSourceShort'),
    target: t('conversion.stepTargetShort'),
    run: t('conversion.stepRunShort'),
    result: t('conversion.stepResultShort'),
  };
  const summaries: Record<StepId, string> = {
    source: hasItems.value ? t('conversion.summaryItems', { count: selectedItems.value.length }) : '',
    target: selectedVersionEntry.value?.label ?? '',
    run: hasRunOnce.value ? resultSummary.value : '',
    result: '',
  };
  return STEP_IDS.map((id, i) => {
    let state: WorkbenchStep['state'];
    if (i === cur) state = 'current';
    // 已越过的步骤若 canEnter 为假（例如转换完成后不允许回到输入），
    // 要标成 locked —— 否则它显示为可点的 done，点了却什么也不发生。
    else if (i < cur) state = canEnter(id) ? 'done' : 'locked';
    else state = canEnter(id) ? 'todo' : 'locked';
    return { id, label: labels[id], summary: i !== cur ? summaries[id] : '', state };
  });
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
  /*
   * 接住主页交接过来的文件。
   *
   * 主页的窗口级拖拽回调原先只 switch-page('conversion') 就把路径丢了 ——
   * 用户把包拖到主页，页面跳过去，队列却是空的。
   * 现在主页把路径写进 localStorage（与既有的 foray.pendingPath 同一个模式），
   * 这里取出来入队后立刻清掉，避免下次进页面重复导入。
   */
  try {
    const raw = localStorage.getItem('conversion.pendingPaths');
    if (raw) {
      localStorage.removeItem('conversion.pendingPaths');
      const parsed = JSON.parse(raw);
      // 兼容 { at, paths } 与旧的纯数组两种形态
      const paths: string[] = Array.isArray(parsed) ? parsed : (parsed?.paths ?? []);
      const at: number = Array.isArray(parsed) ? Date.now() : (parsed?.at ?? 0);
      // 超过 60s 视为失效：交接本该是「写入后立刻跳转并消费」，
      // 隔这么久说明中间出了岔子，不该把陈旧路径导进来。
      if (Date.now() - at >= 60_000) {
        console.warn('[conversion] 待转换路径已过期，忽略');
      } else {
        for (const p of paths) {
          const name = p.split(/[\\/]/).pop() || p;
          const ext = name.includes('.') ? (name.toLowerCase().split('.').pop() || '') : '';
          const isDir = ext === '';
          // 与拖入时的过滤保持一致：文件模式只收 zip / mcpack
          if (!isDir && ext !== 'zip' && ext !== 'mcpack') continue;
          if (selectedItems.value.some((item) => item.path === p)) continue;
          selectedItems.value.push({ name, size: isDir ? 'Folder' : '', path: p, isDir });
        }
      }
    }
  } catch (e) {
    console.warn('[conversion] 读取待转换路径失败', e);
  }

  // 校验持久化的版本标签仍存在（版本列表更新后旧标签可能失效），
  // 失效则回退到默认目标版本。
  if (!versions.some(v => v.label === selectedVersion.value)) {
    selectedVersion.value = DEFAULT_VERSION_LABEL;
  }

  // 记住上次的转换设置（目标版本 / Alpha 修复开关）
  watch(selectedVersion, (v) => {
    localStorage.setItem('conversion.lastVersion', v);
  });
  watch(fixAlphaLayers, (v) => {
    localStorage.setItem('conversion.fixAlphaLayers', String(v));
  });
  watch(adaptShaders, (v) => {
    localStorage.setItem('conversion.adaptShaders', String(v));
  });

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
        if (localStorage.getItem('editorMode') === 'true') {
          localStorage.setItem('foray.pendingPath', paths[0]);
          emit('switch-page', 'foray');
          return;
        }
        for (const p of paths) {
          const fileName = p.split(/[\\/]/).pop() || p;
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
  if (dropHintTimer) clearTimeout(dropHintTimer);
  window.removeEventListener('keydown', handleVersionPickerKey);
  // Tear down the toast action handlers we registered on mount so a
  // later conversion (or page) doesn't accidentally fire them.
  unregisterToastAction('conv:open-output');
});

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
      (document.querySelector('.version-sidebar') as HTMLElement | null)?.focus?.();
    }, 50);
  } else {
    window.removeEventListener('keydown', handleVersionPickerKey);
  }
});

const triggerPicker = async () => {
  try {
    // 应用内浏览器。原来是一个调用同时表达两种模式（directory 开关），
    // 这里拆成两个入口 —— 组件本身也是按模式分岔的，接得上。
    const isFolder = importMode.value === 'folder';
    const result = isFolder
      ? await pickDirectory({ multiple: true })
      // 'zip','mcpack' 之外还放 '*'：原生用第二个过滤器组表达「所有文件」，
      // 这里一个扁平列表，含通配符就等于不做扩展名限制（见 FileBrowser 的注释）。
      : await pickFiles({
          multiple: true,
          startDir: null,
          extensions: ['zip', 'mcpack', '*'],
        });

    if (result) {
      const paths = Array.isArray(result) ? result : [result];
      for (const path of paths) {
        const fileName = path.split(/[\\/]/).pop() || path;
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
  // 进入流程第 ③ 步。hasRunOnce 同时解锁轨道上的「结果」。
  hasRunOnce.value = true;
  goToStep('run');

  // Warn when converting a lot of files at once.
  manyFilesWarning.value = selectedItems.value.length >= MANY_FILES_THRESHOLD;

  // NOTE: deliberately no toast here for "conversion started". Creating
  // a top-level toast means spawning a fresh WebView2 window, and doing
  // that while the conversion saturates the CPU stalls the main thread
  // (dead window buttons / frozen dialogs). The progress panel below
  // already shows the running state; the completion toast is fired
  // AFTER the batch returns, when the CPU is idle again.


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
      adaptShaders: adaptShaders.value,
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
        goToStep('result');
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
        // 成功也进入结果页 —— 流程有一个明确的终点，而不是停在进度页
        goToStep('result');
      } else {
        goToStep('result');
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
 *   * "ask"    → custom-drawn confirm dialog (NOT the OS-native
 *                plugin-dialog `ask` and NOT `window.confirm`, which
 *                blocks the WebView2 JS main thread)
 *   * "delete" → delete unconditionally
 *   * "keep"   → no-op
 */
const showDeleteSourceDialog = ref(false);

const applySourceHandling = (): void => {
  const policy = props.sourceHandling ?? 'ask';
  if (policy === 'keep') return;
  if (policy === 'delete') {
    void deleteSourcePacks();
    return;
  }
  showDeleteSourceDialog.value = true;
};

/// User confirmed the source-pack deletion on the custom dialog.
const confirmDeleteSource = (): void => {
  showDeleteSourceDialog.value = false;
  void deleteSourcePacks();
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
    const savePath = await pickSavePath({
      // 'log' 之外放 '*'：原生第二个过滤器组是「所有文件」，这里是同名替代
      extensions: ['log', '*'],
      defaultName: `conversion_log_${new Date().toISOString().replace(/[:.]/g, '-')}.log`,
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
  color: var(--ink);
  overflow: hidden;
  position: relative;
  display: flex;
  flex-direction: column;
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", "PingFang SC", "Microsoft YaHei", sans-serif;
}

.header-section {
  z-index: 5;
  padding: 28px 40px 10px;
  display: flex;
  align-items: center;
  gap: 16px;
  flex-shrink: 0;
}
.title-group { display: flex; flex-direction: column; min-width: 0; }
.page-title {
  font-size: 26px;
  font-weight: 800;
  letter-spacing: -0.6px;
  margin: 0;
  color: var(--ink);
}
.page-subtitle { margin: 4px 0 0; color: var(--ink-3); font-size: 13px; }

.switch-title {
  display: inline-flex;
  align-items: center;
  gap: 8px;
}
.exp-tag {
  font-size: 10px;
  font-weight: 800;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  padding: 2px 6px;
  border-radius: 999px;
  color: var(--warn);
  background: var(--warn-tint);
  border: 1px solid var(--warn-line);
}
.help-icon {
  width: 18px;
  height: 18px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: 999px;
  font-size: 11px;
  font-weight: 800;
  color: var(--ink-3);
  background: rgba(var(--ink-rgb), 0.05);
  cursor: help;
  user-select: none;
}


/* 卡片：交给 App.vue 全局玻璃皮肤，只补布局 */
.card-hint { font-size: 12px; color: var(--ink-4); margin: 0; flex-shrink: 0; }


/* 文件 / 文件夹：对齐设置页 segmented */
.import-tabs {
  display: inline-flex;
  background: rgba(var(--ink-rgb), 0.05);
  border-radius: calc(var(--ui-radius-btn) + 4px);
  padding: 3px;
  gap: 3px;
  align-self: flex-start;
}
.tab-btn {
  border: none;
  background: transparent;
  padding: 8px 16px;
  border-radius: 6px;
  font-size: 13px;
  font-weight: 500;
  color: var(--ink-3);
  cursor: pointer;
  transition: color 0.18s ease, background 0.18s ease, box-shadow 0.18s ease;
  font-family: inherit;
}
.tab-btn:hover { color: var(--ink); }
.tab-btn.active {
  background: var(--surface);
  color: var(--ink);
  /* 去掉两层投影（环境影 + 1px 描边影）—— 那是卡片语言的遗留。
     设置页的分段按钮已经统一成「白面 + 发丝内描边」，这里跟上。 */
  box-shadow: inset 0 0 0 1px rgba(var(--ink-rgb), 0.12);
}

/* === Version picker === */
.version-picker {
  display: inline-flex;
  align-items: center;
  gap: 14px;
  /* 发丝线 + 暖白面，不用主题色描边和内高光 —— 与设置页同一套 */
  background: var(--panel-3);
  border: 1px solid rgba(var(--warm-rgb), 0.2);
  padding: 12px 14px;
  border-radius: 8px;
  font-family: inherit;
  cursor: pointer;
  transition: border-color 0.2s ease, background 0.2s ease;
  box-shadow: none;
  min-width: 0;
  width: 100%;
  text-align: left;
}
.version-picker:hover {
  border-color: color-mix(in srgb, var(--accent) 38%, transparent);
  box-shadow: 0 4px 16px rgba(var(--ink-rgb), 0.12);
  transform: translateY(-1px);
}
.version-picker:active { transform: translateY(0); }
.version-picker.open {
  border-color: color-mix(in srgb, var(--accent) 50%, transparent);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 16%, transparent);
}
.version-picker-main {
  display: flex;
  flex-direction: column;
  gap: 2px;
  flex: 1;
  min-width: 0;
}
.version-picker-label {
  font-size: 15px;
  font-weight: 700;
  color: var(--ink);
  letter-spacing: -0.01em;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.version-picker-range {
  font-size: 11px;
  color: var(--ink-3);
  font-weight: 500;
  white-space: nowrap;
}
.version-picker-meta {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
}
.version-picker-format {
  font-size: 11px;
  font-weight: 700;
  color: color-mix(in srgb, var(--accent) 80%, #000);
  background: color-mix(in srgb, var(--accent) 12%, transparent);
  padding: 3px 8px;
  border-radius: 999px;
  font-variant-numeric: tabular-nums;
}
.version-picker-chevron {
  font-size: 18px;
  color: var(--ink-4);
  transition: transform 0.22s ease, color 0.18s ease;
}
.version-picker:hover .version-picker-chevron { color: var(--ink); }
.version-picker.open .version-picker-chevron { transform: rotate(180deg); color: var(--ink); }

.switch-line {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  font-size: 13px;
  color: var(--ink-2);
  padding-top: 2px;
}
.switch { position: relative; display: inline-block; width: 42px; height: 24px; flex-shrink: 0; }
.switch input { opacity: 0; width: 0; height: 0; }
.slider {
  position: absolute; inset: 0; cursor: pointer;
  background-color: var(--surface-5);
  transition: background-color 200ms cubic-bezier(0.4, 0, 0.2, 1);
  border-radius: 34px;
}
.slider:before {
  position: absolute; content: "";
  height: 18px; width: 18px; left: 3px; bottom: 3px;
  background-color: white;
  transition: transform 200ms cubic-bezier(0.4, 0, 0.2, 1);
  border-radius: 50%;
  box-shadow: 0 2px 4px rgba(var(--ink-rgb), 0.12);
}
.switch input:checked + .slider { background-color: var(--ink); }
.switch input:checked + .slider:before { transform: translateX(18px); }

/* 步骤：更轻的竖向时间线 */
/*
 * 三阶段进度。
 *
 * 每行 = 一个固定 18px 的标记盒 + 一段普通文字。
 * 标记的状态全部由它自己的边框和底色表达 —— 不用 box-shadow 光环、
 * 不用 ::after 勾：那两者都不占布局，在 flex 行里会把文字挤到不可预期的位置。
 * 文字是普通块级元素，宽度自适应，任何地方都不设 overflow / text-overflow。
 */
.run-steps {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  gap: 14px;
  /* 宽度按内容收缩，不拉满整行 —— 拉满后文字区域变宽，反而不好读 */
  width: fit-content;
}

.run-step {
  display: flex;
  align-items: center;
  gap: 12px;
  font-size: 13px;
  line-height: 1.5;
  color: var(--ink-4);
  transition: color 0.2s ease;
}

/* 标记：实心盒子，占据真实布局宽度 */
.run-mark {
  flex: 0 0 18px;
  width: 18px;
  height: 18px;
  border-radius: 50%;
  box-sizing: border-box;
  border: 1px solid rgba(var(--warm-rgb), 0.3);
  background: transparent;
  transition: background 0.2s ease, border-color 0.2s ease;
}

.run-label { white-space: nowrap; }

/* 进行中：深色描边 + 深色内芯 */
.run-step.active { color: var(--ink); font-weight: 600; }
.run-step.active .run-mark {
  border-color: var(--ink);
  background: radial-gradient(circle, var(--ink) 0 4px, transparent 4px);
}

/* 已完成：整体填深色 */
.run-step.done { color: var(--ink-2); }
.run-step.done .run-mark {
  border-color: var(--ink);
  background: var(--ink);
}

@media (prefers-reduced-motion: reduce) {
  .run-step,
  .run-mark { transition: none; }
}

.progress-section { margin-top: 4px; display: flex; flex-direction: column; gap: 12px; }
.progress-info { display: flex; align-items: center; gap: 8px; font-size: 13px; color: var(--ink-2); }
.progress-info .spin { color: var(--ink); font-size: 16px; }
.status-text { font-weight: 600; }

.many-files-warning {
  display: flex; align-items: center; gap: 8px;
  font-size: 12px; color: var(--warn);
  background: color-mix(in srgb, var(--warn) 5%, transparent);
  border: 1px solid color-mix(in srgb, var(--warn) 22%, transparent);
  border-radius: 8px; padding: 8px 12px;
}
.many-files-warning i { font-size: 14px; }

.cancel-btn {
  display: inline-flex; align-items: center; gap: 6px;
  padding: 8px 14px;
  font-size: 12px; font-weight: 600; font-family: inherit;
  color: var(--danger);
  background: var(--surface);
  border: 1px solid color-mix(in srgb, var(--danger) 18%, transparent);
  border-radius: var(--ui-radius-btn);
  cursor: pointer;
  align-self: flex-start;
  transition: background 0.18s ease, border-color 0.18s ease;
}
.cancel-btn:hover:not(:disabled) { background: color-mix(in srgb, var(--danger) 6%, transparent); border-color: color-mix(in srgb, var(--danger) 35%, transparent); }
.cancel-btn:disabled { opacity: 0.55; cursor: not-allowed; }

@keyframes spin { to { transform: rotate(360deg); } }
.spin { display: inline-block; animation: spin 1s linear infinite; }
.idle-state { color: var(--ink-4); font-size: 12px; padding: 8px 0; }

.drop-header { display: flex; align-items: center; justify-content: space-between; }
.drop-title { font-size: 13px; font-weight: 700; color: var(--ink-3); letter-spacing: 0.04em; text-transform: uppercase; }

/* 拖放区：更安静、主题色描边 */
.drag-drop-frame {
  border: 1.5px dashed color-mix(in srgb, var(--ink) 18%, transparent);
  border-radius: calc(var(--ui-radius-card) + 4px);
  padding: 28px 20px;
  text-align: center;
  background: var(--panel-2);
  transition: border-color 0.2s ease, background 0.2s ease, transform 0.2s ease, box-shadow 0.2s ease;
  cursor: pointer;
  flex-shrink: 0;
}
.drag-drop-frame:hover {
  border-color: color-mix(in srgb, var(--accent) 40%, transparent);
  background: color-mix(in srgb, var(--accent) 4%, var(--panel-2));
}
.drag-drop-frame.is-dragover {
  border-color: var(--accent);
  border-style: solid;
  background: color-mix(in srgb, var(--accent) 8%, transparent);
  transform: scale(1.01);
  box-shadow: 0 0 0 4px color-mix(in srgb, var(--accent) 12%, transparent);
}
.drag-drop-frame.has-file {
  border-style: dashed;
  border-color: color-mix(in srgb, var(--ok) 35%, transparent);
}
.drop-icon-container { display: flex; align-items: center; justify-content: center; margin-bottom: 10px; }
.drop-icon {
  font-size: 34px;
  color: color-mix(in srgb, var(--accent) 70%, var(--ink));
  line-height: 1;
}
.drop-icon.success { color: var(--ok); }
.drop-text h3 { margin: 0; font-size: 14px; font-weight: 700; color: var(--ink); }
.drop-text p { margin: 6px 0 0; color: var(--ink-4); font-size: 12px; }
.drop-hint { color: var(--warn) !important; font-weight: 700; }
.bedrock-warn-text { color: var(--danger); font-weight: 600; line-height: 1.7; }

.selected-items-list {
  background: var(--panel-3);
  border-radius: calc(var(--ui-radius-card) + 2px);
  border: 1px solid rgba(var(--ink-rgb), 0.05);
  overflow: hidden;
  min-height: 0;
  display: flex;
  flex-direction: column;
}
.list-header {
  display: flex; align-items: center; justify-content: space-between;
  padding: 10px 14px; font-size: 12px; color: var(--ink-3);
  background: rgba(var(--ink-rgb), 0.03);
  flex-shrink: 0;
}
.list-actions { display: flex; align-items: center; gap: 8px; }
.view-all-btn {
  border: none;
  background: color-mix(in srgb, var(--accent) 12%, transparent);
  color: color-mix(in srgb, var(--accent) 85%, #000);
  font-size: 12px;
  font-weight: 700;
  padding: 4px 10px;
  border-radius: 8px;
  cursor: pointer;
  font-family: inherit;
  transition: background 0.15s ease;
}
.view-all-btn:hover { background: color-mix(in srgb, var(--accent) 20%, transparent); }
.clear-btn {
  border: none;
  background: transparent;
  color: var(--danger);
  font-weight: 600;
  font-size: 12px;
  cursor: pointer;
  font-family: inherit;
}
.items-scroll { max-height: 150px; overflow-y: auto; }
.item-row {
  display: flex; align-items: center; justify-content: space-between;
  padding: 10px 14px;
  border-top: 1px solid rgba(var(--ink-rgb), 0.05);
  transition: background 0.15s ease;
}
.item-row:hover { background: rgba(var(--ink-rgb), 0.03); }
.item-info { display: flex; align-items: center; gap: 10px; min-width: 0; }
.item-icon { width: 18px; height: 18px; color: var(--ink); display: inline-flex; align-items: center; justify-content: center; }
.item-icon i { font-size: 18px; line-height: 1; }
.item-name {
  font-size: 13px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.item-actions { display: flex; align-items: center; gap: 10px; flex-shrink: 0; }
.item-size { font-size: 12px; color: var(--ink-4); }
.remove-item-btn {
  border: none;
  background: rgba(var(--ink-rgb), 0.05);
  width: 22px; height: 22px;
  border-radius: 999px;
  cursor: pointer;
  font-size: 14px;
  line-height: 1;
  color: var(--ink-3);
  transition: background 0.15s ease, color 0.15s ease;
}
.remove-item-btn:hover { background: color-mix(in srgb, var(--danger) 16%, transparent); color: var(--danger); }
.more-row {
  padding: 10px 14px;
  border-top: 1px solid rgba(var(--ink-rgb), 0.05);
  color: var(--ink-3);
  font-size: 12px;
}

.action-bar { margin-top: auto; padding-top: 4px; }
.start-conversion-button {
  width: 100%;
  border: none;
  background: var(--accent);
  color: var(--surface);
  padding: 13px 16px;
  border-radius: var(--ui-radius-btn);
  font-weight: 700;
  font-size: 14px;
  font-family: inherit;
  cursor: pointer;
  transition: background 0.18s ease, transform 0.15s ease, box-shadow 0.18s ease, opacity 0.18s ease;
  box-shadow: 0 6px 18px color-mix(in srgb, var(--accent) 28%, transparent);
}
.start-conversion-button:disabled {
  opacity: 0.45;
  cursor: not-allowed;
  box-shadow: none;
}
.start-conversion-button:hover:not(:disabled) {
  background: color-mix(in srgb, var(--accent) 88%, #000);
  transform: translateY(-1px);
}
.start-conversion-button:active:not(:disabled) { transform: translateY(0); }

.dialog-header { display: flex; justify-content: space-between; margin-bottom: 1rem; }
.dialog-close { border: none; background: transparent; font-size: 20px; cursor: pointer; color: var(--ink-3); }
.dialog-body { margin-bottom: 1.5rem; }
.result-summary { margin-bottom: 1rem; }
.result-item { display: flex; justify-content: space-between; margin-bottom: 0.5rem; padding: 0.5rem; border-radius: 8px; background: var(--surface-2); }
.result-value { font-weight: 700; }
.result-value.success { color: var(--ok); }
.result-value.error { color: var(--danger); }
.error-details { margin-top: 1rem; padding: 1rem; border-radius: 8px; background: color-mix(in srgb, var(--danger) 6%, transparent); border: 1px solid color-mix(in srgb, var(--danger) 16%, transparent); }
.error-details h4 { margin-top: 0; color: var(--danger); font-size: 1rem; }
.error-list { margin: 0.5rem 0 0 0; padding-left: 1.5rem; }
.error-list li { margin-bottom: 0.25rem; color: var(--danger); font-size: 0.9rem; }
.dialog-footer { display: flex; gap: 12px; justify-content: flex-end; margin-top: 2rem; }
.dialog-button { padding: 8px 20px; border-radius: 10px; border: none; background: var(--accent); color: white; font-weight: 600; cursor: pointer; transition: all 0.2s; }
.dialog-button:hover:not(:disabled) { background: color-mix(in srgb, var(--accent) 85%, #000); transform: translateY(-1px); }
.dialog-button.secondary { background: var(--surface-3); color: var(--ink-2); }
.dialog-button.secondary:hover { background: var(--surface-5); }
.dialog-button.danger { background: var(--danger); }
.dialog-button.danger:hover:not(:disabled) { background: var(--danger); }
.dialog-button:disabled { background: var(--surface-6); cursor: not-allowed; }

/* === Version picker sidebar (右侧抽屉,从右滑入) === */
/* .sidebar-overlay 已统一到 shared.css —— 见那里的说明。
   页面级定义会因 scoped 的 data-v 获得更高特异度，留着会盖掉全局那份。 */
.sidebar-content {
  /* position: fixed + right: 0 — aside 跟 .sidebar-overlay 是 sibling,
     不嵌套在 overlay flex 容器里。aside 自己用 right:0 锚定在屏幕右侧,
     transform translateX(100%) = + 自身 420px width 推到屏幕外右侧;
     transform translateX(0) = 回到 right:0 位置(屏幕内最右 420px)。 */
  /* 几何、底色、边框、flex 都来自 shared.css 的 .sidebar-content。
     这里只留真正有差异的三项，以及下面那段 transform 兜底。 */
  height: 100vh;
  z-index: 201;
  outline: none;
  /* 投影由 useSidebarSlide 在 enter/leave 期间用内联样式维护，这里只是静态兜底。
     颜色改暖（原来是冷黑 rgba(var(--ink-rgb), 0.08)，压在暖白界面上会发脏）。 */
  box-shadow: -12px 0 36px rgba(var(--ink-rgb), 0.12);
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
  border-bottom: 1px solid rgba(var(--ink-rgb), 0.05);
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
  color: var(--ink);
  letter-spacing: -0.02em;
}
.sidebar-hint {
  margin: 0;
  font-size: 12px;
  color: var(--ink-3);
  font-weight: 500;
  font-variant-numeric: tabular-nums;
}
.sidebar-close {
  width: 32px;
  height: 32px;
  flex-shrink: 0;
  border: none;
  background: transparent;
  color: var(--ink-3);
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
  background: rgba(var(--ink-rgb), 0.05);
  color: var(--ink);
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
  background: rgba(var(--ink-rgb), 0.12);
  border-radius: 3px;
}
.sidebar-body::-webkit-scrollbar-thumb:hover { background: rgba(var(--ink-rgb), 0.18); }

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
  color: var(--ink-3);
}
.version-era-count {
  font-size: 10px;
  color: var(--ink-4);
  font-weight: 600;
  font-variant-numeric: tabular-nums;
}


@keyframes version-row-in {
  from { opacity: 0; transform: translateX(8px); }
  to   { opacity: 1; transform: translateX(0); }
}
.version-row:active {
  background: rgba(var(--ink-rgb), 0.05);
}
.version-row.has-status { padding-right: 8px; }

.version-row-label {
  font-size: 14px;
  font-weight: 600;
  color: var(--ink);
  letter-spacing: -0.01em;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.version-row-tail {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
}
.version-row-check {
  font-size: 16px;
  color: var(--accent);
  animation: version-check-pop 200ms cubic-bezier(0.22, 1, 0.36, 1);
}
@keyframes version-check-pop {
  from { opacity: 0; transform: scale(0.85); }
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
  color: var(--surface);
  background: linear-gradient(135deg, var(--accent), color-mix(in srgb, var(--accent) 65%, #000));
  box-shadow: 0 1px 3px color-mix(in srgb, var(--accent) 40%, transparent);
}
.status-stable {
  color: color-mix(in srgb, var(--accent) 85%, #000);
  background: color-mix(in srgb, var(--accent) 12%, transparent);
  border: 1px solid color-mix(in srgb, var(--accent) 30%, transparent);
}

.items-dialog {
  width: min(860px, 92vw);
}
.items-dialog-list {
  max-height: 420px;
  overflow: auto;
  border: 1px solid rgba(var(--ink-rgb), 0.05);
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
  .header-section { padding: 20px 20px 8px; }
}
@media (min-width: 981px) and (max-width: 1200px) {
  .header-section { padding: 24px 28px 8px; }
}
@media (min-width: 1600px) {
  .header-section { padding: 28px 64px 8px; }
}
/*
 * 拖放区与队列按可用空间伸缩。
 * 原实现里拖放区 flex-shrink: 0、队列 max-height: 150px，都是为「卡片里的一小块」
 * 调的；搬到整屏舞台后会在下方留出大片空白。
 * 现在：未导入时拖放区吃掉剩余空间（内容居中），导入后缩成一条、由队列接管。
 */
.stage-panel > .drag-drop-frame {
  flex: 1 1 auto;
  min-height: 168px;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
}
.stage-panel > .drag-drop-frame.has-file {
  flex: 0 0 auto;
  min-height: 0;
  padding: 18px 20px;
}
.stage-panel .selected-items-list { flex: 1 1 auto; min-height: 0; }
.stage-panel .items-scroll { max-height: none; flex: 1 1 auto; min-height: 0; }
/* ── 修复项：一行一项，横线分隔 ── */
.option-list { display: flex; flex-direction: column; flex-shrink: 0; }
.option-row {
  display: flex;
  align-items: flex-start;
  gap: 16px;
  padding: 14px 0;
  border-bottom: 1px solid var(--border-color);
}
.option-row:last-child { border-bottom: none; }
.option-info { flex: 1; min-width: 0; }
.option-title {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 14px;
  font-weight: 600;
  color: var(--ink);
}
.option-hint { margin: 4px 0 0; font-size: 12px; color: var(--ink-3); line-height: 1.5; }

/* ── 结果列表 ── */
.result-list { display: flex; flex-direction: column; gap: 8px; flex-shrink: 0; }
.result-row {
  display: flex;
  gap: 10px;
  padding: 10px 12px;
  border-radius: var(--radius-small);
  background: var(--hover-color);
}
.result-icon { font-size: 16px; line-height: 1.45; flex-shrink: 0; }
.result-row.success .result-icon { color: var(--ok); }
.result-row.error .result-icon { color: var(--danger); }
.result-row.cancelled .result-icon { color: var(--warn); }
.result-body { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
.result-name { font-size: 13px; font-weight: 600; color: var(--ink); word-break: break-all; }
.result-path { font-size: 11.5px; color: var(--ink-3); word-break: break-all; line-height: 1.45; }
.result-path.error { color: var(--danger); }
</style>
