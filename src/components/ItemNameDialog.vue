<template>
  <div class="dialog-overlay">
    <div class="dialog-container dialog-content">
      <div class="dialog-header">
        <h2 class="dialog-title">{{ t('dialog.itemName.title') }}</h2>
        <div class="header-actions">
          <button 
            v-if="hasParentPack" 
            class="ghost-btn import-btn" 
            @click="importFromParent"
            :title="t('dialog.itemName.importHint')"
          >
            <i class="ri-download-cloud-2-line"></i>
            <span>{{ t('dialog.itemName.syncParent') }}</span>
          </button>
          <select v-model="selectedLang" class="lang-select" @change="loadLangFile">
            <option value="zh_cn">简体中文 (zh_cn)</option>
            <option value="en_us">English (en_us)</option>
          </select>
          <button class="icon-button close-btn" @click="closeDialog" :aria-label="t('common.close')">
            <i class="ri-close-line"></i>
          </button>
        </div>
      </div>

      <div class="dialog-content">
        <div class="search-bar-container">
          <div class="search-input-wrapper">
            <i class="ri-search-line search-icon"></i>
            <input 
              v-model="searchText" 
              class="search-input" 
              :placeholder="t('dialog.itemName.searchPlaceholder')"
            />
          </div>
          <div class="items-count">
            {{ t('dialog.itemName.itemCount', { count: filteredEntries.length }) }}
          </div>
        </div>

        <div class="lang-list-container" ref="listContainer" @scroll="handleScroll">
          <div v-if="isLoading" class="loading-state">
            <i class="ri-loader-4-line spin"></i>
            <span>{{ t('dialog.itemName.loading') }}</span>
          </div>
          <div v-else-if="filteredEntries.length === 0" class="empty-state">
            <i class="ri-inbox-line"></i>
            <span>{{ t('dialog.itemName.noMatch') }}</span>
          </div>
          <div v-else class="lang-grid">
            <div 
              v-for="entry in displayEntries" 
              :key="entry.key" 
              class="lang-item"
              :class="{ 'is-modified': isModified(entry.key) }"
            >
              <div class="item-info">
                <code class="item-key">{{ entry.key.replace('item.minecraft.', '') }}</code>
                <div class="preview-container" v-if="editedData[entry.key]">
                  <span class="preview-label">{{ t('dialog.itemName.preview') }}</span>
                  <span class="mc-preview" v-html="renderMinecraftText(editedData[entry.key])"></span>
                </div>
                <span class="original-val" v-if="isModified(entry.key)">
                  {{ t('dialog.itemName.original', { value: langData[entry.key] }) }}
                </span>
              </div>
              <div class="item-edit">
                <div class="input-group">
                  <input 
                    v-model="editedData[entry.key]" 
                    class="edit-input" 
                    :placeholder="t('dialog.itemName.editPlaceholder')"
                    @focus="activeKey = entry.key"
                  />
                  <button 
                    class="style-btn" 
                    @click.stop="toggleStylePanel($event, entry.key)"
                    title=""
                  >
                    <i class="ri-palette-line"></i>
                  </button>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- 样式悬浮面板 -->
      <Teleport to="body">
        <div 
          v-if="showStylePanel" 
          class="style-panel" 
          :style="stylePanelPos"
          @mousedown.stop
        >
          <div class="style-section">
            <span class="section-label">{{ t('dialog.itemName.colors') }}</span>
            <div class="color-grid">
              <button 
                v-for="c in minecraftColors" 
                :key="c.code" 
                class="color-swatch"
                :style="{ background: c.hex }"
                :title="c.name"
                @click="insertFormat(c.code)"
              ></button>
            </div>
          </div>
          <div class="style-section">
            <span class="section-label">{{ t('dialog.itemName.formatting') }}</span>
            <div class="format-grid">
              <button 
                v-for="f in minecraftFormats" 
                :key="f.code" 
                class="format-btn"
                @click="insertFormat(f.code)"
                :title="f.name"
              >
                <span :style="f.style">{{ f.label }}</span>
              </button>
              <button class="format-btn reset" @click="insertFormat('r')" :title="t('dialog.itemName.resetStyle')">
                <i class="ri-format-clear"></i>
              </button>
            </div>
          </div>
        </div>
      </Teleport>

      <div class="dialog-footer">
        <div class="modified-hint" v-if="modifiedKeys.length > 0">
          {{ t('dialog.itemName.modified', { count: modifiedKeys.length }) }}
        </div>
        <div class="save-status" v-if="saveStatus" :class="saveStatus.type">
          <i :class="saveStatus.type === 'success' ? 'ri-checkbox-circle-line' : 'ri-error-warning-line'"></i>
          {{ saveStatus.text }}
        </div>
        <div class="footer-btns">
          <button class="ghost-btn" @click="closeDialog">{{ t('common.cancel') }}</button>
          <button
            class="primary-btn"
            :disabled="isSaving || modifiedKeys.length === 0"
            @click="saveAndClose"
          >
            <i class="ri-save-line" v-if="!isSaving"></i>
            <i class="ri-loader-4-line spin" v-else></i>
            {{ isSaving ? t('dialog.itemName.saving') : t('dialog.itemName.saveChanges') }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { invoke } from '@tauri-apps/api/core';
import { message } from '@tauri-apps/plugin-dialog';

const { t } = useI18n();

const props = defineProps<{
  projectName: string
}>();

const emit = defineEmits<{ close: [] }>();

const selectedLang = ref('zh_cn');
const searchText = ref('');
const isSaving = ref(false);
const isLoading = ref(false);
const saveStatus = ref<{ text: string, type: 'success' | 'error' } | null>(null);
const langData = ref<Record<string, string>>({});
const editedData = ref<Record<string, string>>({});
const activeKey = ref<string | null>(null);

// 样式面板相关
const showStylePanel = ref(false);
const stylePanelPos = ref({ top: '0px', left: '0px' });
const currentEditingKey = ref<string | null>(null);

const minecraftColors = [
  { code: '0', hex: '#000000', name: t('dialog.itemName.colorNames.black') },
  { code: '1', hex: '#0000AA', name: t('dialog.itemName.colorNames.dark_blue') },
  { code: '2', hex: '#00AA00', name: t('dialog.itemName.colorNames.dark_green') },
  { code: '3', hex: '#00AAAA', name: t('dialog.itemName.colorNames.dark_aqua') },
  { code: '4', hex: '#AA0000', name: t('dialog.itemName.colorNames.dark_red') },
  { code: '5', hex: '#AA00AA', name: t('dialog.itemName.colorNames.dark_purple') },
  { code: '6', hex: '#FFAA00', name: t('dialog.itemName.colorNames.gold') },
  { code: '7', hex: '#AAAAAA', name: t('dialog.itemName.colorNames.gray') },
  { code: '8', hex: '#555555', name: t('dialog.itemName.colorNames.dark_gray') },
  { code: '9', hex: '#5555FF', name: t('dialog.itemName.colorNames.blue') },
  { code: 'a', hex: '#55FF55', name: t('dialog.itemName.colorNames.green') },
  { code: 'b', hex: '#55FFFF', name: t('dialog.itemName.colorNames.aqua') },
  { code: 'c', hex: '#FF5555', name: t('dialog.itemName.colorNames.red') },
  { code: 'd', hex: '#FF55FF', name: t('dialog.itemName.colorNames.light_purple') },
  { code: 'e', hex: '#FFFF55', name: t('dialog.itemName.colorNames.yellow') },
  { code: 'f', hex: '#FFFFFF', name: t('dialog.itemName.colorNames.white') },
];

const minecraftFormats = [
  { code: 'l', label: 'B', name: t('dialog.itemName.formatNames.bold'), style: { fontWeight: 'bold' } },
  { code: 'o', label: 'I', name: t('dialog.itemName.formatNames.italic'), style: { fontStyle: 'italic' } },
  { code: 'n', label: 'U', name: t('dialog.itemName.formatNames.underline'), style: { textDecoration: 'underline' } },
  { code: 'm', label: 'S', name: t('dialog.itemName.formatNames.strikethrough'), style: { textDecoration: 'line-through' } },
  { code: 'k', label: '?', name: t('dialog.itemName.formatNames.obfuscated'), style: {} },
];

const hasParentPack = ref(false);

const checkParentPack = async () => {
  try {
    const projects = await invoke<any[]>('get_overlay_projects');
    const project = projects.find(p => p.name === props.projectName);
    if (project && project.parentPackPath) {
      hasParentPack.value = true;
    }
  } catch {
    // ignore
  }
};

const importFromParent = async () => {
  isLoading.value = true;
  try {
    const data = await invoke<Record<string, string>>('import_lang_from_parent', { 
      projectName: props.projectName,
      langCode: selectedLang.value 
    });
    langData.value = data;
    // 重置编辑数据为新导入的数据，但保留已有的修改
    const currentProjectData = await invoke<any>('get_overlay_json', { projectName: props.projectName });
    const savedNames = currentProjectData.custom_names || {};
    
    const initialData = { ...data };
    Object.keys(savedNames).forEach(key => {
      if (initialData[key] !== undefined) {
        initialData[key] = savedNames[key];
      }
    });
    editedData.value = initialData;
    await message(t('dialog.itemName.synced'), { title: t('dialog.itemName.syncSuccess'), kind: 'info' });
  } catch (error) {
    console.error('导入失败:', error);
    await message(t('dialog.itemName.importFailed', { error }), { title: t('common.error'), kind: 'error' });
  } finally {
    isLoading.value = false;
  }
};

const loadLangFile = async () => {
  isLoading.value = true;
  try {
    // 1. 保存当前选择的语言偏好
    await invoke('save_overlay_lang', { lang: selectedLang.value });
    
    // 2. 读取语言文件内容
    const data = await invoke<Record<string, string>>('read_lang_file', { langCode: selectedLang.value });
    langData.value = data;
    
    // 3. 从项目配置中还原已修改的内容
    const projectData = await invoke<any>('get_overlay_json', { projectName: props.projectName });
    const savedNames = projectData.custom_names || {};
    
    // 4. 初始化编辑数据
    const initialData = { ...data };
    Object.keys(savedNames).forEach(key => {
      if (initialData[key] !== undefined) {
        initialData[key] = savedNames[key];
      }
    });
    editedData.value = initialData;
  } catch (error) {
    console.error('加载语言文件失败:', error);
    await message(t('dialog.itemName.loadFailed', { error }), { title: t('common.error'), kind: 'error' });
  } finally {
    isLoading.value = false;
  }
};

const filteredEntries = computed(() => {
  const search = searchText.value.toLowerCase();
  const entries = Object.entries(editedData.value).map(([key, val]) => ({ key, val }));
  
  if (!search) return entries;
  
  return entries.filter(e => 
    e.key.toLowerCase().includes(search) || 
    (e.val && e.val.toLowerCase().includes(search)) ||
    e.key.replace('item.minecraft.', '').toLowerCase().includes(search)
  );
});

// 使用简单的分页逻辑来优化渲染性能
const pageSize = ref(100);
const currentPage = ref(1);

const displayEntries = computed(() => {
  return filteredEntries.value.slice(0, currentPage.value * pageSize.value);
});

const handleScroll = (e: Event) => {
  const target = e.target as HTMLElement;
  if (target.scrollHeight - target.scrollTop - target.clientHeight < 200) {
    if (displayEntries.value.length < filteredEntries.value.length) {
      currentPage.value++;
    }
  }
};

watch(searchText, () => {
  currentPage.value = 1;
});

const modifiedKeys = computed(() => {
  return Object.keys(editedData.value).filter(key => editedData.value[key] !== langData.value[key]);
});

const isModified = (key: string) => editedData.value[key] !== langData.value[key];

const toggleStylePanel = (event: MouseEvent, key: string) => {
  if (showStylePanel.value && currentEditingKey.value === key) {
    showStylePanel.value = false;
    return;
  }
  
  currentEditingKey.value = key;
  const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
  stylePanelPos.value = {
    top: `${rect.bottom + 5}px`,
    left: `${rect.left - 200}px`
  };
  showStylePanel.value = true;
};

const insertFormat = (code: string) => {
  if (!currentEditingKey.value) return;
  const currentVal = editedData.value[currentEditingKey.value] || '';
  editedData.value[currentEditingKey.value] = currentVal + '§' + code;
};

// Minecraft 文本渲染逻辑
const renderMinecraftText = (text: string) => {
  if (!text) return '';
  
  let html = '';
  let currentStyles: Record<string, string> = {};
  let currentClasses: string[] = [];
  
  const colorMap: Record<string, string> = {
    '0': '#000000', '1': '#0000AA', '2': '#00AA00', '3': '#00AAAA',
    '4': '#AA0000', '5': '#AA00AA', '6': '#FFAA00', '7': '#AAAAAA',
    '8': '#555555', '9': '#5555FF', 'a': '#55FF55', 'b': '#55FFFF',
    'c': '#FF5555', 'd': '#FF55FF', 'e': '#FFFF55', 'f': '#FFFFFF'
  };
  
  const formatMap: Record<string, { style?: string, class?: string }> = {
    'l': { style: 'font-weight: bold' },
    'o': { style: 'font-style: italic' },
    'n': { style: 'text-decoration: underline' },
    'm': { style: 'text-decoration: line-through' },
    'k': { class: 'mc-obfuscated' }
  };

  const parts = text.split('§');
  html += parts[0]; // 第一个部分没有格式代码

  for (let i = 1; i < parts.length; i++) {
    const part = parts[i];
    if (part.length === 0) continue;
    
    const code = part[0].toLowerCase();
    const content = part.substring(1);
    
    if (code === 'r') {
      currentStyles = {};
      currentClasses = [];
    } else if (colorMap[code]) {
      currentStyles = { color: colorMap[code] };
      currentClasses = []; // 颜色代码会重置格式代码
    } else if (formatMap[code]) {
      const fmt = formatMap[code];
      if (fmt.style) {
        const [prop, val] = fmt.style.split(': ');
        currentStyles[prop] = val;
      }
      if (fmt.class) currentClasses.push(fmt.class);
    }
    
    const styleStr = Object.entries(currentStyles).map(([k, v]) => `${k}:${v}`).join(';');
    const classStr = currentClasses.join(' ');
    html += `<span style="${styleStr}" class="${classStr}">${content}</span>`;
  }
  
  return html;
};

const saveAndClose = async () => {
  isSaving.value = true;
  saveStatus.value = null;
  try {
    // 获取当前已修改的条目
    const currentUpdates: Record<string, string> = {};
    modifiedKeys.value.forEach(key => {
      currentUpdates[key] = editedData.value[key];
    });

    // 读取现有的项目配置以进行合并
    const currentProjectData = await invoke<any>('get_overlay_json', { projectName: props.projectName });
    
    // 合并 custom_names
    const mergedData = { 
      ...currentProjectData, 
      custom_names: {
        ...(currentProjectData.custom_names || {}),
        ...currentUpdates
      }
    };

    await invoke('save_overlay_json', { 
      projectName: props.projectName, 
      data: mergedData 
    });
    
    saveStatus.value = { text: t('dialog.itemName.allSaved'), type: 'success' };
    
    // 延迟 1.5 秒后关闭，让用户看到成功状态
    setTimeout(() => {
      emit('close');
    }, 1500);
  } catch (error) {
    console.error('保存失败:', error);
    saveStatus.value = { text: t('dialog.itemName.saveFailed', { error }), type: 'error' };
  } finally {
    isSaving.value = false;
  }
};

const closeDialog = () => {
  emit('close');
};

const handleGlobalClick = () => {
  if (showStylePanel.value) showStylePanel.value = false;
};

onMounted(async () => {
  window.addEventListener('mousedown', handleGlobalClick);
  await checkParentPack();
  
  try {
    const savedLang = await invoke<string>('get_overlay_lang');
    if (savedLang) selectedLang.value = savedLang;
  } catch {
    // ignore
  }
  
  await loadLangFile();
});
</script>

<style scoped>
.dialog-overlay {
  position: fixed;
  inset: 0;
  background: rgba(15, 23, 42, 0.4);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  backdrop-filter: blur(12px);
  animation: overlay-fade 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

.dialog-container {
  width: min(920px, 96vw);
  height: 85vh;
  background: #ffffff;
  border-radius: 20px;
  /* 原来 0 25px 50px -12px rgba(0,0,0,0.25) — blur 50px 太大,leave 期间
     shadow 跟着 opacity 渐变时 "淡出拖尾" 比 dialog 本身还久。改紧凑点
     跟 550ms leave 同步消失。 */
  box-shadow: 0 16px 32px -8px rgba(0, 0, 0, 0.18);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  border: 1px solid rgba(0, 0, 0, 0.05);
  animation: panel-scale-up 0.3s cubic-bezier(0.34, 1.56, 0.64, 1);
}

.dialog-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 20px 24px;
  background: #f8fafc;
  border-bottom: 1px solid #e2e8f0;
}

.dialog-title {
  margin: 0;
  font-size: 20px;
  font-weight: 800;
  color: #0f172a;
  letter-spacing: -0.02em;
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 12px;
}

.lang-select {
  padding: 8px 12px;
  border-radius: 10px;
  border: 1px solid #e2e8f0;
  background: #fff;
  font-size: 14px;
  font-weight: 600;
  color: #475569;
  cursor: pointer;
  outline: none;
  transition: all 0.2s;
}

.lang-select:hover {
  border-color: var(--theme-color);
  background: #f1f5f9;
}

.close-btn {
  width: 36px;
  height: 36px;
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #64748b;
  transition: all 0.2s;
}

.close-btn:hover {
  background: #fee2e2;
  color: #ef4444;
}

.dialog-content {
  flex: 1;
  padding: 24px;
  display: flex;
  flex-direction: column;
  gap: 20px;
  overflow: hidden;
}

.search-bar-container {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
}

.search-input-wrapper {
  flex: 1;
  position: relative;
}

.search-icon {
  position: absolute;
  left: 14px;
  top: 50%;
  transform: translateY(-50%);
  color: #94a3b8;
  font-size: 18px;
}

.search-input {
  width: 100%;
  padding: 12px 16px 12px 44px;
  border-radius: 14px;
  border: 2px solid #f1f5f9;
  background: #f8fafc;
  font-size: 15px;
  transition: all 0.2s;
  outline: none;
}

.search-input:focus {
  border-color: var(--theme-color);
  background: #fff;
  box-shadow: 0 0 0 4px rgba(var(--theme-color-rgb), 0.1);
}

.items-count {
  font-size: 13px;
  font-weight: 600;
  color: #94a3b8;
  white-space: nowrap;
}

.lang-list-container {
  flex: 1;
  overflow-y: auto;
  padding-right: 8px;
}

/* 自定义滚动条 */
.lang-list-container::-webkit-scrollbar {
  width: 6px;
}
.lang-list-container::-webkit-scrollbar-track {
  background: transparent;
}
.lang-list-container::-webkit-scrollbar-thumb {
  background: #e2e8f0;
  border-radius: 10px;
}
.lang-list-container::-webkit-scrollbar-thumb:hover {
  background: #cbd5e1;
}

.lang-grid {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.lang-item {
  display: grid;
  grid-template-columns: 1fr 1.2fr;
  gap: 20px;
  padding: 16px;
  background: #f8fafc;
  border-radius: 16px;
  border: 1px solid #f1f5f9;
  transition: all 0.2s;
}

.lang-item:hover {
  border-color: #e2e8f0;
  background: #fff;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.03);
}

.lang-item.is-modified {
  border-color: rgba(var(--theme-color-rgb), 0.3);
  background: rgba(var(--theme-color-rgb), 0.02);
}

.item-info {
  display: flex;
  flex-direction: column;
  gap: 4px;
  justify-content: center;
}

.item-key {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-size: 13px;
  color: #475569;
  font-weight: 600;
  background: #e2e8f0;
  padding: 2px 8px;
  border-radius: 6px;
  width: fit-content;
}

.preview-container {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 4px;
}

.preview-label {
  font-size: 11px;
  font-weight: 700;
  color: #94a3b8;
  text-transform: uppercase;
}

.mc-preview {
  font-family: 'Minecraft', sans-serif;
  font-size: 14px;
  background: #1e1e1e;
  padding: 2px 8px;
  border-radius: 4px;
  color: #fff;
  display: inline-block;
  min-height: 24px;
}

.mc-obfuscated {
  animation: mc-obf 0.1s steps(1) infinite;
}

@keyframes mc-obf {
  0% { opacity: 0.8; }
  50% { opacity: 1; }
}

.original-val {
  font-size: 12px;
  color: #94a3b8;
}

.item-edit {
  display: flex;
  align-items: center;
}

.input-group {
  width: 100%;
  display: flex;
  gap: 8px;
}

.edit-input {
  flex: 1;
  padding: 10px 14px;
  border-radius: 10px;
  border: 1px solid #e2e8f0;
  background: #fff;
  font-size: 14px;
  font-weight: 500;
  outline: none;
  transition: all 0.2s;
}

.edit-input:focus {
  border-color: var(--theme-color);
  box-shadow: 0 0 0 3px rgba(var(--theme-color-rgb), 0.1);
}

.style-btn {
  width: 40px;
  height: 40px;
  border-radius: 10px;
  border: 1px solid #e2e8f0;
  background: #fff;
  color: #64748b;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: all 0.2s;
}

.style-btn:hover {
  border-color: var(--theme-color);
  color: var(--theme-color);
  background: #f8fafc;
}

/* 样式面板 */
.style-panel {
  position: fixed;
  width: 260px;
  background: #ffffff;
  border-radius: 16px;
  box-shadow: 0 10px 30px rgba(0, 0, 0, 0.15);
  padding: 16px;
  border: 1px solid #e2e8f0;
  z-index: 2000;
  display: flex;
  flex-direction: column;
  gap: 16px;
  animation: style-panel-pop 0.2s cubic-bezier(0.34, 1.56, 0.64, 1);
}

.style-section {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.section-label {
  font-size: 12px;
  font-weight: 700;
  color: #94a3b8;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.color-grid {
  display: grid;
  grid-template-columns: repeat(8, 1fr);
  gap: 6px;
}

.color-swatch {
  aspect-ratio: 1;
  border-radius: 4px;
  border: 1px solid rgba(0, 0, 0, 0.1);
  cursor: pointer;
  transition: transform 0.1s;
}

.color-swatch:hover {
  transform: scale(1.2);
  z-index: 1;
}

.format-grid {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.format-btn {
  height: 32px;
  min-width: 32px;
  padding: 0 8px;
  border-radius: 6px;
  border: 1px solid #e2e8f0;
  background: #f8fafc;
  font-size: 14px;
  font-weight: 600;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.1s;
}

.format-btn:hover {
  background: #fff;
  border-color: #cbd5e1;
  transform: translateY(-1px);
}

.format-btn.reset {
  color: #ef4444;
}

.format-btn.reset:hover {
  background: #fee2e2;
  border-color: #fecaca;
}

.dialog-footer {
  padding: 16px 24px;
  background: #f8fafc;
  border-top: 1px solid #e2e8f0;
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.modified-hint {
  font-size: 13px;
  font-weight: 700;
  color: var(--theme-color);
  background: rgba(var(--theme-color-rgb), 0.1);
  padding: 4px 12px;
  border-radius: 20px;
}

.save-status {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
  font-weight: 700;
  padding: 4px 12px;
  border-radius: 20px;
  animation: panel-scale-up 0.3s cubic-bezier(0.34, 1.56, 0.64, 1);
}

.save-status.success {
  color: #10b981;
  background: rgba(16, 185, 129, 0.1);
}

.save-status.error {
  color: #ef4444;
  background: rgba(239, 68, 68, 0.1);
}

.footer-btns {
  display: flex;
  gap: 12px;
  margin-left: auto;
}

.ghost-btn {
  padding: 10px 20px;
  border-radius: 12px;
  border: none;
  background: transparent;
  font-size: 14px;
  font-weight: 700;
  color: #64748b;
  cursor: pointer;
  transition: all 0.2s;
}

.ghost-btn:hover {
  background: #f1f5f9;
  color: #0f172a;
}

.primary-btn {
  padding: 10px 24px;
  border-radius: 12px;
  border: none;
  background: var(--theme-color);
  color: #fff;
  font-size: 14px;
  font-weight: 700;
  display: flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
  transition: all 0.2s;
  box-shadow: 0 4px 12px rgba(var(--theme-color-rgb), 0.2);
}

.primary-btn:hover:not(:disabled) {
  transform: translateY(-2px);
  box-shadow: 0 6px 16px rgba(var(--theme-color-rgb), 0.3);
}

.primary-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
  filter: grayscale(0.5);
}

.loading-state, .empty-state {
  height: 200px;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  color: #94a3b8;
}

.loading-state i, .empty-state i {
  font-size: 48px;
}

.spin {
  animation: ri-spin 1s linear infinite;
}

@keyframes ri-spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

@keyframes overlay-fade {
  from { opacity: 0; }
  to { opacity: 1; }
}

@keyframes panel-scale-up {
  from { opacity: 0; transform: scale(0.95) translateY(10px); }
  to { opacity: 1; transform: scale(1) translateY(0); }
}

@keyframes style-panel-pop {
  from { opacity: 0; transform: scale(0.9) translateY(5px); }
  to { opacity: 1; transform: scale(1) translateY(0); }
}
</style>
