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

    <!-- 列表 / 编辑：淡入淡出 + 轻微缩放 -->
    <div class="flip-viewport">
      <Transition name="view-fade" mode="out-in">
        <div v-if="viewMode === 'list'" key="list" class="panel-grid list-grid">
      <div class="card card-projects">
        <div class="card-header-row">
          <div class="card-title">{{ t('overlay.myProjects') }}</div>
          <span class="project-count" v-if="overlayHistory.length">{{ overlayHistory.length }}</span>
        </div>

        <div v-if="overlayHistory.length === 0" class="empty-history">
          <i class="ri-inbox-line"></i>
          <p>{{ t('overlay.emptyHistory') }}</p>
        </div>

        <div v-else class="history-list">
          <div
            v-for="item in overlayHistory"
            :key="item.id"
            class="history-row"
            @click="loadOverlay(item)"
          >
              <div class="row-icon" aria-hidden="true">
                <i class="ri-file-copy-line"></i>
              </div>
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
        </div>
      </div>

      <div class="card card-actions-panel">
        <div class="card-title">{{ t('overlay.quickStart') }}</div>
        <p class="card-hint">{{ t('overlay.quickStartHint') }}</p>

        <button class="action-tile create-tile" @click="showCreateDialog = true">
          <div class="tile-icon"><i class="ri-add-circle-line" aria-hidden="true"></i></div>
          <div class="tile-text">
            <h4>{{ t('overlay.newProject') }}</h4>
            <p>{{ t('overlay.newProjectHint') }}</p>
          </div>
          <i class="ri-arrow-right-s-line option-arrow" aria-hidden="true"></i>
        </button>

        <button class="action-tile" @click="showImportDialog = true">
          <div class="tile-icon"><i class="ri-download-cloud-2-line" aria-hidden="true"></i></div>
          <div class="tile-text">
            <h4>{{ t('overlay.importCode') }}</h4>
            <p>{{ t('overlay.importDesc') }}</p>
          </div>
          <i class="ri-arrow-right-s-line option-arrow" aria-hidden="true"></i>
        </button>
      </div>
    </div>

    <!-- 编辑模式: 左配置 / 右功能，主按钮落在右栏底部 -->
    <div v-else key="editor" class="panel-grid editor-grid">
      <div class="card card-config">
        <div class="card-title">{{ t('overlay.projectConfig', { name: currentOverlay.name }) }}</div>
        <div class="config-block">
          <span class="meta-label">{{ t('overlay.parentPackPath') }}</span>
          <div class="path-row">
            <span class="path-value">{{ currentOverlay.parentPackPath || t('overlay.notSelected') }}</span>
            <button class="select-pack-btn" @click="selectParentPack">
              <i class="ri-folder-open-line" aria-hidden="true"></i>
              <span>{{ t('overlay.selectParent') }}</span>
            </button>
          </div>
        </div>
        <p class="card-hint">{{ t('overlay.parentPackHint') }}</p>
      </div>

      <div class="card card-options">
        <div class="card-title">{{ t('overlay.optionsTitle') }}</div>
        <div class="options-list">
          <button
            v-for="option in options"
            :key="option.id"
            class="option-row"
            @click="option.action"
          >
            <div class="option-icon"><i :class="option.icon" aria-hidden="true"></i></div>
            <div class="option-text">
              <h4>{{ option.title }}</h4>
              <p>{{ option.description }}</p>
            </div>
            <i class="ri-arrow-right-s-line option-arrow" aria-hidden="true"></i>
          </button>
        </div>

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
      </div>
      </Transition>
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

    <!-- 自定义内容：右侧侧栏（自带滑入，Transition 提供滑出） -->
    <Transition name="sidebar-out">
      <ItemNameDialog
        v-if="showCustomNameDialog"
        :projectName="currentOverlay.name"
        @close="showCustomNameDialog = false"
      />
    </Transition>

    <Transition name="sidebar-out">
      <ItemSizeDialog
        v-if="showItemSizeDialog"
        :projectName="currentOverlay.name"
        @close="showItemSizeDialog = false"
      />
    </Transition>

    <Transition name="sidebar-out">
      <VisualDialog
        v-if="showVisualDialog"
        :projectName="currentOverlay.name"
        @close="showVisualDialog = false"
      />
    </Transition>

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
import { open, save } from '@tauri-apps/plugin-dialog';
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
  const defaultName = (currentOverlay.name || '你的覆盖包').replace(/[\\/:*?"<>|]/g, '_');
  let picked: string | null = null;
  try {
    picked = await save({
      filters: [{ name: t('overlay.zipFilter', { defaultValue: '资源包 ZIP' }), extensions: ['zip'] }],
      defaultPath: `${defaultName}.zip`
    });
  } catch {
    picked = null;
  }
  // 用户取消保存对话框：不打包
  if (!picked) return;

  isPackaging.value = true;
  statusMsg.value = null;
  try {
    const outputPath = await invoke<string>('overlay_package', {
      projectName: currentOverlay.name,
      outputPath: picked
    });
    statusMsg.value = { text: t('overlay.packSuccess', { path: outputPath }), type: 'success' };
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
  overflow: hidden;
  position: relative;
  display: flex;
  flex-direction: column;
  color: #1d1d1f;
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", "PingFang SC", "Microsoft YaHei", sans-serif;
}

/* 翻页视口：两页绝对定位叠在同一层，才能看见「一页推出、一页推入」 */
.flip-viewport {
  position: relative;
  flex: 1;
  min-height: 0;
  overflow: hidden;
  z-index: 2;
}

.flip-viewport > .panel-grid {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  flex: none;
  will-change: transform;
}

/* 与 ConversionPage .panel-grid 同构：双栏卡片网格铺满视口 */
.panel-grid {
  min-height: 0;
  z-index: 2;
  padding: 12px 40px 28px;
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(0, 1.15fr);
  gap: 14px;
  overflow: hidden;
  background: transparent;
}

.list-grid {
  grid-template-rows: minmax(0, 1fr);
}

.editor-grid {
  grid-template-rows: minmax(0, 1fr);
}

.panel-grid .card {
  padding: 18px 20px;
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-height: 0;
  overflow: hidden;
}

.header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0;
  padding: 28px 40px 10px;
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

.card-header-row {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}

.project-count {
  font-size: 11px;
  font-weight: 700;
  color: color-mix(in srgb, var(--theme-color) 80%, #000);
  background: color-mix(in srgb, var(--theme-color) 12%, transparent);
  padding: 2px 8px;
  border-radius: 999px;
}

.card-title {
  font-size: 13px;
  font-weight: 700;
  color: #6b7280;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  flex-shrink: 0;
  margin: 0;
}

.card-hint {
  font-size: 12px;
  color: #94a3b8;
  margin: 0;
  flex-shrink: 0;
}

.empty-history {
  display: flex;
  flex: 1;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  min-height: 160px;
  color: #94a3b8;
  gap: 10px;
  border: 1px dashed rgba(0, 0, 0, 0.1);
  border-radius: var(--ui-radius-btn);
  background: rgba(255, 255, 255, 0.35);
}

.empty-history i { font-size: 36px; }
.empty-history p { margin: 0; font-size: 13px; }

.history-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
  overflow-y: auto;
  min-height: 0;
  flex: 1;
  padding-right: 2px;
}

.history-list::-webkit-scrollbar { width: 6px; }
.history-list::-webkit-scrollbar-thumb {
  background: rgba(0, 0, 0, 0.12);
  border-radius: 3px;
}

.history-row {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 14px;
  border-radius: var(--ui-radius-btn);
  border: 1px solid rgba(0, 0, 0, 0.04);
  background: rgba(255, 255, 255, 0.55);
  cursor: pointer;
  transition: background 0.15s ease, border-color 0.15s ease, box-shadow 0.15s ease;
  flex-shrink: 0;
}

.history-row:hover {
  background: rgba(255, 255, 255, 0.92);
  border-color: color-mix(in srgb, var(--theme-color) 32%, transparent);
  box-shadow: 0 2px 8px color-mix(in srgb, var(--theme-color) 10%, transparent);
}

.row-icon {
  width: 36px;
  height: 36px;
  border-radius: 10px;
  flex-shrink: 0;
  background: color-mix(in srgb, var(--theme-color) 12%, transparent);
  color: var(--theme-color);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 18px;
}

.card-info { min-width: 0; flex: 1; }

.card-name {
  font-size: 14px;
  font-weight: 700;
  color: #1d1d1f;
  margin: 0 0 2px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.card-meta {
  font-size: 12px;
  color: #86868b;
  display: flex;
  align-items: center;
  gap: 4px;
  margin: 0;
}

.card-actions { display: flex; align-items: center; gap: 2px; flex-shrink: 0; }

.icon-btn {
  width: 32px;
  height: 32px;
  border: none;
  background: transparent;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: 8px;
  cursor: pointer;
  color: #64748b;
  transition: background 0.15s ease, color 0.15s ease;
}
.icon-btn:hover { background: rgba(0, 0, 0, 0.05); color: #1d1d1f; }
.delete-btn { color: #94a3b8; }
.delete-btn:hover { color: #ef4444; background: #fee2e2; }

/* 右栏：快速开始 */
.action-tile {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  width: 100%;
  padding: 14px;
  border: 1px solid rgba(0, 0, 0, 0.04);
  border-radius: var(--ui-radius-btn);
  background: rgba(255, 255, 255, 0.55);
  cursor: pointer;
  text-align: left;
  font-family: inherit;
  transition: background 0.15s ease, border-color 0.15s ease, box-shadow 0.15s ease;
}

.action-tile:hover {
  background: rgba(255, 255, 255, 0.92);
  border-color: color-mix(in srgb, var(--theme-color) 32%, transparent);
  box-shadow: 0 2px 8px color-mix(in srgb, var(--theme-color) 10%, transparent);
}

.tile-icon {
  width: 36px;
  height: 36px;
  border-radius: 10px;
  flex-shrink: 0;
  background: color-mix(in srgb, var(--theme-color) 12%, transparent);
  color: var(--theme-color);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 18px;
}

.create-tile {
  border-color: color-mix(in srgb, var(--theme-color) 22%, transparent);
  background: color-mix(in srgb, var(--theme-color) 6%, #ffffff);
}

.tile-text { flex: 1; min-width: 0; }
.tile-text h4 {
  margin: 0 0 4px;
  font-size: 14px;
  font-weight: 700;
  color: #1d1d1f;
}
.tile-text p {
  margin: 0;
  font-size: 12px;
  color: #86868b;
  line-height: 1.45;
  white-space: normal;
  word-break: break-word;
}

/* 编辑：母包配置 */
.config-block {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 14px;
  background: rgba(255, 255, 255, 0.55);
  border: 1px solid rgba(0, 0, 0, 0.04);
  border-radius: var(--ui-radius-btn);
}

.meta-label { font-size: 13px; font-weight: 600; color: #64748b; }

.path-row {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
}

.path-value {
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 12px;
  color: #475569;
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.select-pack-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  height: 32px;
  padding: 0 12px;
  flex-shrink: 0;
  border: 1px solid rgba(0, 0, 0, 0.08);
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.9);
  color: #1d1d1f;
  font-family: inherit;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: border-color 0.15s ease, background 0.15s ease;
}

.select-pack-btn:hover {
  border-color: color-mix(in srgb, var(--theme-color) 35%, transparent);
  background: #fff;
}

/* 编辑：功能入口竖排 */
.options-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
  flex: 1;
  min-height: 0;
  overflow-y: auto;
}

.option-row {
  display: flex;
  align-items: center;
  gap: 12px;
  width: 100%;
  padding: 14px;
  border: 1px solid rgba(0, 0, 0, 0.04);
  border-radius: var(--ui-radius-btn);
  background: rgba(255, 255, 255, 0.55);
  cursor: pointer;
  text-align: left;
  font-family: inherit;
  transition: background 0.15s ease, border-color 0.15s ease, box-shadow 0.15s ease;
}

.option-row:hover {
  background: rgba(255, 255, 255, 0.92);
  border-color: color-mix(in srgb, var(--theme-color) 32%, transparent);
  box-shadow: 0 2px 8px color-mix(in srgb, var(--theme-color) 10%, transparent);
}

.option-icon {
  width: 36px;
  height: 36px;
  border-radius: 10px;
  flex-shrink: 0;
  background: color-mix(in srgb, var(--theme-color) 12%, transparent);
  color: var(--theme-color);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 18px;
}

.option-text { flex: 1; min-width: 0; }
.option-row h4 { margin: 0 0 4px; font-size: 14px; font-weight: 700; color: #1d1d1f; }
.option-row p {
  margin: 0;
  font-size: 12px;
  color: #86868b;
  line-height: 1.45;
  white-space: normal;
  word-break: break-word;
}

.option-arrow {
  font-size: 16px;
  color: #c6c6c8;
  flex-shrink: 0;
  transition: color 0.15s ease, transform 0.15s ease;
}
.option-row:hover .option-arrow,
.action-tile:hover .option-arrow {
  color: var(--theme-color);
  transform: translateX(2px);
}

@media (max-width: 900px) {
  .panel-grid {
    grid-template-columns: 1fr;
    padding: 12px 20px 24px;
  }
  .header { padding: 20px 20px 8px; }
}

/* 底部操作条：贴在右栏卡片内 */
.editor-actions {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  margin-top: auto;
  padding-top: 8px;
  flex-shrink: 0;
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
  box-shadow: 0 8px 18px color-mix(in srgb, var(--theme-color) 32%, transparent);
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
.primary-btn:hover:not(:disabled) { opacity: 0.92; }
.primary-btn:disabled { opacity: 0.5; cursor: not-allowed; }

.danger-btn {
  padding: 10px 20px; border-radius: 12px; border: none;
  background: #ef4444; color: #fff; font-weight: 700; cursor: pointer;
  transition: all 0.2s;
}
.danger-btn:hover { background: #dc2626; }

/* 动画相关 */
.page-transition { animation: slide-up 0.6s cubic-bezier(0.16, 1, 0.3, 1); }
@keyframes slide-up { from { opacity: 0; transform: translateY(20px); } to { opacity: 1; transform: translateY(0); } }

/* 列表 ↔ 编辑：淡入淡出 + 轻微缩放（out-in，干净不叠影） */
.view-fade-enter-active {
  transition: opacity 0.22s ease, transform 0.22s cubic-bezier(0.22, 1, 0.36, 1);
}
.view-fade-leave-active {
  transition: opacity 0.16s ease, transform 0.16s ease;
}
.view-fade-enter-from {
  opacity: 0;
  transform: scale(0.985);
}
.view-fade-leave-to {
  opacity: 0;
  transform: scale(1.01);
}

/* dialog enter/leave 走全局 CSS class 模式(`<transition name="dialog-pop">`,
   见 App.vue 全局 .dialog-pop-* 规则)。Vue 3 在 element insert 时直接加
   enter-from class,跟 element 同一个 commit,第一帧 paint 一定看到 from 状态
   → 杜绝「打开瞬间闪一下」。 */

.spin { animation: ri-spin 1s linear infinite; }
@keyframes ri-spin { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }

/* 侧栏滑出：Transition 加在组件根上，要用 :deep 才能动到子组件里的 .dialog-container */
.sidebar-out-leave-active {
  transition: opacity 0.22s ease;
}
.sidebar-out-leave-active :deep(.dialog-container) {
  animation: none !important;
  transition: transform 0.22s cubic-bezier(0.4, 0, 1, 1) !important;
}
.sidebar-out-leave-to {
  opacity: 0;
}
.sidebar-out-leave-to :deep(.dialog-container) {
  transform: translateX(100%) !important;
}
</style>
