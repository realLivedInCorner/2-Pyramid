/* .dialog-overlay / .dialog-content 已统一到 shared.css —— 见那里的说明。
   页面级的定义会因 scoped 的 data-v 属性获得更高特异度，留着会盖掉全局那份。 */
<template>
  <!-- 不加 page-transition：与 Conversion 一致，入场交给 App 全局 stagger，
       避免 slide-up transform 和 fade-scale blur 叠在同一层导致卡死 -->
  <div class="overlay-container">
    <!-- 全应用共用的舞台背景，见 shell/StageBackdrop.vue -->
    <StageBackdrop />

    <!-- 头部区域 -->
    <div class="header">
      <div class="header-section">
        <button
          class="back-btn"
          @click="goBack"
          :aria-label="viewMode === 'editor' ? t('overlay.backToList') : t('common.backToHome')"
        >
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

    <!-- 列表 / 编辑：视图切换 -->
    <div class="flip-viewport">
      <Transition name="view-fade" mode="out-in">
        <div v-if="viewMode === 'list'" key="list" class="list-view">
          <!--
            工具栏：两个入口从原来占半屏的「快捷开始」面板搬到这里。
            它们是一次性动作，不是常驻内容 —— 放在标题行的右侧，
            视线扫过标题时顺手就能点到，不占用内容的宽度。
          -->
          <div class="list-toolbar">
            <span class="meta-label">
              {{ t('overlay.myProjects') }}
              <b v-if="overlayHistory.length">{{ overlayHistory.length }}</b>
            </span>
            <div class="toolbar-actions">
              <button class="ghost-btn" @click="showImportDialog = true">
                <i class="ri-download-cloud-2-line" aria-hidden="true"></i>
                <span>{{ t('overlay.importCode') }}</span>
              </button>
              <button class="primary-btn" @click="showCreateDialog = true">
                <i class="ri-add-line" aria-hidden="true"></i>
                <span>{{ t('overlay.newProject') }}</span>
              </button>
            </div>
          </div>

          <!-- 空态：一句话 + 一个入口，不画框 -->
          <div v-if="overlayHistory.length === 0" class="list-empty">
            <p class="empty-title">{{ t('overlay.emptyHistory') }}</p>
            <p class="empty-hint">{{ t('overlay.quickStartHint') }}</p>
          </div>

          <!--
            项目列表：整宽、表格式。
            原来是双栏网格里的一半宽度 —— 每行只有名字和日期两条信息，
            横向却铺得很开，密度反而低。整宽之后行高收紧、信息靠左成列，
            一屏能看到的项目数明显变多。
          -->
          <div v-else class="list-table">
            <div class="list-head">
              <span>{{ t('overlay.myProjects') }}</span>
              <span>{{ t('overlay.updatedAt') }}</span>
              <span></span>
            </div>
            <div
              v-for="item in overlayHistory"
              :key="item.id"
              class="list-row"
              @click="loadOverlay(item)"
            >
              <span class="col-name">
                <i class="ri-file-copy-line" aria-hidden="true"></i>
                {{ item.name }}
              </span>
              <span class="col-time">{{ formatDate(item.updatedAt) }}</span>
              <span class="col-ops">
                <button class="icon-btn share-btn" @click.stop="exportShareCode(item)" :title="t('overlay.exportCode')">
                  <i class="ri-share-forward-line" aria-hidden="true"></i>
                </button>
                <button class="icon-btn delete-btn" @click.stop="deleteOverlay(item)" :title="t('common.delete')">
                  <i class="ri-delete-bin-line" aria-hidden="true"></i>
                </button>
              </span>
            </div>
          </div>
        </div>

    <!-- 编辑模式: 左配置 / 右功能，主按钮落在右栏底部 -->
    <div v-else key="editor" class="editor">
      <!--
        母包是前提，不是「第一步」——它单独一行放在顶部，不进任何序列。
      -->
      <div class="editor-prereq">
        <span class="meta-label">{{ t('overlay.parentPackPath') }}</span>
        <span class="path-value">{{ currentOverlay.parentPackPath || t('overlay.notSelected') }}</span>
        <button class="select-pack-btn" @click="selectParentPack">
          <i class="ri-folder-open-line" aria-hidden="true"></i>
          <span>{{ t('overlay.selectParent') }}</span>
        </button>
      </div>

      <!--
        三个修改项：并列、等权、无先后。
        原来是一条带 → 的纵向列表 + 末尾挂打包按钮，读起来像「按顺序走三步」；
        但它们其实互不依赖、随时可改，所以改成横向等分的独立块，
        去掉箭头（箭头暗示「进入下一项」），也去掉序号。
      -->
      <div class="editor-mods">
        <span class="meta-label">{{ t('overlay.optionsTitle') }}</span>
        <div class="mod-row">
          <button
            v-for="(option, i) in options"
            :key="option.id"
            class="mod-tile"
            :style="{ '--i': i }"
            @click="option.action"
          >
            <span class="mod-icon"><i :class="option.icon" aria-hidden="true"></i></span>
            <span class="mod-title">{{ option.title }}</span>
            <span class="mod-desc">{{ option.description }}</span>
          </button>
        </div>
      </div>

      <!-- 全局动作单独一行，不属于任何修改项 -->
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
      </Transition>
    </div>

    <CreateProjectDialog v-model="showCreateDialog" @create="handleCreateProject" />
    <ImportShareDialog v-model="showImportDialog" @import="handleImportShareCode" />
    <ExportShareDialog v-model="showExportDialog" :code="exportedShareCode" @copy="copyShareCode" />
    <DeleteProjectDialog
      v-model="showDeleteDialog"
      :name="pendingDelete?.name ?? ''"
      @confirm="confirmDeleteOverlay"
    />

    <!-- 自定义内容：遮罩/面板各自 Transition（对齐转换页版本选择器） -->
    <ItemNameDialog v-model="showCustomNameDialog" :projectName="currentOverlay.name" />
    <ItemSizeDialog v-model="showItemSizeDialog" :projectName="currentOverlay.name" />
    <VisualDialog v-model="showVisualDialog" :projectName="currentOverlay.name" />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, reactive } from 'vue';
