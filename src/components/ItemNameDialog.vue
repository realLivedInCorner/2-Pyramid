<template>
  <transition name="sidebar-overlay-fade">
    <div v-if="visible" class="sidebar-overlay" @click="visible = false"></div>
  </transition>

  <transition
    :css="false"
    @before-enter="onBeforeEnter"
    @enter="onEnter"
    @after-enter="onAfterEnter"
    @before-leave="onBeforeLeave"
    @leave="onLeave"
    @after-leave="onAfterLeave"
  >
    <aside v-if="visible" class="sidebar-content name-panel" @click.stop tabindex="-1">
      <div class="panel-header">
        <h2 class="panel-title">{{ t('dialog.itemName.title') }}</h2>
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
            <option value="zh_cn">{{ t('dialog.itemName.langZh') }}</option>
            <option value="en_us">{{ t('dialog.itemName.langEn') }}</option>
          </select>
        </div>
      </div>

      <div class="panel-body">
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
              <code class="item-key" :title="entry.key">{{ shortKey(entry.key) }}</code>
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
              <div class="item-meta" v-if="editedData[entry.key] || isModified(entry.key)">
                <span class="mc-preview" v-if="editedData[entry.key]" v-html="renderMinecraftText(editedData[entry.key])"></span>
                <span class="original-val" v-if="isModified(entry.key)">
                  {{ t('dialog.itemName.original', { value: langData[entry.key] }) }}
                </span>
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

      <div class="panel-footer">
        <div class="modified-hint" v-if="modifiedKeys.length > 0">
          {{ t('dialog.itemName.modified', { count: modifiedKeys.length }) }}
        </div>
        <div class="save-status" v-if="saveStatus" :class="saveStatus.type">
          <i :class="saveStatus.type === 'success' ? 'ri-checkbox-circle-line' : 'ri-error-warning-line'"></i>
          {{ saveStatus.text }}
        </div>
        <div class="footer-btns">
          <button class="primary-btn" :disabled="isSaving || modifiedKeys.length === 0" @click="saveAndClose">
            <i class="ri-save-line" v-if="!isSaving"></i>
            <i class="ri-loader-4-line spin" v-else></i>
            {{ isSaving ? t('common.saving') : t('dialog.itemName.saveChanges') }}
          </button>
          <button class="ghost-btn back-btn" @click="visible = false">
            <i class="ri-arrow-go-back-line"></i>
            <span>{{ t('common.back') }}</span>
          </button>
        </div>
      </div>
    </aside>
  </transition>
</template>

<script setup lang="ts">
import { ref, onMounted, computed, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { invoke } from '@tauri-apps/api/core';
import { message } from '@tauri-apps/plugin-dialog';
import { useSidebarSlide } from '../composables/useSidebarSlide';

const { t } = useI18n();

const visible = defineModel<boolean>({ required: true });

const props = defineProps<{
  projectName: string
}>();

// 抽屉投影：暖色中性，与 scoped 里的兜底态取同一个值（原来是冷灰 0 12px 48px）
const {
  onBeforeEnter,
  onEnter,
  onAfterEnter,
  onBeforeLeave,
  onLeave,
  onAfterLeave,
} = useSidebarSlide({ shadow: '-16px 0 48px rgba(var(--ink-rgb), 0.18)' });

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
  { code: 'f', hex: 'var(--surface)', name: t('dialog.itemName.colorNames.white') },
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

function shortKey(key: string) {
  return key
    .replace(/^item\.minecraft\./, '')
    .replace(/^block\.minecraft\./, '')
    .replace(/^entity\.minecraft\./, '')
    .replace(/^enchantment\.minecraft\./, '')
    .replace(/^potion\.minecraft\./, '')
    .replace(/^biome\.minecraft\./, '')
    .replace(/^\.minecraft\./, '')
    .replace(/^minecraft\./, '');
}

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

/**
 * Escape text before it goes into a v-html string.
 *
 * The text rendered by renderMinecraftText comes from `lang/*.json` inside a
 * resource pack — i.e. from whoever built the pack, not from us. Without this
 * a pack could ship `"item.x": "§a<img src=x onerror=...>"` and have it
 * parsed as markup.
 *
 * The CSP (`default-src 'self'`, no `script-src`, no remote `img-src`) blocks
 * the obvious payloads, but that is a second line of defence and it does not
 * stop CSS injection, which `style-src 'unsafe-inline'` does allow. Escaping
 * here removes the whole class rather than relying on the CSP to catch each
 * shape individually.
 */
const escapeHtml = (s: string) =>
  s.replace(/[&<>"']/g, (c) =>
    ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c] as string),
  );

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
    'c': '#FF5555', 'd': '#FF55FF', 'e': '#FFFF55', 'f': 'var(--surface)'
  };
  
  const formatMap: Record<string, { style?: string, class?: string }> = {
    'l': { style: 'font-weight: bold' },
    'o': { style: 'font-style: italic' },
    'n': { style: 'text-decoration: underline' },
    'm': { style: 'text-decoration: line-through' },
    'k': { class: 'mc-obfuscated' }
  };

  const parts = text.split('§');
  html += escapeHtml(parts[0]); // 第一个部分没有格式代码

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
    html += `<span style="${styleStr}" class="${classStr}">${escapeHtml(content)}</span>`;
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
      visible.value = false;
    }, 1500);
  } catch (error) {
    console.error('保存失败:', error);
    saveStatus.value = { text: t('dialog.itemName.saveFailed', { error }), type: 'error' };
  } finally {
    isSaving.value = false;
  }
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
/*
 * 这是个右侧抽屉，不是居中弹窗：沿用 sidebar-content + useSidebarSlide 的滑入实现，
 * 只把配色 / 字号 / 圆角 / 发丝线并到全局那一套语言上（见 docs/compose/spec/dialog-spec.md）。
 *
 * 定位与遮罩必须留在这里：ConversionPage 里的 .sidebar-overlay / .sidebar-content 是 scoped 的，
 * 加过 data-v 后只作用于它自己那棵子树，别的组件拿不到（useSidebarSlide 的注释也写了这一条）。
 */
