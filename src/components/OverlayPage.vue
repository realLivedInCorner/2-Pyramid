<template>
  <div class="overlay-container page-transition">
    <!-- 头部区域 -->
    <div class="header">
      <div class="header-section">
        <button class="back-btn" @click="goBack" :aria-label="t('common.backToHome')">
          <i class="ri-arrow-left-line back-icon" aria-hidden="true"></i>
          <span>{{ t('common.back') }}</span>
        </button>
        <div class="title-group">
          <h1 class="title">{{ t('overlay.title') }}</h1>
          <p class="page-subtitle">{{ t('overlay.subtitle') }}</p>
        </div>
      </div>
      <transition name="header-status-toast">
        <div v-if="statusMsg" class="header-status" :class="statusMsg.type">
          <i :class="statusMsg.type === 'success' ? 'ri-checkbox-circle-line' : 'ri-error-warning-line'"></i>
          {{ statusMsg.text }}
        </div>
      </transition>
    </div>

    <!-- 列表模式: 历史项目管理 -->
    <div v-if="viewMode === 'list'" class="content">
      <div class="history-section">
        <div class="section-header">
          <h2 class="section-title">{{ t('overlay.myProjects') }}</h2>
          <div class="header-btns">
            <button class="ghost-btn import-share-btn" @click="showImportDialog = true">
              <i class="ri-download-cloud-2-line"></i>
              <span>{{ t('overlay.importCode') }}</span>
            </button>
            <button class="create-btn" @click="showCreateDialog = true">
              <i class="ri-add-line"></i>
              <span>{{ t('overlay.newProject') }}</span>
            </button>
          </div>
        </div>

        <div v-if="overlayHistory.length === 0" class="empty-history">
          <i class="ri-inbox-line"></i>
          <p>{{ t('overlay.emptyHistory') }}</p>
        </div>

        <div v-else class="history-grid">
          <TransitionGroup name="staggered-fade">
            <div 
              v-for="(item, index) in overlayHistory" 
              :key="item.id" 
              class="history-card"
              :style="{ '--index': index }"
              @click="loadOverlay(item)"
            >
              <div class="card-info">
                <h3 class="card-name">{{ item.name }}</h3>
                <p class="card-meta">
                  <i class="ri-time-line"></i> {{ formatDate(item.updatedAt) }}
                </p>
              </div>
              <div class="card-actions">
                <button class="icon-btn share-btn" @click.stop="exportShareCode(item)" :title="t('overlay.exportCode')">
                  <i class="ri-share-forward-line"></i>
                </button>
                <button class="icon-btn delete-btn" @click.stop="deleteOverlay(item)">
                  <i class="ri-delete-bin-line"></i>
                </button>
              </div>
            </div>
          </TransitionGroup>
        </div>
      </div>
    </div>

    <!-- 编辑模式: 具体项目配置 -->
    <div v-else class="content">
      <div class="card card-config">
        <div class="card-title">{{ t('overlay.projectConfig', { name: currentOverlay.name }) }}</div>

        <div class="meta-list">
          <div class="meta-item">
            <span class="meta-label">{{ t('overlay.parentPackPath') }}</span>
            <div class="meta-content">
              <span class="meta-value path-value">{{ currentOverlay.parentPackPath || t('overlay.notSelected') }}</span>
              <button class="icon-btn" @click="selectParentPack" :title="t('overlay.selectParent')">
                <i class="ri-folder-open-line"></i>
              </button>
            </div>
          </div>
        </div>
      </div>

      <div class="options-grid">
        <TransitionGroup name="staggered-fade">
          <button
            v-for="(option, index) in options"
            :key="option.id"
            class="option-card"
            :style="{ '--index': index }"
            @click="option.action"
          >
            <div class="option-icon"><i :class="option.icon" aria-hidden="true"></i></div>
            <div class="option-text">
              <h4>{{ option.title }}</h4>
              <p>{{ option.description }}</p>
            </div>
            <i class="ri-arrow-right-s-line option-arrow" aria-hidden="true"></i>
          </button>
        </TransitionGroup>
      </div>

      <!-- 底部操作条（非悬浮，避免遮挡内容） -->
      <div class="editor-actions">
        <button class="ghost-btn" @click="viewMode = 'list'">
          <i class="ri-arrow-go-back-line"></i>
          <span>{{ t('overlay.exitEdit') }}</span>
        </button>
        <button class="primary-btn package-btn" :disabled="isPackaging" @click="handlePackage">
          <i class="ri-archive-line" v-if="!isPackaging"></i>
          <i class="ri-loader-4-line spin" v-else></i>
          <span>{{ isPackaging ? t('overlay.packaging') : t('overlay.startPackaging') }}</span>
        </button>
      </div>
    </div>

    <!-- 新建项目对话框 -->
    <transition name="dialog-pop-quick">
      <div v-if="showCreateDialog" class="dialog-overlay" @click.self="showCreateDialog = false">
        <div class="simple-dialog dialog-content">
          <h3>{{ t('overlay.createTitle') }}</h3>
          <input
            v-model="newProjectName"
            :placeholder="t('overlay.createPlaceholder')"
            class="project-input"
            @keyup.enter="handleCreateProject"
          />
          <div class="dialog-footer">
            <button class="ghost-btn" @click="showCreateDialog = false">{{ t('common.cancel') }}</button>
            <button
              class="primary-btn"
              :disabled="!newProjectName"
              @click="handleCreateProject"
            >
              {{ t('common.create') }}
            </button>
          </div>
        </div>
      </div>
    </transition>

    <!-- 对话框 -->
    <transition name="dialog-pop">
      <ItemNameDialog
        v-if="showCustomNameDialog"
        :projectName="currentOverlay.name"
        @close="showCustomNameDialog = false"
      />
    </transition>

    <transition name="dialog-pop">
      <ItemSizeDialog
        v-if="showItemSizeDialog"
        :projectName="currentOverlay.name"
        @close="showItemSizeDialog = false"
      />
    </transition>

    <transition name="dialog-pop">
      <VisualDialog
        v-if="showVisualDialog"
        :projectName="currentOverlay.name"
        @close="showVisualDialog = false"
      />
    </transition>

    <!-- 导入分享码对话框 -->
    <transition name="dialog-pop-quick">
      <div v-if="showImportDialog" class="dialog-overlay" @click.self="showImportDialog = false">
        <div class="simple-dialog dialog-content">
          <h3>{{ t('overlay.importTitle') }}</h3>
          <p class="dialog-desc">{{ t('overlay.importDesc') }}</p>
          <textarea
            v-model="shareCodeToImport"
            :placeholder="t('overlay.importPlaceholder')"
            class="project-input share-textarea"
          ></textarea>
          <div class="dialog-footer">
            <button class="ghost-btn" @click="showImportDialog = false">{{ t('common.cancel') }}</button>
            <button
              class="primary-btn"
              :disabled="!shareCodeToImport.startsWith('HRCN-')"
              @click="handleImportShareCode"
            >
              {{ t('common.import') }}
            </button>
          </div>
        </div>
      </div>
    </transition>

    <!-- 导出分享码对话框 -->
    <transition name="dialog-pop">
      <div v-if="showExportDialog" class="dialog-overlay" @click.self="showExportDialog = false">
        <div class="simple-dialog dialog-content">
          <h3>{{ t('overlay.exportTitle') }}</h3>
          <p class="dialog-desc">{{ t('overlay.exportDesc') }}</p>
          <div class="share-code-box">
            <code>{{ exportedShareCode }}</code>
          </div>
          <div class="dialog-footer">
            <button class="ghost-btn" @click="showExportDialog = false">{{ t('common.close') }}</button>
            <button class="primary-btn" @click="copyShareCode">{{ t('overlay.copyCode') }}</button>
          </div>
        </div>
      </div>
    </transition>

    <!-- 删除覆盖包确认对话框（自绘，不使用系统原生弹窗） -->
    <transition name="dialog-pop-quick">
      <div v-if="showDeleteDialog" class="dialog-overlay" @click.self="showDeleteDialog = false">
        <div class="simple-dialog dialog-content">
          <h3>{{ t('overlay.deleteTitle') }}</h3>
          <p class="dialog-desc">{{ t('overlay.deleteConfirm', { name: pendingDelete?.name ?? '' }) }}</p>
          <div class="dialog-footer">
            <button class="ghost-btn" @click="showDeleteDialog = false">{{ t('common.cancel') }}</button>
            <button class="danger-btn" @click="confirmDeleteOverlay">
              {{ t('overlay.deleteConfirmBtn') }}
            </button>
          </div>
        </div>
      </div>
    </transition>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, reactive } from 'vue';
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import ItemNameDialog from './ItemNameDialog.vue';
import ItemSizeDialog from './ItemSizeDialog.vue';
import VisualDialog from './VisualDialog.vue';
const { t } = useI18n()

