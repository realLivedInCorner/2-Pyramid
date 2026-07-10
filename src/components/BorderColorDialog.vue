<template>
  <div class="dialog-overlay">
    <div class="dialog-container">
      <div class="dialog-header">
        <h2 class="dialog-title">{{ t('dialog.borderColor.title') }}</h2>
        <button class="icon-button" @click="closeDialog" :aria-label="t('common.close')">
          <i class="ri-close-line" aria-hidden="true"></i>
        </button>
      </div>

      <div class="dialog-content">
        <div class="section">
          <div class="section-title">{{ t('dialog.borderColor.borderMode') }}</div>
          <div class="mode-row">
            <label class="radio">
              <input type="radio" value="none" v-model="mode" />
              <span>{{ t('dialog.borderColor.off') }}</span>
            </label>
            <label class="radio">
              <input type="radio" value="custom" v-model="mode" />
              <span>{{ t('dialog.borderColor.customColor') }}</span>
            </label>
            <label class="radio">
              <input type="radio" value="rainbow" v-model="mode" />
              <span>{{ t('dialog.borderColor.rainbow') }}</span>
            </label>
          </div>
        </div>

        <div class="section" v-if="mode === 'custom'">
          <div class="section-title">{{ t('dialog.borderColor.colorThickness') }}</div>
          <div class="color-row">
            <input type="color" v-model="selectedColor" class="color-input" />
            <span class="color-value">{{ selectedColor }}</span>
          </div>
          <label class="slider-row">
            {{ t('dialog.borderColor.thickness', { thickness }) }}
            <input type="range" min="1" max="5" step="1" v-model.number="thickness" />
          </label>
        </div>

        <div class="tips">
          {{ t('dialog.borderColor.tips') }}
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
import { ref, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { invoke } from '@tauri-apps/api/core';
import { message } from '@tauri-apps/plugin-dialog';

const { t } = useI18n();

const emit = defineEmits<{ close: [] }>();

const mode = ref<'none' | 'custom' | 'rainbow'>('none');
const selectedColor = ref('#00b2ff');
const thickness = ref(2);
const isSaving = ref(false);

const loadSettings = async () => {
  try {
    const settings = await invoke('get_overlay_settings');
    if (settings && typeof settings === 'object') {
      const s = settings as any;
      if (s.core_outline_rainbow?.enabled) {
        mode.value = 'rainbow';
      } else if (s.core_outline?.enabled) {
        mode.value = 'custom';
        const color = s.core_outline.color || { r: 1, g: 1, b: 1, a: 1 };
        selectedColor.value = rgbaToHex(color.r, color.g, color.b);
        thickness.value = s.core_outline.thickness || 2;
      } else {
        mode.value = 'none';
      }
    }
  } catch (error) {
    console.error('加载边框设置失败:', error);
  }
};

const saveAndClose = async () => {
  isSaving.value = true;
  try {
    const payload: any = {};

    if (mode.value === 'rainbow') {
      payload.core_outline_rainbow = { enabled: true };
      payload.core_outline = { enabled: false };
    } else if (mode.value === 'custom') {
      const rgba = hexToRgba(selectedColor.value);
      payload.core_outline = {
        enabled: true,
        color: rgba,
        thickness: thickness.value
      };
      payload.core_outline_rainbow = { enabled: false };
    } else {
      payload.core_outline = { enabled: false };
      payload.core_outline_rainbow = { enabled: false };
    }

    await invoke('save_overlay_settings', { settings: payload });
    await message(t('dialog.borderColor.saved'), { title: t('dialog.borderColor.saveSuccess'), kind: 'info' });
    emit('close');
  } catch (error) {
    console.error('保存边框设置失败:', error);
    await message(t('dialog.borderColor.saveFailed', { error }), { title: t('dialog.borderColor.saveFailedTitle'), kind: 'error' });
  } finally {
    isSaving.value = false;
  }
};

const hexToRgba = (hex: string) => {
  const normalized = hex.replace('#', '');
  const r = parseInt(normalized.slice(0, 2), 16) / 255;
  const g = parseInt(normalized.slice(2, 4), 16) / 255;
  const b = parseInt(normalized.slice(4, 6), 16) / 255;
  return { r, g, b, a: 1.0 };
};

const rgbaToHex = (r: number, g: number, b: number) => {
  const toHex = (v: number) => Math.round(v * 255).toString(16).padStart(2, '0');
  return `#${toHex(r)}${toHex(g)}${toHex(b)}`;
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
  width: min(560px, 92vw);
  max-height: 86vh;
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

.dialog-content {
  padding: 20px;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.section {
  display: grid;
  gap: 10px;
  padding: 12px;
  border-radius: 10px;
  background: rgba(0, 0, 0, 0.03);
}

.section-title {
  font-weight: 600;
  color: #1f2937;
}

.mode-row {
  display: flex;
  gap: 12px;
  flex-wrap: wrap;
}

.radio {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
  color: #475569;
}

.color-row {
  display: flex;
  align-items: center;
  gap: 12px;
}

.color-input {
  width: 44px;
  height: 36px;
  border: none;
  background: transparent;
}

.color-value {
  font-size: 12px;
  color: #475569;
}

.slider-row {
  display: grid;
  gap: 8px;
  font-size: 13px;
  color: #475569;
}

.tips {
  font-size: 12px;
  color: #0f766e;
  background: #ecfeff;
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