/* .sidebar-overlay 已统一到 shared.css —— 见那里的说明。
   页面级定义会因 scoped 的 data-v 获得更高特异度，留着会盖掉全局那份。 */

.name-panel.sidebar-content {
  position: fixed;
  top: 0;
  right: 0;
  width: min(980px, 96vw);
  height: 100vh;
  z-index: 201;
  background: var(--surface);
  border-left: 1px solid rgba(var(--warm-rgb), 0.14);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  outline: none;
  /* 显式压回 1：App.vue 的 .page-shell > * > * 页面入场 stagger 会给 aside 锁 opacity */
  opacity: 1 !important;
  /* 兜底态 = 屏幕外。进出场由 inline style + transition 驱动（useSidebarSlide），
     这里只保证 element insert 那一帧不闪。 */
  transform: translateX(100%);
  /* 抽屉是真的浮起来的元素，投影保留；冷灰换成暖色中性投影 */
  box-shadow: -16px 0 48px rgba(var(--ink-rgb), 0.18);
  will-change: transform, box-shadow;
}

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  /* 顶部避开浮动窗口控制按钮（top:10 + ~44px 高） */
  padding: 56px 22px 14px;
  border-bottom: 1px solid rgba(var(--warm-rgb), 0.14);
  flex-shrink: 0;
  background: var(--surface);
}

.panel-title {
  margin: 0;
  font-size: 18px;
  /* 原来是 800 —— 层级由字号和留白建立，面板标题不该靠字重压人 */
  font-weight: 600;
  color: var(--ink);
  letter-spacing: -0.01em;
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 10px;
}

.lang-select {
  height: 32px;
  padding: 0 10px;
  border-radius: 8px;
  border: 1px solid rgba(var(--warm-rgb), 0.2);
  background: var(--surface);
  font-size: 13px;
  font-weight: 500;
  font-family: inherit;
  color: var(--ink-2);
  cursor: pointer;
  outline: none;
}
.lang-select:hover { border-color: rgba(var(--warm-rgb), 0.3); }
.lang-select:focus-visible { border-color: rgba(var(--ink-rgb), 0.45); }

.panel-body {
  flex: 1;
  min-height: 0;
  padding: 14px 22px 8px;
  display: flex;
  flex-direction: column;
  gap: 10px;
  overflow: hidden;
}

.search-bar-container {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-shrink: 0;
}

.search-input-wrapper {
  flex: 1;
  position: relative;
}

.search-icon {
  position: absolute;
  left: 12px;
  top: 50%;
  transform: translateY(-50%);
  color: var(--ink-4);
  font-size: 16px;
  pointer-events: none;
}

.search-input {
  width: 100%;
  padding: 9px 12px 9px 36px;
  border-radius: 8px;
  border: 1px solid rgba(var(--warm-rgb), 0.2);
  background: var(--surface);
  font-size: 13px;
  font-family: inherit;
  color: var(--ink);
  outline: none;
  transition: border-color 0.15s ease, background 0.15s ease;
}
/* 焦点环用 outline 而不是 box-shadow：投影按规格只留给真正浮起的元素。
   outline-offset 取负值，让环画在输入框内侧、不撑动同行元素。 */