interface OverlayProject {
  id: string;
  name: string;
  parentPackPath: string;
  updatedAt: number;
}

const emit = defineEmits(['switch-page']);

const viewMode = ref<'list' | 'editor'>('list');
const overlayHistory = ref<OverlayProject[]>([]);
const currentOverlay = reactive<OverlayProject>({
  id: '',
  name: '',
  parentPackPath: '',
  updatedAt: 0
});

const isPackaging = ref(false);
const statusMsg = ref<{ text: string, type: 'success' | 'error' } | null>(null);
const showCustomNameDialog = ref(false);
const showItemSizeDialog = ref(false);
const showVisualDialog = ref(false);
const showCreateDialog = ref(false);
const showImportDialog = ref(false);
const showExportDialog = ref(false);
const showDeleteDialog = ref(false);
const pendingDelete = ref<OverlayProject | null>(null);
const shareCodeToImport = ref('');
const exportedShareCode = ref('');
const newProjectName = ref('');

const options = [
  {
    id: 'name',
    title: t('overlay.options.customName.title'),
    description: t('overlay.options.customName.desc'),
    icon: 'ri-text',
    action: () => showCustomNameDialog.value = true
  },
  {
    id: 'size',
    title: t('overlay.options.itemSize.title'),
    description: t('overlay.options.itemSize.desc'),
    icon: 'ri-fullscreen-line',
    action: () => showItemSizeDialog.value = true
  },
  {
    id: 'visual',
    title: t('overlay.options.visual.title'),
    description: t('overlay.options.visual.desc'),
    icon: 'ri-magic-line',
    action: () => showVisualDialog.value = true
  }
];

