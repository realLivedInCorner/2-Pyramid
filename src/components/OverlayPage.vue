<template>
  <div class="overlay-container page-transition">
    <!-- 头部区域 -->
    <div class="header">
      <div class="header-left">
        <button class="back-btn" @click="goBack" :aria-label="t('overlay.backAria')">
          <i class="ri-arrow-left-line"></i>
        </button>
        <h1 class="title">{{ t('overlay.title') }}</h1>
      </div>
      <div class="header-status" v-if="statusMsg" :class="statusMsg.type">
        <i :class="statusMsg.type === 'success' ? 'ri-checkbox-circle-line' : 'ri-error-warning-line'"></i>
        {{ statusMsg.text }}
      </div>
    </div>

    <!-- Developing 占位 -->
    <div class="developing-overlay">
      <div class="developing-card">
        <div class="developing-icon">
          <svg viewBox="0 0 200 200" fill="none">
            <circle cx="100" cy="100" r="60" stroke="var(--theme-color)" stroke-width="3" opacity="0.2"/>
            <circle cx="100" cy="100" r="40" stroke="var(--theme-color)" stroke-width="2" opacity="0.15"/>
            <g stroke="var(--theme-color)" stroke-width="3" stroke-linecap="round" opacity="0.5">
              <line x1="80" y1="88" x2="92" y2="100"/>
              <line x1="92" y1="100" x2="80" y2="112"/>
              <line x1="108" y1="88" x2="120" y2="100"/>
              <line x1="120" y1="100" x2="108" y2="112"/>
            </g>
            <circle cx="100" cy="100" r="70" stroke="var(--theme-color)" stroke-width="1" opacity="0.1" stroke-dasharray="6 6">
              <animateTransform attributeName="transform" type="rotate" from="0 100 100" to="360 100 100" dur="20s" repeatCount="indefinite"/>
            </circle>
          </svg>
        </div>
        <h2 class="developing-title">Developing</h2>
        <p class="developing-desc">此功能正在开发中，敬请期待</p>
        <div class="developing-badge">
          <i class="ri-code-s-slash-line"></i>
          <span>Coming Soon</span>
        </div>
      </div>
    </div>

    <!-- 列表模式: 历史项目管理 (已禁用) -->
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
            <div>
              <h4>{{ option.title }}</h4>
              <p>{{ option.description }}</p>
            </div>
          </button>
        </TransitionGroup>
      </div>

      <!-- 悬浮操作按钮组 (右下角) -->
      <div class="floating-actions">
        <button class="action-btn exit-btn" @click="viewMode = 'list'">
          <i class="ri-arrow-go-back-line"></i>
          <span>{{ t('overlay.exitEdit') }}</span>
        </button>
        <button class="action-btn package-btn" :disabled="isPackaging" @click="handlePackage">
          <i class="ri-archive-line" v-if="!isPackaging"></i>
          <i class="ri-loader-4-line spin" v-else></i>
          <span>{{ isPackaging ? t('overlay.packaging') : t('overlay.startPackaging') }}</span>
        </button>
      </div>
    </div>

    <!-- 新建项目对话框 -->
    <Transition name="dialog-fade">
      <div v-if="showCreateDialog" class="dialog-overlay" @click.self="showCreateDialog = false">
        <div class="simple-dialog">
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
    </Transition>

    <!-- 对话框 -->
    <Transition name="dialog-fade">
      <ItemNameDialog 
        v-if="showCustomNameDialog" 
        :projectName="currentOverlay.name"
        @close="showCustomNameDialog = false" 
      />
    </Transition>
    
    <Transition name="dialog-fade">
      <ItemSizeDialog 
        v-if="showItemSizeDialog" 
        :projectName="currentOverlay.name"
        @close="showItemSizeDialog = false" 
      />
    </Transition>

    <Transition name="dialog-fade">
      <VisualDialog 
        v-if="showVisualDialog" 
        :projectName="currentOverlay.name"
        @close="showVisualDialog = false" 
      />
    </Transition>

    <!-- 导入分享码对话框 -->
    <Transition name="dialog-fade">
      <div v-if="showImportDialog" class="dialog-overlay" @click.self="showImportDialog = false">
        <div class="simple-dialog">
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
    </Transition>

    <!-- 导出分享码对话框 -->
    <Transition name="dialog-fade">
      <div v-if="showExportDialog" class="dialog-overlay" @click.self="showExportDialog = false">
        <div class="simple-dialog">
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
    </Transition>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, reactive } from 'vue';
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core';
import { open, ask } from '@tauri-apps/plugin-dialog';
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