.search-input:focus {
  border-color: rgba(var(--ink-rgb), 0.45);
  outline: 2px solid rgba(var(--ink-rgb), 0.12);
  outline-offset: -1px;
}
.search-input::placeholder { color: var(--ink-4); }

.items-count {
  font-size: 12px;
  color: var(--ink-3);
  white-space: nowrap;
}

.lang-list-container {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 2px 4px 16px 0;
}

.lang-list-container::-webkit-scrollbar { width: 8px; }
.lang-list-container::-webkit-scrollbar-track { background: transparent; }
.lang-list-container::-webkit-scrollbar-thumb {
  background: rgba(var(--warm-rgb), 0.2);
  border-radius: 6px;
}
.lang-list-container::-webkit-scrollbar-thumb:hover { background: rgba(var(--warm-rgb), 0.3); }

/* 双列平铺：条目数常在千条以上，单列会把一半横向空间空着 */
.lang-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 6px;
}

@media (max-width: 820px) {
  .name-panel { width: 100vw; }
  .lang-grid { grid-template-columns: 1fr; }
}

/*
 * 一条 = 一行：键名定宽成列，输入框和调色钮的起点因此上下对齐，
 * 整片看起来是张表而不是一堆无差别灰卡片。
 * 原来是「键名独占一行 + 输入 + 预览」三层竖排，一条 ~76px，一屏只能看十几条。
 */
.lang-item {
  display: grid;
  grid-template-columns: minmax(0, 158px) minmax(0, 1fr);
  grid-template-areas:
    'key input'
    'meta meta';
  align-items: center;
  gap: 5px 8px;
  padding: 6px 10px;
  border: 1px solid rgba(var(--warm-rgb), 0.14);
  border-radius: 6px;
  min-width: 0;
  transition: border-color 0.15s ease, background 0.15s ease;
}
/* 行不是浮起物：只换底色和发丝线，不加投影 */
.lang-item:hover {
  border-color: rgba(var(--warm-rgb), 0.3);
  background: rgba(var(--warm-rgb), 0.06);
}
.lang-item.is-modified {
  border-color: rgba(var(--warm-rgb), 0.3);
  background: rgba(var(--warm-rgb), 0.06);
}

.item-key {
  grid-area: key;
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 11px;
  font-weight: 500;
  color: var(--ink-3);
  background: rgba(var(--warm-rgb), 0.09);
  padding: 3px 6px;
  border-radius: 6px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 100%;
}
/* 改过的条目把键名提到墨色 —— 比给整行染色更省事，也不会和 hover 撞色 */
.lang-item.is-modified .item-key {
  color: var(--ink);
  background: rgba(var(--warm-rgb), 0.14);
}

.input-group {
  grid-area: input;
  display: flex;
  gap: 6px;
  min-width: 0;
}

.edit-input {
  flex: 1;
  min-width: 0;
  height: 30px;
  padding: 0 9px;
  border-radius: 8px;
  border: 1px solid rgba(var(--warm-rgb), 0.2);
  background: var(--surface);
  font-size: 13px;
  font-family: inherit;
  color: var(--ink);
  outline: none;
  transition: border-color 0.15s ease;
}
.edit-input:focus {
  border-color: rgba(var(--ink-rgb), 0.45);
  outline: 2px solid rgba(var(--ink-rgb), 0.12);
  outline-offset: -1px;
}
.edit-input::placeholder { color: var(--ink-4); }

.style-btn {
  width: 30px;
  height: 30px;
  flex-shrink: 0;
  border-radius: 6px;
  border: 1px solid rgba(var(--warm-rgb), 0.2);
  background: var(--surface);
  color: var(--ink-3);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: border-color 0.15s ease, color 0.15s ease, background 0.15s ease;
}
.style-btn:hover {
  border-color: rgba(var(--warm-rgb), 0.3);
  background: rgba(var(--warm-rgb), 0.06);
  color: var(--ink);
}

/* 预览与「原值」这一行只在有内容时渲染（见模板的 v-if），普通条目因此是单行高 */
.item-meta {
  grid-area: meta;
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
  flex-wrap: wrap;
}