const goBack = () => emit('switch-page', 'home');

const formatDate = (timestamp: number) => {
  return new Date(timestamp).toLocaleString('zh-CN', {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit'
  });
};

const loadProjects = async () => {
  try {
    const projects = await invoke<any[]>('get_overlay_projects');
    // 兼容处理后端返回的 snake_case 或 camelCase
    const processedProjects = projects.map(p => ({
      id: p.id,
      name: p.name,
      parentPackPath: p.parentPackPath || p.parent_pack_path || '',
      updatedAt: p.updatedAt || p.updated_at || Date.now()
    }));
    overlayHistory.value = processedProjects.sort((a, b) => b.updatedAt - a.updatedAt);
  } catch (e) {
    console.error('加载项目列表失败:', e);
  }
};

const handleCreateProject = async () => {
  if (!newProjectName.value) return;
  statusMsg.value = null;
  try {
    const newProject = await invoke<OverlayProject>('overlay_init', {
      request: {
        name: newProjectName.value,
        parentPackPath: ''
      }
    });
    overlayHistory.value.unshift(newProject);
    showCreateDialog.value = false;
    newProjectName.value = '';
    statusMsg.value = { text: t('overlay.createSuccess', { name: newProject.name }), type: 'success' };
    setTimeout(() => statusMsg.value = null, 3000);
    loadOverlay(newProject);
  } catch (e) {
    statusMsg.value = { text: t('overlay.createFailed', { error: e }), type: 'error' };
    setTimeout(() => statusMsg.value = null, 5000);
  }
};

const exportShareCode = async (item: OverlayProject) => {
  try {
    const code = await invoke<string>('export_overlay_share_code', { projectName: item.name });
    exportedShareCode.value = code;
    showExportDialog.value = true;
  } catch (e) {
    statusMsg.value = { text: t('overlay.exportFailed', { error: e }), type: 'error' };
    setTimeout(() => statusMsg.value = null, 5000);
  }
};