const deleteOverlay = async (item: OverlayProject) => {
  const confirmed = await ask(t('overlay.deleteConfirm', { name: item.name }), {
    title: t('overlay.deleteTitle'),
    kind: 'warning'
  });
  
  if (confirmed) {
    try {
      await invoke('delete_overlay_project', { id: item.id });
      overlayHistory.value = overlayHistory.value.filter(i => i.id !== item.id);
      statusMsg.value = { text: t('overlay.deleteSuccess', { name: item.name }), type: 'success' };
      setTimeout(() => statusMsg.value = null, 3000);
    } catch (e) {
      statusMsg.value = { text: t('overlay.deleteFailed', { error: e }), type: 'error' };
      setTimeout(() => statusMsg.value = null, 5000);
    }
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
  padding: 24px 48px 40px;
  position: relative;
}

/* Developing overlay */
.developing-overlay {
  position: absolute;
  inset: 0;
  z-index: 100;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(255, 255, 255, 0.85);
  backdrop-filter: blur(8px);
  -webkit-backdrop-filter: blur(8px);
}
.developing-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 16px;
  padding: 48px 64px;
  background: rgba(255, 255, 255, 0.9);
  border-radius: 24px;
  border: 1px solid rgba(0, 0, 0, 0.06);
  box-shadow: 0 8px 40px rgba(0, 0, 0, 0.04);
}
.developing-icon {
  width: 120px;
  height: 120px;
}
.developing-icon svg {
  width: 100%;
  height: 100%;
}
.developing-title {
  font-size: 28px;
  font-weight: 800;
  color: #1a1a2e;
  margin: 0;
  letter-spacing: -0.5px;
}
.developing-desc {
  font-size: 14px;
  color: #9ca3af;
  margin: 0;
}
.developing-badge {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 6px 16px;
  border-radius: 20px;
  background: color-mix(in srgb, var(--theme-color) 10%, transparent);
  color: var(--theme-color);
  font-size: 12px;
  font-weight: 700;
  letter-spacing: 0.5px;
  margin-top: 8px;
}

.content {
  max-width: 1200px;
  margin: 0 auto;
}

.header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 40px;
  position: relative;
  z-index: 10;
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-right: 140px; /* 增加边距，确保不遮挡无边框窗口的三个按钮 */
}

.back-btn {
  width: 44px; height: 44px; border-radius: 12px; border: none;
  background: #f1f5f9; color: #64748b; cursor: pointer; transition: all 0.2s;
}

