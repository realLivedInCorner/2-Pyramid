<template>
  <div class="dialog-overlay">
    <div class="dialog-container dialog-content">
      <div class="dialog-header">
        <h2 class="dialog-title">{{ t('dialog.visual.title') }}</h2>
        <button class="icon-button close-btn" @click="emit('close')" :aria-label="t('common.close')">
          <i class="ri-close-line"></i>
        </button>
      </div>

      <div class="dialog-content">
        <!-- 背包无阴影 -->
        <div class="option-item">
          <div class="option-info">
            <div class="option-label">{{ t('dialog.visual.noShadow') }}</div>
            <div class="option-desc">{{ t('dialog.visual.noShadowDesc') }}</div>
          </div>
          <label class="switch">
            <input type="checkbox" v-model="settings.no_shadow">
            <span class="slider round"></span>
          </label>
        </div>

        <!-- 附魔闪光 -->
        <div class="option-item">
          <div class="option-info">
            <div class="option-label">{{ t('dialog.visual.enchantGlint') }}</div>
            <div class="option-desc">{{ t('dialog.visual.enchantGlintDesc') }}</div>
          </div>
          <label class="switch">
            <input type="checkbox" v-model="settings.custom_glint">
            <span class="slider round"></span>
          </label>
        </div>

        <!-- 实体描边 -->
        <div class="option-group">
          <div class="group-header">
            <div class="group-label">{{ t('dialog.visual.outline') }}</div>
            <div class="group-desc">{{ t('dialog.visual.outlineDesc') }}</div>
          </div>
          <div class="outline-grid">
            <button
              v-for="type in outlineTypes"
              :key="type.id"
              class="outline-card"
              :class="{ active: settings.outline_type === type.id }"
              @click="settings.outline_type = type.id"
            >
              <div class="outline-preview" :style="{ background: type.color }"></div>
              <div class="outline-name">{{ type.name }}</div>
            </button>
          </div>

          <!-- 标准描边：对齐 overlay.py 的 core_outline.color / thickness -->
          <div v-if="settings.outline_type === 'default'" class="outline-detail">
            <div class="detail-row">
              <label class="detail-label">{{ t('dialog.visual.outlineColor') }}</label>
              <div class="color-row">
                <input
                  type="color"
                  class="color-input"
                  :value="hexFromRgba(settings.core_outline.color)"
                  @input="onColorInput($event)"
                />
                <span class="color-hex">{{ hexFromRgba(settings.core_outline.color) }}</span>
              </div>
            </div>
            <div class="detail-row">
              <label class="detail-label">{{ t('dialog.visual.outlineAlpha') }}</label>
              <input
                type="range"
                min="0"
                max="1"
                step="0.05"
                v-model.number="settings.core_outline.color.a"
                class="range-input"
              />
              <span class="range-val">{{ settings.core_outline.color.a.toFixed(2) }}</span>
            </div>
            <div class="detail-row">
              <label class="detail-label">{{ t('dialog.visual.outlineThickness') }}</label>
              <input
                type="range"
                min="0.5"
                max="6"
                step="0.5"
                v-model.number="settings.core_outline.thickness"
                class="range-input"
              />
              <span class="range-val">{{ settings.core_outline.thickness }}x</span>
            </div>
          </div>
        </div>
      </div>

      <div class="dialog-footer">
        <div class="save-status" v-if="saveStatus" :class="saveStatus.type">
          <i :class="saveStatus.type === 'success' ? 'ri-checkbox-circle-line' : 'ri-error-warning-line'"></i>
          {{ saveStatus.text }}
        </div>
        <div class="footer-btns">
          <button class="ghost-btn" @click="emit('close')">{{ t('common.cancel') }}</button>
          <button class="primary-btn" :disabled="isSaving" @click="handleSave">
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
import { ref, onMounted, reactive } from 'vue';
import { useI18n } from 'vue-i18n';
import { invoke } from '@tauri-apps/api/core';

const { t } = useI18n();

const props = defineProps<{
  projectName: string
}>();

const emit = defineEmits<{ close: [] }>();

const isSaving = ref(false);
const saveStatus = ref<{ text: string, type: 'success' | 'error' } | null>(null);

