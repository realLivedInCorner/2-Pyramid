<template>
  <div class="dialog-overlay">
    <div class="dialog-container dialog-content">
      <div class="dialog-header">
        <h2 class="dialog-title">{{ t('dialog.itemSize.title') }}</h2>
        <button class="icon-button close-btn" @click="closeDialog" :aria-label="t('common.close')">
          <i class="ri-close-line"></i>
        </button>
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
          <button class="ghost-btn" @click="closeDialog">{{ t('common.cancel') }}</button>
          <button
            class="primary-btn"
            :disabled="isSaving"
            @click="saveAndClose"
          >
            <i class="ri-save-line" v-if="!isSaving"></i>
            <i class="ri-loader-4-line spin" v-else></i>
            {{ isSaving ? t('common.saving') : t('common.save') }}
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
.dialog-overlay {
  position: fixed;
  inset: 0;
  background: rgba(15, 23, 42, 0.4);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  backdrop-filter: blur(12px);
}

.dialog-container {
  width: min(800px, 90vw);
  height: 80vh;
  background: #ffffff;
  border-radius: 20px;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  /* 原来 0 25px 50px -12px rgba(0,0,0,0.25) — blur 50px 太大,leave 期间
     shadow 跟着 opacity 渐变时 "淡出拖尾" 比 dialog 本身还久。改紧凑点
     跟 550ms leave 同步消失。 */
  box-shadow: 0 16px 32px -8px rgba(0, 0, 0, 0.18);
}

.dialog-header {
  padding: 20px 24px;
  background: #f8fafc;
  border-bottom: 1px solid #e2e8f0;
  display: flex;
  justify-content: space-between;
  align-items: center;
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
  padding: 16px 24px;
  background: #f8fafc;
  border-top: 1px solid #e2e8f0;
  display: flex;
  justify-content: flex-end;
}

.footer-btns {
  display: flex;
  gap: 12px;
}

.ghost-btn {
  padding: 8px 16px;
  border-radius: 10px;
  border: none;
  background: transparent;
  color: #64748b;
  font-weight: 600;
  cursor: pointer;
}

.primary-btn {
  padding: 8px 20px;
  border-radius: 10px;
  border: none;
  background: var(--theme-color);
  color: #fff;
  font-weight: 600;
  display: flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
  box-shadow: 0 4px 12px rgba(var(--theme-color-rgb), 0.2);
}

.spin {
  animation: ri-spin 1s linear infinite;
}

@keyframes ri-spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}
</style>