.back-btn:hover { background: #e2e8f0; color: #0f172a; transform: translateX(-2px); }

.title { font-size: 32px; font-weight: 800; color: #0f172a; letter-spacing: -0.02em; }

.header-status {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 16px;
  border-radius: 12px;
  font-size: 14px;
  font-weight: 700;
  animation: status-slide-in 0.3s cubic-bezier(0.34, 1.56, 0.64, 1);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.05);
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

@keyframes status-slide-in {
  from { opacity: 0; transform: translateX(20px); }
  to { opacity: 1; transform: translateX(0); }
}

.history-section { display: flex; flex-direction: column; gap: 24px; }

.section-header { display: flex; justify-content: space-between; align-items: center; }

.section-title { font-size: 20px; font-weight: 700; color: #1e293b; }

.create-btn {
  display: flex; align-items: center; gap: 8px; padding: 10px 20px;
  background: var(--theme-color); color: #fff; border: none; border-radius: 12px;
  font-weight: 700; cursor: pointer; transition: all 0.2s;
  box-shadow: 0 4px 12px rgba(var(--theme-color-rgb), 0.2);
}

.create-btn:hover { transform: translateY(-2px); box-shadow: 0 6px 16px rgba(var(--theme-color-rgb), 0.3); }

.empty-history {
  display: flex; flex-direction: column; align-items: center; justify-content: center;
  padding: 80px 0; color: #94a3b8; gap: 12px;
}

.empty-history i { font-size: 48px; }

.history-grid {
  display: grid; grid-template-columns: repeat(auto-fill, minmax(300px, 1fr)); gap: 20px;
}

.history-card {
  background: #fff; border: 1px solid #f1f5f9; border-radius: 20px; padding: 20px;
  cursor: pointer; display: flex; justify-content: space-between; align-items: center;
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

.history-card:hover {
  border-color: var(--theme-color); transform: translateY(-4px);
  box-shadow: 0 12px 24px -8px rgba(var(--theme-color-rgb), 0.15);
}

.card-name { font-size: 16px; font-weight: 700; color: #0f172a; margin-bottom: 4px; }

.card-meta {
  font-size: 12px; color: #94a3b8; display: flex; align-items: center; gap: 4px;
}

.delete-btn { color: #94a3b8; padding: 8px; border-radius: 8px; transition: all 0.2s; }
.delete-btn:hover { color: #ef4444; background: #fee2e2; }

.card {
  background: #fff; border-radius: 24px; padding: 32px; border: 1px solid #f1f5f9;
  box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.05); margin-bottom: 32px;
}

.card-title { font-size: 18px; font-weight: 700; color: #0f172a; margin-bottom: 24px; }

.meta-item {
  display: flex; justify-content: space-between; align-items: center;
  padding: 16px; background: #f8fafc; border-radius: 16px;
}

.meta-label { font-size: 14px; font-weight: 600; color: #64748b; }

.meta-content { display: flex; align-items: center; gap: 12px; }

.path-value {
  font-family: monospace; font-size: 13px; color: #64748b;
  max-width: 300px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
}

.options-grid {
  display: grid; grid-template-columns: repeat(auto-fill, minmax(280px, 1fr)); gap: 20px;
}

.option-card {
  display: flex; align-items: center; gap: 20px; padding: 24px; background: #fff;
  border: 1px solid #f1f5f9; border-radius: 20px; cursor: pointer;
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1); text-align: left;
}

.option-card:hover {
  transform: translateY(-4px); border-color: var(--theme-color);
  box-shadow: 0 12px 24px -8px rgba(var(--theme-color-rgb), 0.2);
}

.option-icon {
  width: 56px; height: 56px; border-radius: 16px;
  background: rgba(var(--theme-color-rgb), 0.1); color: var(--theme-color);
  display: flex; align-items: center; justify-content: center; font-size: 24px;
}

.option-card h4 { margin: 0 0 4px 0; font-size: 16px; font-weight: 700; color: #0f172a; }
.option-card p { margin: 0; font-size: 13px; color: #64748b; line-height: 1.4; }

.action-btn {
  display: flex; align-items: center; gap: 8px; padding: 12px 24px;
  border-radius: 14px; border: none; font-weight: 700; cursor: pointer; transition: all 0.2s;
}

.exit-btn { background: #f1f5f9; color: #475569; }
.exit-btn:hover { background: #e2e8f0; color: #0f172a; }

.package-btn {
  background: var(--theme-color); color: #fff;
  box-shadow: 0 8px 16px -4px rgba(var(--theme-color-rgb), 0.3);
}

.package-btn:hover:not(:disabled) {
  transform: translateY(-2px); box-shadow: 0 12px 20px -4px rgba(var(--theme-color-rgb), 0.4);
}

.package-btn:disabled { opacity: 0.7; cursor: not-allowed; }

/* 悬浮操作按钮组 (右下角) */
.floating-actions {
  position: fixed;
  right: 40px;
  bottom: 40px;
  display: flex;
  gap: 16px;
  z-index: 100;
  animation: actions-slide-in 0.5s cubic-bezier(0.16, 1, 0.3, 1);
}

.floating-actions .action-btn {
  box-shadow: 0 10px 25px -5px rgba(0, 0, 0, 0.1), 0 8px 10px -6px rgba(0, 0, 0, 0.1);
}

@keyframes actions-slide-in {
  from { opacity: 0; transform: translateY(20px); }
  to { opacity: 1; transform: translateY(0); }
}

/* 弹窗样式 */
.dialog-overlay {
  position: fixed; inset: 0; background: rgba(15, 23, 42, 0.4);
  display: flex; align-items: center; justify-content: center; z-index: 1000; backdrop-filter: blur(8px);
}

.simple-dialog {
  background: #fff; padding: 32px; border-radius: 24px; width: 400px;
  box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.25);
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

/* 动画相关 */
.page-transition { animation: slide-up 0.6s cubic-bezier(0.16, 1, 0.3, 1); }
@keyframes slide-up { from { opacity: 0; transform: translateY(20px); } to { opacity: 1; transform: translateY(0); } }

.staggered-fade-enter-active {
  animation: card-in 0.5s cubic-bezier(0.16, 1, 0.3, 1) both;
  animation-delay: calc(var(--index) * 0.1s);
}
@keyframes card-in { from { opacity: 0; transform: scale(0.9) translateY(20px); } to { opacity: 1; transform: scale(1) translateY(0); } }

.dialog-fade-enter-active, .dialog-fade-leave-active { transition: all 0.3s ease; }
.dialog-fade-enter-from, .dialog-fade-leave-to { opacity: 0; transform: scale(1.05); }

.spin { animation: ri-spin 1s linear infinite; }
@keyframes ri-spin { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }
</style>
