<template>
  <div class="dialog-overlay">
    <div class="dialog-container">
      <div class="dialog-header">
        <h2 class="dialog-title">{{ t('dialog.customName.title') }}</h2>
        <button class="icon-button" @click="closeDialog" :aria-label="t('common.close')">
          <i class="ri-close-line" aria-hidden="true"></i>
        </button>
      </div>

      <div class="dialog-toolbar">
        <div class="toolbar-left">
          <span class="toolbar-label">{{ t('dialog.customName.language') }}</span>
          <div class="segmented">
            <button :class="['seg-btn', selectedLanguage === 'zh_cn' && 'active']" @click="selectedLanguage = 'zh_cn'">中文</button>
            <button :class="['seg-btn', selectedLanguage === 'en_us' && 'active']" @click="selectedLanguage = 'en_us'">English</button>
          </div>
        </div>
        <div class="toolbar-right">
          <input v-model="searchText" class="text-input" :placeholder="t('dialog.customName.searchPlaceholder')" />
        </div>
      </div>

      <div class="dialog-content">
        <div class="code-panel">
          <div class="section-title">{{ t('dialog.customName.colorFormat') }}</div>
          <div class="code-grid">
            <button v-for="code in colorCodes" :key="code.value" class="code-chip" :style="{ color: code.preview }" @click="insertCode(code.value)">{{ code.label }}</button>
          </div>
          <div class="code-grid">
            <button v-for="code in formatCodes" :key="code.value" class="code-chip" @click="insertCode(code.value)">{{ code.label }}</button>
          </div>
          <div class="helper-text">{{ t('dialog.customName.clickHint') }}</div>
        </div>

        <div class="list-section">
          <div v-for="group in filteredGroups" :key="group.title" class="group">
            <div class="group-title">{{ getGroupTitle(group.title) }}</div>
            <div class="group-body">
              <div v-for="item in group.items" :key="item.id" class="item-row">
                <div class="item-info">
                  <div class="item-name">{{ getDisplayName(item) }}</div>
                  <div class="item-id">{{ item.id }}</div>
                </div>
                <input
                  v-model="customNames[item.id]"
                  class="text-input"
                  :placeholder="t('dialog.customName.inputPlaceholder')"
                  @focus="activeItemId = item.id"
                />
              </div>
            </div>
          </div>
        </div>

        <div class="tips">
          {{ t('dialog.customName.colorTip') }}
        </div>
      </div>

      <div class="dialog-footer">
        <button class="ghost-btn" @click="closeDialog">{{ t('common.cancel') }}</button>
        <button class="primary-btn" @click="saveAndClose" :disabled="isSaving">
          {{ isSaving ? t('common.saving') : t('common.save') }}
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { invoke } from '@tauri-apps/api/core';
import { message } from '@tauri-apps/plugin-dialog';

const { t } = useI18n();

const emit = defineEmits<{ close: [] }>();

type ItemDef = { id: string; zh: string; en: string };

type GroupDef = { title: string; items: ItemDef[] };

const groups: GroupDef[] = [
  {
    title: '工具与武器',
    items: [
      { id: 'minecraft:wooden_sword', zh: '木剑', en: 'Wooden Sword' },
      { id: 'minecraft:stone_sword', zh: '石剑', en: 'Stone Sword' },
      { id: 'minecraft:iron_sword', zh: '铁剑', en: 'Iron Sword' },
      { id: 'minecraft:golden_sword', zh: '金剑', en: 'Golden Sword' },
      { id: 'minecraft:diamond_sword', zh: '钻石剑', en: 'Diamond Sword' },
      { id: 'minecraft:netherite_sword', zh: '下界合金剑', en: 'Netherite Sword' },
      { id: 'minecraft:bow', zh: '弓', en: 'Bow' },
      { id: 'minecraft:trident', zh: '三叉戟', en: 'Trident' }
    ]
  },
  {
    title: '矿石与材料',
    items: [
      { id: 'minecraft:coal_ore', zh: '煤矿', en: 'Coal Ore' },
      { id: 'minecraft:iron_ore', zh: '铁矿', en: 'Iron Ore' },
      { id: 'minecraft:gold_ore', zh: '金矿', en: 'Gold Ore' },
      { id: 'minecraft:diamond_ore', zh: '钻石矿', en: 'Diamond Ore' },
      { id: 'minecraft:emerald_ore', zh: '绿宝石矿', en: 'Emerald Ore' }
    ]
  },
  {
    title: '食物',
    items: [
      { id: 'minecraft:apple', zh: '苹果', en: 'Apple' },
      { id: 'minecraft:bread', zh: '面包', en: 'Bread' },
      { id: 'minecraft:beef', zh: '生牛肉', en: 'Beef' },
      { id: 'minecraft:cooked_beef', zh: '熟牛肉', en: 'Steak' },
      { id: 'minecraft:golden_apple', zh: '金苹果', en: 'Golden Apple' }
    ]
  },
  {
    title: '装备',
    items: [
      { id: 'minecraft:leather_helmet', zh: '皮革头盔', en: 'Leather Helmet' },
      { id: 'minecraft:iron_helmet', zh: '铁头盔', en: 'Iron Helmet' },
      { id: 'minecraft:diamond_helmet', zh: '钻石头盔', en: 'Diamond Helmet' },
      { id: 'minecraft:netherite_helmet', zh: '下界合金头盔', en: 'Netherite Helmet' },
      { id: 'minecraft:elytra', zh: '鞘翅', en: 'Elytra' }
    ]
  }
];