const handleImportShareCode = async () => {
  if (!shareCodeToImport.value) return;
  try {
    const newProject = await invoke<any>('import_overlay_share_code', { shareCode: shareCodeToImport.value });
    const processedProject = {
      id: newProject.id,
      name: newProject.name,
      parentPackPath: newProject.parentPackPath || '',
      updatedAt: newProject.updatedAt || Date.now()
    };
    overlayHistory.value.unshift(processedProject);
    showImportDialog.value = false;
    shareCodeToImport.value = '';
    statusMsg.value = { text: t('overlay.importSuccess', { name: processedProject.name }), type: 'success' };
    setTimeout(() => statusMsg.value = null, 3000);
  } catch (e) {
    statusMsg.value = { text: t('overlay.importFailed', { error: e }), type: 'error' };
    setTimeout(() => statusMsg.value = null, 5000);
  }
};

const copyShareCode = async () => {
  try {
    await navigator.clipboard.writeText(exportedShareCode.value);
    statusMsg.value = { text: t('overlay.codeCopied'), type: 'success' };
    setTimeout(() => statusMsg.value = null, 3000);
  } catch (e) {
    statusMsg.value = { text: t('overlay.copyFailed'), type: 'error' };
  }
};

const loadOverlay = (item: OverlayProject) => {
  Object.assign(currentOverlay, item);
  viewMode.value = 'editor';
};

const deleteOverlay = (item: OverlayProject) => {
  // Custom-drawn confirm dialog instead of the OS-native
  // plugin-dialog `ask` — the native window looks alien next to the
  // rest of the UI (and can't be themed / animated).
  pendingDelete.value = item;
  showDeleteDialog.value = true;
};

const confirmDeleteOverlay = async () => {
  const item = pendingDelete.value;
  if (!item) return;
  showDeleteDialog.value = false;
  try {
    await invoke('delete_overlay_project', { id: item.id });
    overlayHistory.value = overlayHistory.value.filter(i => i.id !== item.id);
    statusMsg.value = { text: t('overlay.deleteSuccess', { name: item.name }), type: 'success' };
    setTimeout(() => statusMsg.value = null, 3000);
  } catch (e) {
    statusMsg.value = { text: t('overlay.deleteFailed', { error: e }), type: 'error' };
    setTimeout(() => statusMsg.value = null, 5000);
  }
};

const selectParentPack = async () => {
  try {
    const selected = await open({
      multiple: false,
      filters: [{ name: t('overlay.resourcePackFilter'), extensions: ['zip', 'mcpack'] }]
    });
    if (selected) {
      const path = selected as string;
      await invoke('overlay_set_parent_pack', {
        patch: {
          projectId: currentOverlay.id,
          parentPackPath: path
        }
      });
      currentOverlay.parentPackPath = path;
      currentOverlay.updatedAt = Date.now();
      
      // 更新列表中的时间
      const idx = overlayHistory.value.findIndex(i => i.id === currentOverlay.id);
      if (idx !== -1) {
        overlayHistory.value[idx].parentPackPath = path;
        overlayHistory.value[idx].updatedAt = currentOverlay.updatedAt;
      }
      statusMsg.value = { text: t('overlay.parentPackUpdated'), type: 'success' };
      setTimeout(() => statusMsg.value = null, 3000);
    }
  } catch (e) {
    statusMsg.value = { text: t('overlay.selectFailed', { error: e }), type: 'error' };
    setTimeout(() => statusMsg.value = null, 5000);
  }
};

const handlePackage = async () => {
  isPackaging.value = true;
  statusMsg.value = null;
  try {
    const outputPath = await invoke<string>('overlay_package', {
      projectName: currentOverlay.name
    });
    statusMsg.value = { text: t('overlay.packSuccess', { path: outputPath }), type: 'success' };
    // 打包成功提示可以多停留一会
    setTimeout(() => statusMsg.value = null, 8000);
  } catch (e) {
    statusMsg.value = { text: t('overlay.packFailed', { error: e }), type: 'error' };
    setTimeout(() => statusMsg.value = null, 5000);
  } finally {
    isPackaging.value = false;
  }
};

onMounted(() => {
  loadProjects();
});
</script>

<style scoped>
/* 之前的样式保留，并添加弹窗样式 */
.overlay-container {
  width: 100%;
  height: 100%;
  overflow-y: auto;
  padding: 20px 40px 28px;
  position: relative;
  display: flex;
  flex-direction: column;
}

