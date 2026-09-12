<template>
  <div class="dialog-overlay">
    <div class="dialog-container dialog-content">
      <div class="dialog-header">
        <h2 class="dialog-title">{{ t('dialog.itemSize.title') }}</h2>
      </div>

      <div class="dialog-content">
        <div class="search-bar-container">
          <div class="search-input-wrapper">
            <i class="ri-search-line search-icon"></i>
            <input 
              v-model="searchText" 
              class="search-input" 
              :placeholder="t('dialog.itemSize.searchPlaceholder')"
            />
          </div>
        </div>

        <div class="items-list-container">
          <!-- 放大物品分类 -->
          <div v-if="filteredZoomInItems.length > 0" class="category-section">
            <h3 class="category-title">{{ t('dialog.itemSize.enlarge') }}</h3>
            <div class="items-grid">
              <div v-for="item in filteredZoomInItems" :key="item" class="item-row">
                <span class="item-id">{{ item }}</span>
                <div class="controls">
                  <div class="control-group">
                    <label>{{ t('dialog.itemSize.handheld') }}</label>
                    <select v-model="settings.big_item[item].handheld_scale" class="scale-select">
                      <option v-for="s in scales" :key="s" :value="s">{{ s }}</option>
                    </select>
                  </div>
                  <div class="control-group">
                    <label>{{ t('dialog.itemSize.dropped') }}</label>
                    <select v-model="settings.big_item[item].dropped_scale" class="scale-select">
                      <option v-for="s in scales" :key="s" :value="s">{{ s }}</option>
                    </select>
                  </div>
                </div>
              </div>
            </div>
          </div>

          <!-- 缩小物品分类 -->
          <div v-if="filteredZoomOutItems.length > 0" class="category-section">
            <h3 class="category-title">{{ t('dialog.itemSize.shrink') }}</h3>
            <div class="items-grid">
              <div v-for="item in filteredZoomOutItems" :key="item" class="item-row">
                <span class="item-id">{{ item }}</span>
                <div class="controls">
                  <label class="checkbox-container">
                    <input type="checkbox" v-model="settings.small_item[item].should_shrink" />
                    <span class="checkmark"></span>
                    {{ t('dialog.itemSize.enableShrink') }}
                  </label>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <div class="dialog-footer">
        <div class="footer-btns">
          <button
            class="primary-btn"
            :disabled="isSaving"
            @click="saveAndClose"
          >
            <i class="ri-save-line" v-if="!isSaving"></i>
            <i class="ri-loader-4-line spin" v-else></i>
            {{ isSaving ? t('common.saving') : t('common.save') }}
          </button>
          <button class="ghost-btn back-btn" @click="closeDialog">
            <i class="ri-arrow-go-back-line"></i>
            <span>{{ t('common.back') }}</span>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed, reactive } from 'vue';
import { useI18n } from 'vue-i18n';
import { invoke } from '@tauri-apps/api/core';
import { message } from '@tauri-apps/plugin-dialog';

const { t } = useI18n();

const props = defineProps<{
  projectName: string
}>();

const emit = defineEmits<{ close: [] }>();

const searchText = ref('');
const isSaving = ref(false);
const scales = ["1x", "2x", "3x", "4x"];

const zoomInItemsList = [
  "anvil", "book", "chipped_anvil", "cobweb", "compass", 
  "damaged_anvil", "elytra", "enchanted_golden_apple",
  "enchanting_table", "ender_pearl", "experience_bottle", "firework_rocket",
  "golden_apple", "golden_axe", "handheld_rod",
  "netherite_sword", "player_head", "slime_ball",
  "splash_potion", "totem_of_undying", "trident", "water_bucket"
];

const zoomOutItemsList = ["block", "generated", "handheld", "shield", "shield_blocking"];

const settings = reactive<{
  big_item: Record<string, { type: string, handheld_scale: string, dropped_scale: string }>,
  small_item: Record<string, { type: string, should_shrink: boolean }>
}>({
  big_item: {},
  small_item: {}
});

// Initialize reactive state
zoomInItemsList.forEach(item => {
  settings.big_item[item] = { type: 'zoom_in', handheld_scale: '1x', dropped_scale: '1x' };
});
zoomOutItemsList.forEach(item => {
  settings.small_item[item] = { type: 'zoom_out', should_shrink: false };
});

const filteredZoomInItems = computed(() => {
  if (!searchText.value) return zoomInItemsList;
  return zoomInItemsList.filter(item => item.toLowerCase().includes(searchText.value.toLowerCase()));
});

const filteredZoomOutItems = computed(() => {
  if (!searchText.value) return zoomOutItemsList;
  return zoomOutItemsList.filter(item => item.toLowerCase().includes(searchText.value.toLowerCase()));
});

const loadSettings = async () => {
  try {
    const data = await invoke<any>('get_overlay_json', { projectName: props.projectName });
    if (data.big_item) {
      Object.keys(data.big_item).forEach(key => {
        if (settings.big_item[key]) {
          settings.big_item[key].handheld_scale = data.big_item[key].handheld_scale || '1x';
          settings.big_item[key].dropped_scale = data.big_item[key].dropped_scale || '1x';
        }
      });
    }
    if (data.small_item) {
      Object.keys(data.small_item).forEach(key => {
        if (settings.small_item[key]) {
          settings.small_item[key].should_shrink = !!data.small_item[key].should_shrink;
        }
      });
    }
  } catch (error) {
    console.error('加载设置失败:', error);
  }
};