/* 游戏内名称是深底彩字，预览必须跟着深底，否则白色 / 亮色名称根本看不见 */
.mc-preview {
  font-family: 'Minecraft', monospace;
  font-size: 12px;
  background: var(--ink);
  padding: 1px 6px;
  border-radius: 6px;
  color: var(--surface);
  line-height: 1.5;
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.mc-obfuscated { animation: mc-obf 0.1s steps(1) infinite; }
@keyframes mc-obf {
  0% { opacity: 0.8; }
  50% { opacity: 1; }
}

.original-val {
  font-size: 11px;
  color: var(--ink-4);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 55%;
}

/* ── 样式悬浮面板（Teleport 到 body，是真的浮层，z-index 要压过抽屉的 201） ── */
.style-panel {
  position: fixed;
  width: 260px;
  background: var(--surface);
  border: 1px solid rgba(var(--warm-rgb), 0.2);
  border-radius: 8px;
  padding: 12px;
  box-shadow: 0 12px 32px rgba(var(--ink-rgb), 0.18);
  z-index: 2000;
  display: flex;
  flex-direction: column;
  gap: 12px;
  /* fill-mode 必须是 backwards：用 both / forwards 会把末帧的 translate: 0 0 永久留下，
     元素从此成为包含块（内部一旦出现 position: fixed 就相对它定位）。
     基线规则里没有 opacity: 0 之类的隐形态，所以动画结束回落到基线是安全的。 */
  animation: style-panel-in 180ms var(--ease-out) backwards;
}

@keyframes style-panel-in {
  from { opacity: 0; translate: 0 -4px; }
  to { opacity: 1; translate: 0 0; }
}

.style-section {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.section-label {
  font-size: 10.5px;
  font-weight: 500;
  color: var(--ink-4);
  text-transform: uppercase;
  letter-spacing: 0.18em;
}

.color-grid {
  display: grid;
  grid-template-columns: repeat(8, 1fr);
  gap: 5px;
}

.color-swatch {
  aspect-ratio: 1;
  border-radius: 6px;
  border: 1px solid rgba(var(--warm-rgb), 0.2);
  cursor: pointer;
  transition: transform 120ms var(--ease-spring);
}
.color-swatch:hover { transform: scale(1.12); }

.format-grid {
  display: flex;
  flex-wrap: wrap;
  gap: 5px;
}

.format-btn {
  height: 28px;
  min-width: 28px;
  padding: 0 7px;
  border-radius: 6px;
  border: 1px solid rgba(var(--warm-rgb), 0.2);
  background: var(--surface);
  color: var(--ink-2);
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  font-family: inherit;
  transition: background 0.15s ease, border-color 0.15s ease;
}
.format-btn:hover {
  background: rgba(var(--warm-rgb), 0.06);
  border-color: rgba(var(--warm-rgb), 0.3);
}
.format-btn.reset { color: var(--danger); }
.format-btn.reset:hover {
  background: rgba(179, 38, 30, 0.08);
  border-color: rgba(179, 38, 30, 0.24);
}

.panel-footer {
  padding: 12px 22px;
  border-top: 1px solid rgba(var(--warm-rgb), 0.14);
  background: var(--surface);
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 10px;
  flex-shrink: 0;
}

.modified-hint {
  font-size: 12px;
  font-weight: 500;
  color: var(--ink);
  background: rgba(var(--warm-rgb), 0.14);
  padding: 3px 10px;
  border-radius: 6px;
  margin-right: auto;
}

.save-status {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  font-weight: 500;
  padding: 3px 10px;
  border-radius: 6px;
}
.save-status.success { color: var(--ok); background: rgba(16, 185, 129, 0.1); }
.save-status.error { color: var(--danger); background: rgba(179, 38, 30, 0.08); }

.footer-btns { display: flex; gap: 8px; align-items: center; }

/* 主 / 次按钮的皮肤来自 shared.css（.primary-btn / .ghost-btn），这里只补齐并排时的高度：
   全局主按钮 40px、幽灵按钮 36px，同一行差 4px 会显得没对齐。 */
.panel-footer .primary-btn,
.panel-footer .ghost-btn {
  height: 36px;
}

.loading-state, .empty-state {
  height: 200px;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 10px;
  color: var(--ink-4);
  font-size: 13px;
}
.loading-state i, .empty-state i { font-size: 30px; }

/* 旋转图标用全局 .spin（shared.css）；这里不再重复一份，scoped 的 0,2,0 会盖掉全局那份 */

/* 减少动效：浮层进场、色块缩放、闪烁字、以及各处底色过渡全部停掉 */
@media (prefers-reduced-motion: reduce) {
  .style-panel { animation: none; }
  .color-swatch { transition: none; }
  .color-swatch:hover { transform: none; }
  .mc-obfuscated { animation: none; }
  .lang-item,
  .lang-select,
  .search-input,
  .edit-input,
  .style-btn,
  .format-btn { transition: none; }
}
</style>