.content {
  width: 100%;
  max-width: 920px;
  margin: 0 auto;
  display: flex;
  flex-direction: column;
  gap: 14px;
  flex: 1 1 auto;
  min-height: 0;
}

/* 编辑模式：操作条贴底，避免悬在半空 */
.content .editor-actions {
  margin-top: auto;
  padding-top: 18px;
}

.header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 18px;
  position: relative;
  z-index: 10;
  flex-shrink: 0;
}

/* Header layout (matches ConversionPage exactly):
   ┌────────────────────────────────────────────────┐
   │ [← 返回]  覆盖包制作                            │
   │            创建自定义覆盖包,叠加到任意母包之上    │
   └────────────────────────────────────────────────┘
   Back button on the left; title + subtitle stacked
   vertically in `.title-group` to the right. */
.header-section {
  display: flex;
  align-items: center;
  gap: 20px;
  min-width: 0;
  flex: 1 1 auto;
}
.title-group { display: flex; flex-direction: column; min-width: 0; }

.header-actions {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-right: 140px; /* 增加边距，确保不遮挡无边框窗口的三个按钮 */
}

.title { font-size: 26px; font-weight: 800; color: #1d1d1f; letter-spacing: -0.6px; margin: 0; }

.page-subtitle { margin: 4px 0 0; color: #86868b; font-size: 13px; }

.header-status {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 16px;
  border-radius: 12px;
  font-size: 14px;
  font-weight: 700;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.05);
  max-width: 480px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.header-status.success {
  background: #ecfdf5;
  color: #10b981;
  border: 1px solid #d1fae5;
}

.header-status.error {
  background: #fef2f2;
  color: #ef4444;
  border: 1px solid #fee2e2;
}

/* status enter/leave 走全局 CSS class 模式(`<transition name="header-status-toast">`,
   见 App.vue 全局 .header-status-toast-* 规则)。 */

.history-section { display: flex; flex-direction: column; gap: 14px; }

.section-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
}

.section-title {
  font-size: 13px;
  font-weight: 700;
  color: #6b7280;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  margin: 0;
}

.header-btns { display: flex; align-items: center; gap: 8px; }

.import-share-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  height: 38px;
  padding: 0 14px;
  font-size: 13px;
  font-weight: 600;
  font-family: inherit;
  cursor: pointer;
}

.create-btn {
  display: inline-flex; align-items: center; gap: 8px; padding: 10px 18px;
  background: var(--theme-color); color: #fff; border: none;
  border-radius: var(--ui-radius-btn);
  font-weight: 700; font-size: 13px; font-family: inherit; cursor: pointer;
  box-shadow: 0 6px 16px color-mix(in srgb, var(--theme-color) 26%, transparent);
  transition: background 0.18s ease, transform 0.15s ease, box-shadow 0.18s ease;
}