const settings = reactive({
  no_shadow: false,
  custom_glint: false,
  outline_type: 'none' as 'none' | 'default' | 'rainbow',
  core_outline: {
    color: { r: 1, g: 1, b: 1, a: 1 },
    thickness: 2
  }
});

const outlineTypes = [
  { id: 'none' as const, name: t('dialog.visual.outlineTypes.none'), color: '#f1f5f9' },
  // 传统：core_outline（N 卡 / 通用）
  { id: 'default' as const, name: t('dialog.visual.outlineTypes.standard'), color: '#fff' },
  // 核显：core_rainbow_outline
  { id: 'rainbow' as const, name: t('dialog.visual.outlineTypes.rainbow'), color: 'linear-gradient(45deg, #ff0000, #00ff00, #0000ff)' }
];

function hexFromRgba(c: { r: number; g: number; b: number }) {
  const to = (v: number) => Math.round(Math.min(1, Math.max(0, v)) * 255).toString(16).padStart(2, '0');
  return `#${to(c.r)}${to(c.g)}${to(c.b)}`;
}

function onColorInput(e: Event) {
  const hex = (e.target as HTMLInputElement).value.replace('#', '');
  if (hex.length !== 6) return;
  settings.core_outline.color.r = parseInt(hex.slice(0, 2), 16) / 255;
  settings.core_outline.color.g = parseInt(hex.slice(2, 4), 16) / 255;
  settings.core_outline.color.b = parseInt(hex.slice(4, 6), 16) / 255;
}

const loadSettings = async () => {
  try {
    const data = await invoke<any>('get_overlay_json', { projectName: props.projectName });
    settings.no_shadow = !!data.no_shadow;
    settings.custom_glint = !!data.custom_glint;
    // 旧配置里的 hexian 归入彩虹（核显）档
    let ot = data.outline_type || 'none';
    if (ot === 'rainbow_hexian') ot = 'rainbow';
    settings.outline_type = ot;
    // 兼容 Python core_outline.color / thickness
    const co = data.core_outline;
    if (co && typeof co === 'object') {
      if (co.color && typeof co.color === 'object') {
        settings.core_outline.color.r = Number(co.color.r ?? 1);
        settings.core_outline.color.g = Number(co.color.g ?? 1);
        settings.core_outline.color.b = Number(co.color.b ?? 1);
        settings.core_outline.color.a = Number(co.color.a ?? 1);
      }
      if (typeof co.thickness === 'number') {
        settings.core_outline.thickness = co.thickness;
      }
    }
  } catch (e) {
    console.error('加载设置失败:', e);
  }
};

const handleSave = async () => {
  isSaving.value = true;
  saveStatus.value = null;
  try {
    const currentData = await invoke<any>('get_overlay_json', { projectName: props.projectName });
    const ot = settings.outline_type;
    // 同时写入 Vue 简化键 + Python 兼容键，保证打包逻辑与旧 overlay.json 都能读
    const mergedData = {
      ...currentData,
      no_shadow: settings.no_shadow,
      custom_glint: settings.custom_glint,
      outline_type: ot,
      core_shadow: { enabled: settings.no_shadow },
      core_outline: {
        enabled: ot === 'default',
        color: { ...settings.core_outline.color },
        thickness: settings.core_outline.thickness
      },
      core_outline_rainbow: { enabled: ot === 'rainbow' }
    };
    await invoke('save_overlay_json', { projectName: props.projectName, data: mergedData });
    saveStatus.value = { text: t('dialog.visual.saved'), type: 'success' };
    setTimeout(() => emit('close'), 1500);
  } catch (e) {
    saveStatus.value = { text: t('dialog.visual.saveFailed', { error: e }), type: 'error' };
  } finally {
    isSaving.value = false;
  }
};

onMounted(loadSettings);
</script>

<style scoped>
.dialog-overlay {
  position: fixed; inset: 0; background: rgba(15, 23, 42, 0.4);
  display: flex; align-items: center; justify-content: center; z-index: 1000; backdrop-filter: blur(12px);
}