const colorCodes = [
  { label: '§0 黑', value: '§0', preview: '#000000' },
  { label: '§1 深蓝', value: '§1', preview: '#0000AA' },
  { label: '§2 深绿', value: '§2', preview: '#00AA00' },
  { label: '§3 湖蓝', value: '§3', preview: '#00AAAA' },
  { label: '§4 深红', value: '§4', preview: '#AA0000' },
  { label: '§5 紫', value: '§5', preview: '#AA00AA' },
  { label: '§6 金', value: '§6', preview: '#FFAA00' },
  { label: '§7 灰', value: '§7', preview: '#AAAAAA' },
  { label: '§8 深灰', value: '§8', preview: '#555555' },
  { label: '§9 蓝', value: '§9', preview: '#5555FF' },
  { label: '§a 绿', value: '§a', preview: '#55FF55' },
  { label: '§b 青', value: '§b', preview: '#55FFFF' },
  { label: '§c 红', value: '§c', preview: '#FF5555' },
  { label: '§d 粉', value: '§d', preview: '#FF55FF' },
  { label: '§e 黄', value: '§e', preview: '#FFFF55' },
  { label: '§f 白', value: '§f', preview: '#FFFFFF' }
];

const formatCodes = [
  { label: '§k 随机', value: '§k' },
  { label: '§l 粗体', value: '§l' },
  { label: '§m 删除线', value: '§m' },
  { label: '§n 下划线', value: '§n' },
  { label: '§o 斜体', value: '§o' },
  { label: '§r 重置', value: '§r' }
];

const customNames = ref<Record<string, string>>({});

const getGroupTitle = (title: string): string => {
  const map: Record<string, string> = {
    '工具与武器': t('dialog.customName.groups.tools'),
    '矿石与材料': t('dialog.customName.groups.ores'),
    '食物': t('dialog.customName.groups.food'),
    '装备': t('dialog.customName.groups.armor')
  };
  return map[title] || title;
};
const selectedLanguage = ref<'zh_cn' | 'en_us'>('zh_cn');
const activeItemId = ref('');
const searchText = ref('');
const isSaving = ref(false);

const filteredGroups = computed(() => {
  if (!searchText.value) return groups;
  const q = searchText.value.toLowerCase();
  return groups
    .map((group) => ({
      ...group,
      items: group.items.filter((item) => {
        const name = selectedLanguage.value === 'zh_cn' ? item.zh : item.en;
        return item.id.includes(q) || name.toLowerCase().includes(q);
      })
    }))
    .filter((group) => group.items.length > 0);
});

const getDisplayName = (item: ItemDef) => {
  return selectedLanguage.value === 'zh_cn' ? item.zh : item.en;
};

const insertCode = (code: string) => {
  if (!activeItemId.value) return;
  const current = customNames.value[activeItemId.value] || '';
  customNames.value[activeItemId.value] = current + code;
};

const loadSettings = async () => {
  try {
    const settings = await invoke('get_overlay_settings');
    if (settings && typeof settings === 'object') {
      const s = settings as any;
      if (s.selected_language === 'en_us' || s.selected_language === 'zh_cn') {
        selectedLanguage.value = s.selected_language;
      }
      if (s.lang_itemname) {
        customNames.value = { ...s.lang_itemname };
      }
    }
  } catch (error) {
    console.error('加载自定义名称设置失败', error);
  }
};

