<template>
  <div class="dialog-overlay">
    <div class="dialog-container dialog-content">
      <div class="dialog-header">
        <h2 class="dialog-title">{{ t('dialog.visual.title') }}</h2>
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

          <!-- 双色循环（N 卡）：HSV 取色 + 实时预览 -->
          <div v-if="settings.outline_type === 'gradient'" class="outline-detail">
            <div class="gradient-preview" :style="gradientPreviewStyle" aria-hidden="true">
              <span class="preview-label">{{ t('dialog.visual.gradientPreview') }}</span>
            </div>

            <div class="detail-block">
              <label class="detail-label">{{ t('dialog.visual.gradientColorA') }}</label>
              <HsvColorPicker v-model="settings.core_gradient_outline.color_a" />
            </div>
            <div class="detail-block">
              <label class="detail-label">{{ t('dialog.visual.gradientColorB') }}</label>
              <HsvColorPicker v-model="settings.core_gradient_outline.color_b" />
            </div>
            <p class="detail-hint">{{ t('dialog.visual.gradientHint') }}</p>
          </div>
        </div>
      </div>

      <div class="dialog-footer">
        <div class="save-status" v-if="saveStatus" :class="saveStatus.type">
          <i :class="saveStatus.type === 'success' ? 'ri-checkbox-circle-line' : 'ri-error-warning-line'"></i>
          {{ saveStatus.text }}
        </div>
        <div class="footer-btns">
          <button class="primary-btn" :disabled="isSaving" @click="handleSave">
            <i class="ri-save-line" v-if="!isSaving"></i>
            <i class="ri-loader-4-line spin" v-else></i>
            {{ isSaving ? t('common.saving') : t('common.save') }}
          </button>
          <button class="ghost-btn back-btn" @click="emit('close')">
            <i class="ri-arrow-go-back-line"></i>
            <span>{{ t('common.back') }}</span>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, reactive, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { invoke } from '@tauri-apps/api/core';
import HsvColorPicker from './HsvColorPicker.vue';

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
  outline_type: 'none' as 'none' | 'default' | 'rainbow' | 'rainbow_hexian' | 'gradient',
  core_outline: {
    color: { r: 1, g: 1, b: 1, a: 1 },
    thickness: 2
  },
  // 双色循环（N 卡）：默认浅紫 ↔ 黑
  core_gradient_outline: {
    color_a: { r: 0.7, g: 0.4, b: 0.9, a: 1 },
    color_b: { r: 0, g: 0, b: 0, a: 1 }
  }
});

const outlineTypes = [
  { id: 'none' as const, name: t('dialog.visual.outlineTypes.none'), color: '#f1f5f9' },
  // core_rainbow_outline：N 卡彩虹
  { id: 'rainbow' as const, name: t('dialog.visual.outlineTypes.rainbow'), color: 'linear-gradient(45deg, #ff0000, #00ff00, #0000ff)' },
  // core_rainbow_outline_hexian：其他显卡彩虹
  { id: 'rainbow_hexian' as const, name: t('dialog.visual.outlineTypes.rainbowHexian'), color: 'linear-gradient(135deg, #ff8a00, #e52e71, #9b59b6)' },
  // core_gradient_outline：N 卡双色循环
  { id: 'gradient' as const, name: t('dialog.visual.outlineTypes.gradient'), color: 'linear-gradient(90deg, #b366e6, #000000)' },
  // core_outline：可改颜色 / 粗细
  { id: 'default' as const, name: t('dialog.visual.outlineTypes.standard'), color: '#fff' }
];

function hexFromRgba(c: { r: number; g: number; b: number }) {
  const to = (v: number) => Math.round(Math.min(1, Math.max(0, v)) * 255).toString(16).padStart(2, '0');
  return `#${to(c.r)}${to(c.g)}${to(c.b)}`;
}

function rgbaCss(c: { r: number; g: number; b: number; a: number }) {
  const r = Math.round(Math.min(1, Math.max(0, c.r)) * 255);
  const g = Math.round(Math.min(1, Math.max(0, c.g)) * 255);
  const b = Math.round(Math.min(1, Math.max(0, c.b)) * 255);
  return `rgba(${r}, ${g}, ${b}, ${c.a})`;
}