.dialog-container {
  width: 500px; background: #ffffff; border-radius: 20px;
  display: flex; flex-direction: column;
  /* 原来 0 25px 50px -12px rgba(0,0,0,0.25) — blur 50px 太大,leave 期间
     shadow 跟着 opacity 渐变时 "淡出拖尾" 比 dialog 本身还久。改紧凑点
     跟 550ms leave 同步消失。 */
  box-shadow: 0 16px 32px -8px rgba(0, 0, 0, 0.18);
}

.dialog-header {
  padding: 20px 24px; border-bottom: 1px solid #e2e8f0;
  display: flex; justify-content: space-between; align-items: center;
}

.dialog-title { margin: 0; font-size: 18px; font-weight: 700; color: #0f172a; }

.dialog-content { padding: 24px; display: flex; flex-direction: column; gap: 24px; }

.option-item {
  display: flex; justify-content: space-between; align-items: center;
  padding: 16px; background: #f8fafc; border-radius: 12px;
}

.option-label { font-weight: 700; color: #1e293b; margin-bottom: 4px; }
.option-desc { font-size: 13px; color: #64748b; }

.option-group { display: flex; flex-direction: column; gap: 12px; }
.group-label { font-weight: 700; color: #1e293b; font-size: 15px; }
.group-desc { font-size: 12px; color: #94a3b8; margin-top: 4px; }

.outline-grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 10px; }

.outline-card {
  padding: 12px; border-radius: 12px; border: 2px solid #f1f5f9;
  background: #fff; cursor: pointer; transition: all 0.2s;
  display: flex; align-items: center; gap: 12px;
}

.outline-card.active { border-color: var(--theme-color); background: rgba(var(--theme-color-rgb), 0.05); }

.outline-preview { width: 32px; height: 32px; border-radius: 8px; border: 1px solid rgba(0,0,0,0.05); }

.outline-name { font-size: 14px; font-weight: 600; color: #475569; }

.outline-detail {
  margin-top: 4px;
  padding: 12px 14px;
  border-radius: 12px;
  background: rgba(0, 0, 0, 0.03);
  border: 1px solid #eef2f7;
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.detail-row {
  display: grid;
  grid-template-columns: 88px 1fr auto;
  gap: 10px;
  align-items: center;
  font-size: 13px;
}
.detail-label { color: #64748b; font-weight: 600; }
.color-row { display: flex; align-items: center; gap: 8px; }
.color-input {
  width: 36px; height: 28px; padding: 0; border: 1px solid #e2e8f0;
  border-radius: 6px; cursor: pointer; background: none;
}
.color-hex {
  font-family: ui-monospace, monospace; font-size: 12px; color: #475569;
}
.range-input { width: 100%; accent-color: var(--theme-color); }
.range-val {
  font-variant-numeric: tabular-nums;
  color: #374151; font-weight: 600; min-width: 36px; text-align: right;
}

.switch { position: relative; display: inline-block; width: 44px; height: 22px; }
.switch input { opacity: 0; width: 0; height: 0; }
.slider {
  position: absolute; cursor: pointer; inset: 0; background-color: #cbd5e1;
  transition: .4s; border-radius: 34px;
}
.slider:before {
  position: absolute; content: ""; height: 16px; width: 16px; left: 3px; bottom: 3px;
  background-color: white; transition: .4s; border-radius: 50%;
}
input:checked + .slider { background-color: var(--theme-color); }
input:checked + .slider:before { transform: translateX(22px); }

.dialog-footer {
  padding: 20px 24px; border-top: 1px solid #e2e8f0;
  display: flex; justify-content: space-between; align-items: center;
}

.save-status { display: flex; align-items: center; gap: 6px; font-size: 13px; font-weight: 700; }
.save-status.success { color: #10b981; }
.save-status.error { color: #ef4444; }

.footer-btns { display: flex; gap: 12px; margin-left: auto; }

.primary-btn {
  padding: 10px 24px; background: var(--theme-color); color: #fff;
  border-radius: 12px; font-weight: 700; display: flex; align-items: center; gap: 8px;
}

.ghost-btn { padding: 10px 20px; color: #64748b; font-weight: 700; }

.icon-button { background: none; border: none; cursor: pointer; color: #94a3b8; font-size: 20px; }
.icon-button:hover { color: #0f172a; }

.spin { animation: ri-spin 1s linear infinite; }
@keyframes ri-spin { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }
</style>