const saveAndClose = async () => {
  isSaving.value = true;
  try {
    // Collect modified settings
    const finalSettings: any = {};
    
    const bigItems: any = {};
    zoomInItemsList.forEach(item => {
      const s = settings.big_item[item];
      if (s.handheld_scale !== '1x' || s.dropped_scale !== '1x') {
        bigItems[item] = { ...s };
      }
    });
    if (Object.keys(bigItems).length > 0) finalSettings.big_item = bigItems;

    const smallItems: any = {};
    zoomOutItemsList.forEach(item => {
      const s = settings.small_item[item];
      if (s.should_shrink) {
        smallItems[item] = { ...s };
      }
    });
    if (Object.keys(smallItems).length > 0) finalSettings.small_item = smallItems;

    // Get current overlay.json to merge
    const currentData = await invoke<any>('get_overlay_json', { projectName: props.projectName });
    const mergedData = { ...currentData, ...finalSettings };
    
    // Replace big_item/small_item specifically
    mergedData.big_item = finalSettings.big_item;
    mergedData.small_item = finalSettings.small_item;

    await invoke('save_overlay_json', { 
      projectName: props.projectName, 
      data: mergedData 
    });
    
    await message(t('dialog.itemSize.saved'), { title: t('dialog.itemSize.success'), kind: 'info' });
    emit('close');
  } catch (error) {
    console.error('保存失败:', error);
    await message(t('dialog.itemSize.saveFailed', { error }), { title: t('dialog.itemSize.error'), kind: 'error' });
  } finally {
    isSaving.value = false;
  }
};

const closeDialog = () => {
  emit('close');
};

onMounted(() => {
  loadSettings();
});
</script>

<style scoped>
/* 右侧侧栏：对齐转换页版本选择 */
.dialog-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.28);
  display: flex;
  justify-content: flex-end;
  z-index: 1000;
  animation: overlay-fade 0.28s ease;
}

.dialog-container {
  width: min(520px, 94vw);
  height: 100vh;
  background: #ffffff;
  border-radius: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  box-shadow: -12px 0 36px rgba(0, 0, 0, 0.08);
  opacity: 1 !important;
  animation: sidebar-in 0.32s cubic-bezier(0.22, 1, 0.36, 1);
}

@keyframes sidebar-in {
  from { transform: translateX(100%); }
  to { transform: translateX(0); }
}

.dialog-header {
  padding: 1.25rem 1.5rem 1rem;
  background: #fff;
  border-bottom: 1px solid rgba(0, 0, 0, 0.06);
  display: flex;
  justify-content: space-between;
  align-items: center;
  flex-shrink: 0;
}

.dialog-title {
  margin: 0;
  font-size: 18px;
  font-weight: 700;
  color: #0f172a;
}

.dialog-content {
  flex: 1;
  padding: 24px;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 24px;
}

.search-bar-container {
  position: sticky;
  top: 0;
  background: #fff;
  z-index: 10;
  padding-bottom: 12px;
}

.search-input-wrapper {
  position: relative;
}

.search-icon {
  position: absolute;
  left: 12px;
  top: 50%;
  transform: translateY(-50%);
  color: #94a3b8;
}

.search-input {
  width: 100%;
  padding: 10px 12px 10px 36px;
  border-radius: 10px;
  border: 1px solid #e2e8f0;
  background: #f8fafc;
  outline: none;
}

.category-section {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.category-title {
  font-size: 14px;
  font-weight: 700;
  color: var(--theme-color);
  margin: 0;
  padding-bottom: 8px;
  border-bottom: 2px solid rgba(var(--theme-color-rgb), 0.1);
}

.items-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
  gap: 12px;
}

.item-row {
  background: #f8fafc;
  border: 1px solid #f1f5f9;
  border-radius: 12px;
  padding: 12px 16px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  transition: all 0.2s;
}

.item-row:hover {
  background: #fff;
  border-color: #e2e8f0;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.03);
}

.item-id {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-size: 13px;
  color: #475569;
  font-weight: 600;
}

.controls {
  display: flex;
  gap: 12px;
}

.control-group {
  display: flex;
  align-items: center;
  gap: 6px;
}

.control-group label {
  font-size: 11px;
  font-weight: 700;
  color: #94a3b8;
  text-transform: uppercase;
}

.scale-select {
  padding: 4px 8px;
  border-radius: 6px;
  border: 1px solid #e2e8f0;
  background: #fff;
  font-size: 12px;
  cursor: pointer;
}

.checkbox-container {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  font-weight: 600;
  color: #475569;
  cursor: pointer;
}

.dialog-footer {
  padding: 12px 1.5rem;
  background: #fff;
  border-top: 1px solid rgba(0, 0, 0, 0.06);
  display: flex;
  justify-content: flex-end;
  align-items: center;
  flex-shrink: 0;
}

.footer-btns {
  display: flex;
  gap: 8px;
  align-items: center;
}

.primary-btn,
.back-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  height: 36px;
  padding: 0 16px;
  border-radius: 10px;
  font-weight: 600;
  font-size: 13px;
  font-family: inherit;
  cursor: pointer;
  border: none;
}

.primary-btn {
  background: var(--theme-color);
  color: #fff;
  box-shadow: 0 4px 12px color-mix(in srgb, var(--theme-color) 24%, transparent);
  transition: background 0.15s ease, opacity 0.15s ease;
}
.primary-btn:disabled { opacity: 0.45; cursor: not-allowed; box-shadow: none; }
.primary-btn:hover:not(:disabled) {
  background: color-mix(in srgb, var(--theme-color) 88%, #000);
}

.back-btn {
  background: rgba(0, 0, 0, 0.04);
  color: #64748b;
  transition: background 0.15s ease, color 0.15s ease;
}
.back-btn:hover {
  background: rgba(0, 0, 0, 0.08);
  color: #1d1d1f;
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
</style>