/** 贴近着色器：sin(GameTime·5000 + pos) 在 A/B 间往返 */
const gradientPreviewStyle = computed(() => {
  const a = rgbaCss(settings.core_gradient_outline.color_a);
  const b = rgbaCss(settings.core_gradient_outline.color_b);
  return {
    '--grad-a': a,
    '--grad-b': b,
    background: `linear-gradient(90deg, ${a}, ${b}, ${a})`,
    backgroundSize: '200% 100%',
    animation: 'grad-cycle 2.4s linear infinite'
  };
});

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
    const ot = data.outline_type || 'none';
    settings.outline_type = (
      ot === 'none' || ot === 'default' || ot === 'rainbow' || ot === 'rainbow_hexian' || ot === 'gradient'
    ) ? ot : 'none';
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
    const go = data.core_gradient_outline;
    if (go && typeof go === 'object') {
      if (go.color_a && typeof go.color_a === 'object') {
        settings.core_gradient_outline.color_a.r = Number(go.color_a.r ?? 0.7);
        settings.core_gradient_outline.color_a.g = Number(go.color_a.g ?? 0.4);
        settings.core_gradient_outline.color_a.b = Number(go.color_a.b ?? 0.9);
        settings.core_gradient_outline.color_a.a = Number(go.color_a.a ?? 1);
      }
      if (go.color_b && typeof go.color_b === 'object') {
        settings.core_gradient_outline.color_b.r = Number(go.color_b.r ?? 0);
        settings.core_gradient_outline.color_b.g = Number(go.color_b.g ?? 0);
        settings.core_gradient_outline.color_b.b = Number(go.color_b.b ?? 0);
        settings.core_gradient_outline.color_b.a = Number(go.color_b.a ?? 1);
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
      core_outline_rainbow: { enabled: ot === 'rainbow' || ot === 'rainbow_hexian' },
      core_gradient_outline: {
        enabled: ot === 'gradient',
        color_a: { ...settings.core_gradient_outline.color_a },
        color_b: { ...settings.core_gradient_outline.color_b }
      }
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
  width: min(420px, 94vw);
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
  border-bottom: 1px solid rgba(0, 0, 0, 0.06);
  display: flex;
  justify-content: space-between;
  align-items: center;
  flex-shrink: 0;
}

.dialog-title { margin: 0; font-size: 18px; font-weight: 700; color: #0f172a; }

.dialog-content {
  flex: 1;
  padding: 1.25rem 1.5rem;
  display: flex;
  flex-direction: column;
  gap: 20px;
  overflow-y: auto;
  min-height: 0;
}
.dialog-content::-webkit-scrollbar { width: 6px; }
.dialog-content::-webkit-scrollbar-thumb {
  background: rgba(0, 0, 0, 0.12);
  border-radius: 3px;
}

.option-item {
  display: flex; justify-content: space-between; align-items: center;
  padding: 16px; background: #f8fafc; border-radius: 12px;
}

.option-label { font-weight: 700; color: #1e293b; margin-bottom: 4px; }
.option-desc { font-size: 13px; color: #64748b; }

.option-group { display: flex; flex-direction: column; gap: 12px; }
.group-label { font-weight: 700; color: #1e293b; font-size: 15px; }
.group-desc { font-size: 12px; color: #94a3b8; margin-top: 4px; }

.outline-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 10px; }

.detail-hint {
  margin: 0;
  font-size: 12px;
  color: #94a3b8;
  line-height: 1.45;
}

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
  gap: 12px;
}

.detail-block {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.gradient-preview {
  height: 44px;
  border-radius: 10px;
  border: 1px solid rgba(0, 0, 0, 0.08);
  position: relative;
  overflow: hidden;
  display: flex;
  align-items: center;
  justify-content: center;
}

.preview-label {
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: rgba(255, 255, 255, 0.92);
  text-shadow: 0 1px 2px rgba(0, 0, 0, 0.45);
  pointer-events: none;
}

@keyframes grad-cycle {
  from { background-position: 0% 50%; }
  to { background-position: 200% 50%; }
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
  padding: 12px 1.5rem;
  border-top: 1px solid rgba(0, 0, 0, 0.06);
  background: #fff;
  display: flex;
  justify-content: flex-end;
  align-items: center;
  gap: 10px;
  flex-shrink: 0;
}

.save-status { display: flex; align-items: center; gap: 6px; font-size: 13px; font-weight: 700; }
.save-status.success { color: #10b981; }
.save-status.error { color: #ef4444; }

.footer-btns { display: flex; gap: 8px; align-items: center; }

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

.icon-button { background: none; border: none; cursor: pointer; color: #94a3b8; font-size: 20px; }
.icon-button:hover { color: #0f172a; }

.spin { animation: ri-spin 1s linear infinite; }
@keyframes ri-spin { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }
@keyframes overlay-fade { from { opacity: 0; } to { opacity: 1; } }
</style>