.create-btn:hover {
  background: color-mix(in srgb, var(--theme-color) 88%, #000);
  transform: translateY(-1px);
}

.empty-history {
  display: flex; flex-direction: column; align-items: center; justify-content: center;
  padding: 48px 0; color: #94a3b8; gap: 10px;
  border: 1px dashed rgba(0, 0, 0, 0.1);
  border-radius: var(--ui-radius-card);
  background: rgba(255, 255, 255, 0.35);
}

.empty-history i { font-size: 36px; }

/* 项目列表：通栏行卡片，避免 auto-fill 留出空列把单卡挤在左侧 */
.history-grid {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.history-card {
  width: 100%;
  background: var(--ui-card-surface);
  backdrop-filter: blur(var(--ui-blur)) saturate(var(--ui-saturate));
  border: var(--ui-border);
  border-radius: var(--ui-radius-card);
  box-shadow: var(--ui-shadow);
  padding: 14px 16px;
  cursor: pointer;
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
  transition: box-shadow 0.25s ease, transform 0.18s ease;
}

.history-card:hover {
  box-shadow: var(--ui-shadow-hover);
  transform: translateY(-1px);
}

.card-info { min-width: 0; flex: 1; }

.card-name {
  font-size: 15px; font-weight: 700; color: #1d1d1f; margin-bottom: 4px;
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
}

.card-meta {
  font-size: 12px; color: #86868b; display: flex; align-items: center; gap: 4px;
}

.card-actions { display: flex; align-items: center; gap: 2px; flex-shrink: 0; }

.icon-btn {
  width: 32px; height: 32px; border: none; background: transparent;
  display: inline-flex; align-items: center; justify-content: center;
  border-radius: 8px; cursor: pointer; color: #64748b;
  transition: background 0.15s ease, color 0.15s ease;
}
.icon-btn:hover { background: rgba(0, 0, 0, 0.05); color: #1d1d1f; }
.delete-btn { color: #94a3b8; }
.delete-btn:hover { color: #ef4444; background: #fee2e2; }

/* 卡片走全局玻璃皮肤 */
.card {
  padding: 16px 18px;
  margin-bottom: 0;
}

.card-title {
  font-size: 13px; font-weight: 700; color: #6b7280;
  letter-spacing: 0.04em; text-transform: uppercase;
  margin-bottom: 12px;
}

.meta-item {
  display: flex; justify-content: space-between; align-items: center;
  gap: 12px;
  padding: 12px 14px;
  background: rgba(255, 255, 255, 0.55);
  border: 1px solid rgba(0, 0, 0, 0.04);
  border-radius: var(--ui-radius-btn);
}

.meta-label { font-size: 13px; font-weight: 600; color: #64748b; }

.meta-content { display: flex; align-items: center; gap: 8px; min-width: 0; }

.path-value {
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 12px; color: #475569;
  max-width: 420px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
}

/* 三列入口卡：描述完整可读，不截断 */
.options-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 12px;
  align-content: start;
}

.option-card {
  display: flex; align-items: flex-start; gap: 12px;
  padding: 16px;
  min-height: 110px;
  background: var(--ui-card-surface);
  backdrop-filter: blur(var(--ui-blur)) saturate(var(--ui-saturate));
  border: var(--ui-border);
  border-radius: var(--ui-radius-card);
  box-shadow: var(--ui-shadow);
  cursor: pointer;
  text-align: left;
  font-family: inherit;
  transition: box-shadow 0.25s ease, transform 0.18s ease;
}

.option-card:hover {
  box-shadow: var(--ui-shadow-hover);
  transform: translateY(-1px);
}

.option-icon {
  width: 36px; height: 36px; border-radius: 10px; flex-shrink: 0;
  background: color-mix(in srgb, var(--theme-color) 12%, transparent);
  color: var(--theme-color);
  display: flex; align-items: center; justify-content: center; font-size: 18px;
  margin-top: 2px;
}

.option-text { flex: 1; min-width: 0; }
.option-card h4 { margin: 0 0 6px 0; font-size: 14px; font-weight: 700; color: #1d1d1f; }
.option-card p {
  margin: 0;
  font-size: 12px;
  color: #86868b;
  line-height: 1.5;
  /* 允许折行，不截断 */
  white-space: normal;
  word-break: break-word;
}
.option-arrow {
  font-size: 16px; color: #c6c6c8; flex-shrink: 0; margin-top: 2px;
  transition: color 0.15s ease, transform 0.15s ease;
}
.option-card:hover .option-arrow { color: var(--theme-color); transform: translateX(2px); }

@media (max-width: 900px) {
  .options-grid { grid-template-columns: 1fr; }
  .overlay-container { padding: 16px 20px 24px; }
}

/* 底部操作条 */
.editor-actions {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  margin-top: 20px;
  padding-top: 4px;
}

.editor-actions .ghost-btn,
.editor-actions .primary-btn {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  height: 38px;
  padding: 0 16px;
  font-weight: 600;
  font-size: 13px;
  font-family: inherit;
  border-radius: var(--ui-radius-btn);
  cursor: pointer;
}

.package-btn {
  background: var(--theme-color);
  color: #fff;
  border: none;
  box-shadow: 0 6px 16px color-mix(in srgb, var(--theme-color) 28%, transparent);
  transition: background 0.18s ease, transform 0.15s ease, box-shadow 0.18s ease, opacity 0.18s ease;
}
.package-btn:hover:not(:disabled) {
  background: color-mix(in srgb, var(--theme-color) 88%, #000);
  transform: translateY(-1px);
}
.package-btn:disabled { opacity: 0.45; cursor: not-allowed; box-shadow: none; }

/* 弹窗样式 */
.dialog-overlay {
  position: fixed; inset: 0; background: rgba(15, 23, 42, 0.4);
  display: flex; align-items: center; justify-content: center; z-index: 1000; backdrop-filter: blur(8px);
}

.simple-dialog {
  background: #fff; padding: 32px; border-radius: 24px; width: 400px;
  /* 原来 0 25px 50px -12px rgba(0,0,0,0.25) — blur 50px 太大,leave 期间
     opacity 渐变时 shadow 跟着渐变,大 blur 让 shadow "淡出拖尾" 看起来
     持续时间比 dialog 本身还长。改紧凑点(blur 32, opacity 0.18),跟
     leave 550ms 同步消失。 */
  box-shadow: 0 16px 32px -8px rgba(0, 0, 0, 0.18);
}

.simple-dialog h3 { margin: 0 0 20px 0; font-size: 18px; font-weight: 700; color: #0f172a; }

.project-input {
  width: 100%; padding: 12px 16px; border-radius: 12px; border: 1px solid #e2e8f0;
  background: #f8fafc; font-size: 15px; outline: none; transition: 0.2s; margin-bottom: 24px;
}

.share-textarea {
  height: 120px;
  resize: none;
  font-family: monospace;
  font-size: 12px;
}

.dialog-desc {
  font-size: 13px;
  color: #64748b;
  margin-bottom: 16px;
  margin-top: -12px;
}

.share-code-box {
  background: #f1f5f9;
  padding: 16px;
  border-radius: 12px;
  margin-bottom: 24px;
  max-height: 150px;
  overflow-y: auto;
  word-break: break-all;
}

.share-code-box code {
  font-family: monospace;
  font-size: 12px;
  color: #0f172a;
}

.header-btns {
  display: flex;
  gap: 12px;
}

.import-share-btn {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 16px;
  background: #f1f5f9;
  border-radius: 12px;
  font-weight: 600;
  font-size: 14px;
}

.project-input:focus { border-color: var(--theme-color); background: #fff; }

.dialog-footer { display: flex; justify-content: flex-end; gap: 12px; }

.ghost-btn { padding: 10px 20px; border-radius: 12px; border: none; background: transparent; color: #64748b; font-weight: 700; cursor: pointer; }
.ghost-btn:hover { background: #f1f5f9; color: #0f172a; }

.primary-btn {
  padding: 10px 20px; border-radius: 12px; border: none;
  background: var(--theme-color); color: #fff; font-weight: 700; cursor: pointer;
  transition: all 0.2s;
}
.primary-btn:hover:not(:disabled) { opacity: 0.88; transform: translateY(-1px); }
.primary-btn:disabled { opacity: 0.5; cursor: not-allowed; }

.danger-btn {
  padding: 10px 20px; border-radius: 12px; border: none;
  background: #ef4444; color: #fff; font-weight: 700; cursor: pointer;
  transition: all 0.2s;
}
.danger-btn:hover { background: #dc2626; transform: translateY(-1px); }

/* 动画相关 */
.page-transition { animation: slide-up 0.6s cubic-bezier(0.16, 1, 0.3, 1); }
@keyframes slide-up { from { opacity: 0; transform: translateY(20px); } to { opacity: 1; transform: translateY(0); } }

.staggered-fade-enter-active {
  animation: card-in 0.5s cubic-bezier(0.16, 1, 0.3, 1) both;
  animation-delay: calc(var(--index) * 0.1s);
}
.staggered-fade-leave-active {
  animation: card-out 0.25s ease both;
}
@keyframes card-in { from { opacity: 0; transform: scale(0.9) translateY(20px); } to { opacity: 1; transform: scale(1) translateY(0); } }
@keyframes card-out { from { opacity: 1; transform: scale(1) translateY(0); } to { opacity: 0; transform: scale(0.92) translateY(-8px); } }

/* dialog enter/leave 走全局 CSS class 模式(`<transition name="dialog-pop">`,
   见 App.vue 全局 .dialog-pop-* 规则)。Vue 3 在 element insert 时直接加
   enter-from class,跟 element 同一个 commit,第一帧 paint 一定看到 from 状态
   → 杜绝「打开瞬间闪一下」。 */

.spin { animation: ri-spin 1s linear infinite; }
@keyframes ri-spin { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }
</style>