const saveAndClose = async () => {
  isSaving.value = true;
  try {
    const cleaned: Record<string, string> = {};
    Object.entries(customNames.value).forEach(([key, value]) => {
      if (value && value.trim()) cleaned[key] = value.trim();
    });

    await invoke('save_overlay_settings', {
      settings: {
        selected_language: selectedLanguage.value,
        lang_itemname: cleaned
      }
    });
    await message(t('dialog.customName.saved'), { title: t('dialog.customName.saveSuccess'), kind: 'info' });
    emit('close');
  } catch (error) {
    console.error('保存自定义名称设置失败', error);
    await message(t('dialog.customName.saveFailed', { error }), { title: t('dialog.customName.saveFailedTitle'), kind: 'error' });
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
  background: rgba(15, 23, 42, 0.35);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  backdrop-filter: blur(8px);
  animation: overlay-fade 0.2s ease;
}

.dialog-container {
  width: min(840px, 94vw);
  max-height: 88vh;
  background: #fff;
  border-radius: 12px;
  border: 1px solid rgba(0, 0, 0, 0.08);
  box-shadow: 0 24px 60px rgba(0, 0, 0, 0.2);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  animation: panel-rise 0.24s ease;
}

.dialog-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 18px 20px;
  border-bottom: 1px solid rgba(0, 0, 0, 0.06);
}

.dialog-title {
  margin: 0;
  font-size: 18px;
  font-weight: 700;
}

.dialog-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 20px 0;
  gap: 12px;
}

.toolbar-left {
  display: flex;
  align-items: center;
  gap: 10px;
}

.toolbar-label {
  color: #475569;
  font-size: 13px;
}

.segmented {
  display: inline-flex;
  gap: 6px;
  background: rgba(0, 0, 0, 0.04);
  padding: 4px;
  border-radius: 10px;
}

.seg-btn {
  border: none;
  padding: 6px 14px;
  border-radius: 8px;
  background: transparent;
  font-weight: 600;
  cursor: pointer;
}

.seg-btn.active {
  background: #fff;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.12);
}

.text-input {
  padding: 10px 12px;
  border-radius: 10px;
  border: 1px solid rgba(0, 0, 0, 0.12);
}

.dialog-content {
  padding: 20px;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.code-panel {
  display: grid;
  gap: 10px;
  padding: 12px;
  border-radius: 12px;
  background: rgba(0, 0, 0, 0.03);
}

.section-title {
  font-weight: 600;
  color: #1f2937;
}

.code-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(90px, 1fr));
  gap: 8px;
}

.code-chip {
  border: 1px solid rgba(0, 0, 0, 0.08);
  border-radius: 8px;
  padding: 6px 8px;
  background: #fff;
  font-size: 12px;
  cursor: pointer;
  text-align: left;
}

.helper-text {
  font-size: 12px;
  color: #94a3b8;
}

.group {
  display: grid;
  gap: 10px;
}

.group-title {
  font-weight: 600;
  color: #1f2937;
}

.group-body {
  display: grid;
  gap: 8px;
}

.item-row {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px;
  padding: 10px 12px;
  border-radius: 10px;
  background: rgba(0, 0, 0, 0.03);
}

.item-info {
  display: grid;
  gap: 4px;
}

.item-name {
  font-weight: 600;
}

.item-id {
  font-size: 12px;
  color: #64748b;
}

.tips {
  font-size: 12px;
  color: #b45309;
  background: #fef3c7;
  border-radius: 10px;
  padding: 10px 12px;
}

.dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  padding: 16px 20px;
  border-top: 1px solid rgba(0, 0, 0, 0.06);
}

.primary-btn {
  background: var(--theme-color);
  color: white;
  border: none;
  padding: 10px 16px;
  border-radius: 10px;
  font-weight: 600;
  cursor: pointer;
}

.ghost-btn {
  background: #f1f5f9;
  border: none;
  padding: 10px 16px;
  border-radius: 10px;
  font-weight: 600;
  cursor: pointer;
}

.icon-button {
  border: none;
  background: transparent;
  width: 32px;
  height: 32px;
  border-radius: 8px;
  display: grid;
  place-items: center;
  cursor: pointer;
  color: #6b7280;
}

.icon-button:hover {
  background: rgba(0, 0, 0, 0.06);
}

@keyframes overlay-fade {
  from { opacity: 0; }
  to { opacity: 1; }
}

@keyframes panel-rise {
  from {
    opacity: 0;
    transform: translateY(8px) scale(0.98);
  }
  to {
    opacity: 1;
    transform: translateY(0) scale(1);
  }
}
</style>