import StageBackdrop from './shell/StageBackdrop.vue';
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core';
import { pickFiles, pickSavePath } from '../composables/useFilePicker';
import ItemNameDialog from './ItemNameDialog.vue';
import ItemSizeDialog from './ItemSizeDialog.vue';
import VisualDialog from './VisualDialog.vue';
import CreateProjectDialog from './overlay/CreateProjectDialog.vue';
import ImportShareDialog from './overlay/ImportShareDialog.vue';
import ExportShareDialog from './overlay/ExportShareDialog.vue';
import DeleteProjectDialog from './overlay/DeleteProjectDialog.vue';
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
const exportedShareCode = ref('');

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

/** 分层返回：编辑器 → 项目列表 → 首页 */
const goBack = () => {
  if (viewMode.value === 'editor') {
    viewMode.value = 'list';
    return;
  }
  emit('switch-page', 'home');
};

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

const handleCreateProject = async (name: string) => {
  if (!name) return;
  statusMsg.value = null;
  try {
    const newProject = await invoke<OverlayProject>('overlay_init', {
      request: {
        name,
        parentPackPath: ''
      }
    });
    overlayHistory.value.unshift(newProject);
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

const handleImportShareCode = async (shareCode: string) => {
  if (!shareCode) return;
  try {
    const newProject = await invoke<any>('import_overlay_share_code', { shareCode });
    const processedProject = {
      id: newProject.id,
      name: newProject.name,
      parentPackPath: newProject.parentPackPath || '',
      updatedAt: newProject.updatedAt || Date.now()
    };
    overlayHistory.value.unshift(processedProject);
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
    const selected = await pickFiles({
      titleKey: 'overlay.resourcePackFilter',
      extensions: ['zip', 'mcpack'],
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
  const defaultName = (currentOverlay.name || t('overlay.defaultPackName')).replace(/[\\/:*?"<>|]/g, '_');
  let picked: string | null = null;
  try {
    picked = await pickSavePath({
      extensions: ['zip'],
      defaultName: `${defaultName}.zip`,
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
  box-shadow: 0 4px 12px rgba(20, 23, 26, 0.05);
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
.icon-btn:hover { background: rgba(20, 23, 26, 0.05); color: #1d1d1f; }
.delete-btn { color: #94a3b8; }
.delete-btn:hover { color: #ef4444; background: #fee2e2; }

/* 右栏：快速开始 */





/* 编辑：母包配置 */

.meta-label { font-size: 13px; font-weight: 600; color: #64748b; }


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
  border: 1px solid rgba(120, 110, 95, 0.2);
  border-radius: 8px;
  background: transparent;
  color: #1d1d1f;
  font-family: inherit;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: border-color 0.15s ease, background 0.15s ease;
}

.select-pack-btn:hover {
  border-color: color-mix(in srgb, #14171a 35%, transparent);
  background: rgba(120, 110, 95, 0.055);
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
  background: #14171a;
  color: #fff;
  border: none;
  box-shadow: 0 6px 16px color-mix(in srgb, #14171a 28%, transparent);
  transition: background 0.18s ease, transform 0.15s ease, box-shadow 0.18s ease, opacity 0.18s ease;
}
.package-btn:hover:not(:disabled) {
  background: color-mix(in srgb, #14171a 88%, #000);
  box-shadow: 0 8px 18px color-mix(in srgb, #14171a 32%, transparent);
}
.package-btn:disabled { opacity: 0.45; cursor: not-allowed; box-shadow: none; }

/* 弹窗样式 */

.simple-dialog {
  background: rgba(120, 110, 95, 0.055); padding: 32px; border-radius: 24px; width: 400px;
  /* 原来 0 25px 50px -12px rgba(20, 23, 26, 0.25) — blur 50px 太大,leave 期间
     opacity 渐变时 shadow 跟着渐变,大 blur 让 shadow "淡出拖尾" 看起来
     持续时间比 dialog 本身还长。改紧凑点(blur 32, opacity 0.18),跟
     leave 550ms 同步消失。 */
  box-shadow: 0 16px 32px -8px rgba(20, 23, 26, 0.18);
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

.project-input:focus { border-color: #14171a; background: #fff; }

.dialog-footer { display: flex; justify-content: flex-end; gap: 12px; }

.ghost-btn { padding: 10px 20px; border-radius: 12px; border: none; background: transparent; color: #64748b; font-weight: 700; cursor: pointer; }
.ghost-btn:hover { background: #f1f5f9; color: #0f172a; }

.primary-btn {
  padding: 10px 20px; border-radius: 12px; border: none;
  background: #14171a; color: #fff; font-weight: 700; cursor: pointer;
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

/* 列表 ↔ 编辑：淡入淡出 + 轻微缩放（out-in，干净不叠影） */
/* 旧的 view-fade（用 scale 的那份）已删除 —— 见文件末尾的新定义。
   scale 会让容器成为包含块，过渡期间里面的 fixed 浮层会错位。 */

/* dialog enter/leave 走全局 CSS class 模式(`<transition name="dialog-pop">`,
   见 App.vue 全局 .dialog-pop-* 规则)。Vue 3 在 element insert 时直接加
   enter-from class,跟 element 同一个 commit,第一帧 paint 一定看到 from 状态
   → 杜绝「打开瞬间闪一下」。 */

.spin { animation: ri-spin 1s linear infinite; }
@keyframes ri-spin { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }
/* ══════════ 覆盖页的面 ══════════
 *
 * 这些规则以前由 App.vue 的全局卡片系统提供（玻璃面 + blur + 投影 + 大圆角）。
 * 那份名单已把 .card 摘掉，所以这里补上页面自己的定义 —— 与设置页、转换页同一套：
 * 发丝线分层、8px 圆角、不用投影、靠边线与留白而不是靠明暗块。
 */
.card {
  /* 不再是卡片，只是网格里的一个分区。面交给背景，分区之间靠留白与发丝线。 */
  background: transparent;
  border: none;
  border-radius: 0;
  box-shadow: none;
  backdrop-filter: none;
  -webkit-backdrop-filter: none;
}

/* 卡片标题：跟随首页的小字规范 —— 靠字号与字距，不靠加粗 */

/* 列表行：发丝线分隔，最后一行不画线 */



/* ══════════ 编辑视图：无先后的并列布局 ══════════
 *
 * 三者互不依赖、随时可改，所以：
 *   · 母包 = 前提，单独一行置顶
 *   · 三个修改项 = 横向等分的独立块，等权、无箭头、无序号
 *   · 打包 = 全局动作，单独一行靠右
 * 箭头（→）暗示「进入下一项」，序号暗示「第几步」，两者都拿掉了。
 */
.editor {
  flex: 1;
  min-height: 0;
  z-index: 2;
  padding: 12px 40px 28px;
  display: flex;
  flex-direction: column;
  gap: 26px;
  overflow-y: auto;
}

.editor-prereq {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
  padding-bottom: 14px;
  border-bottom: 1px solid rgba(120, 110, 95, 0.14);
}
.editor-prereq .path-value {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 12px;
  color: #14171a;
}

.editor-mods { display: flex; flex-direction: column; gap: 12px; }

.editor-mods .meta-label,
.editor-prereq .meta-label {
  font-size: 10.5px;
  font-weight: 400;
  letter-spacing: 0.18em;
  text-transform: uppercase;
  color: #7d776d;
}

/* 三项等分。用 auto-fit 而不是固定三列 —— 窗口窄了会自动折行，不挤压 */
.mod-row {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 12px;
}

.mod-tile {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 6px;
  padding: 16px 16px 18px;
  text-align: left;
  font-family: inherit;
  cursor: pointer;
  /* 与设置页、转换页同一套：发丝线 + 8px + 无投影 */
  background: rgba(255, 255, 255, 0.6);
  border: 1px solid rgba(120, 110, 95, 0.16);
  border-radius: 8px;
  transition: background 0.18s ease, border-color 0.18s ease, transform 0.24s cubic-bezier(0.34, 1.56, 0.64, 1);
  /*
   * 入场：错开一点点即可，不要做成「依次点亮」——
   * 它们没有先后，动画也不该暗示有。延迟差 55ms，读起来是一起出现。
   * fill-mode 用 backwards：both / forwards 会永久保留 translate: 0 0，
   * 元素成为包含块，里面的 fixed 浮层会错位。
   */
  animation: mod-in 420ms var(--ease-out, cubic-bezier(0.22, 1, 0.36, 1)) backwards;
  animation-delay: calc(var(--i, 0) * 55ms);
}
.mod-tile:hover {
  background: rgba(120, 110, 95, 0.055);
  border-color: rgba(120, 110, 95, 0.3);
  transform: translateY(-2px);
}
.mod-tile:active { transform: translateY(0); }

.mod-icon { color: #14171a; font-size: 19px; line-height: 1; }
.mod-title { font-size: 14px; font-weight: 600; color: #14171a; }
.mod-desc { font-size: 12px; font-weight: 400; line-height: 1.5; color: #7d776d; }

/* 全局动作：靠右，与其它页面的 stage-actions 一致 */
.editor-actions {
  margin-top: auto;
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  padding-top: 16px;
  border-top: 1px solid rgba(120, 110, 95, 0.14);
}

@keyframes mod-in {
  from { opacity: 0; translate: 0 10px; }
  to   { opacity: 1; translate: 0 0; }
}

@media (prefers-reduced-motion: reduce) {
  .mod-tile { animation-duration: 1ms; animation-delay: 0ms; transition: none; }
}

/* ══════════ 列表视图：单栏表格式 ══════════
 *
 * 排版上的三个决定：
 *   ① 整宽单栏。原来双栏网格里列表只占一半宽度，每行却只有名字+日期两条信息，
 *      横向铺得很开、密度反而低。整宽后名字有时间同列对齐，一屏能看到更多项目。
 *   ② 入口搬到工具栏。新建/导入是一次性动作，不该常驻半个屏幕。
 *   ③ 行高收紧到 44px，靠发丝线分隔，不靠间距拉开。
 */
.list-view {
  flex: 1;
  min-height: 0;
  z-index: 2;
  padding: 12px 40px 28px;
  display: flex;
  flex-direction: column;
  gap: 16px;
  overflow-y: auto;
}

.list-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  flex-wrap: wrap;
}
.list-toolbar .meta-label {
  display: inline-flex;
  align-items: baseline;
  gap: 7px;
  font-size: 10.5px;
  font-weight: 400;
  letter-spacing: 0.18em;
  text-transform: uppercase;
  color: #7d776d;
}
.list-toolbar .meta-label b {
  font-size: 12px;
  font-weight: 600;
  letter-spacing: 0;
  color: #14171a;
  font-variant-numeric: tabular-nums;
}
.toolbar-actions { display: inline-flex; align-items: center; gap: 8px; }

/* ── 表格 ── */
.list-table { display: flex; flex-direction: column; }

/* 列宽共用同一组定义，表头与行不会错位 */
.list-head,
.list-row {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 168px 76px;
  align-items: center;
  gap: 16px;
}

.list-head {
  padding: 0 4px 8px;
  border-bottom: 1px solid rgba(120, 110, 95, 0.2);
  font-size: 10px;
  font-weight: 400;
  letter-spacing: 0.16em;
  text-transform: uppercase;
  color: #9c968c;
}

.list-row {
  min-height: 44px;
  padding: 0 4px;
  border-bottom: 1px solid rgba(120, 110, 95, 0.11);
  border-radius: 6px;
  cursor: pointer;
  transition: background 0.15s ease;
}
.list-row:last-child { border-bottom: none; }
.list-row:hover { background: rgba(120, 110, 95, 0.05); }

.col-name {
  display: inline-flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
  font-size: 13.5px;
  color: #14171a;
  /* 名字过长时省略，不换行 —— 换行会破坏表格式的行高一致性 */
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.col-name i { font-size: 15px; color: #9c968c; flex-shrink: 0; }

.col-time {
  font-size: 12px;
  color: #7d776d;
  font-variant-numeric: tabular-nums;
}

.col-ops {
  display: inline-flex;
  align-items: center;
  justify-content: flex-end;
  gap: 2px;
  /* 平时不显形，悬停才出现 —— 行是主体，操作是附加的 */
  opacity: 0;
  transition: opacity 0.15s ease;
}
.list-row:hover .col-ops,
.list-row:focus-within .col-ops { opacity: 1; }

/* 触屏/无悬停设备上没有 hover，操作必须常驻，否则点不到 */
@media (hover: none) {
  .col-ops { opacity: 1; }
}

/* ── 空态 ── */
.list-empty {
  margin-top: 48px;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  text-align: center;
}
.list-empty .empty-title { margin: 0; font-size: 14px; color: #5f5a51; }
.list-empty .empty-hint { margin: 0; font-size: 12px; color: #9c968c; }

@media (max-width: 720px) {
  .list-head { display: none; }
  .list-row { grid-template-columns: minmax(0, 1fr) auto; }
  .col-time { display: none; }
}

/*
 * 视图切换（列表 ↔ 编辑）。
 * 原来用 scale(0.985) —— transform 会让容器成为包含块，
 * 过渡期间里面的 position: fixed 浮层会相对它定位。
 * 改成透明度 + 位移，且不给 fill-mode，过渡结束即回到自然样式。
 */
.view-fade-enter-active { transition: opacity 0.24s ease, translate 0.24s cubic-bezier(0.22, 1, 0.36, 1); }
.view-fade-leave-active { transition: opacity 0.16s ease, translate 0.16s ease; }
.view-fade-enter-from { opacity: 0; translate: 0 10px; }
.view-fade-leave-to { opacity: 0; translate: 0 -6px; }

</style>
