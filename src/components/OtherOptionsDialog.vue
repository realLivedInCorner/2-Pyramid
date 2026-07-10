<template>
  <div class="dialog-overlay">
    <div class="dialog-container">
      <div class="dialog-header">
        <h2 class="dialog-title">{{ t('dialog.otherOptions.title') }}</h2>
        <button class="icon-button" @click="closeDialog" :aria-label="t('common.close')">
          <i class="ri-close-line" aria-hidden="true"></i>
        </button>
      </div>

      <div class="dialog-content">
        <div class="section">
          <div class="section-title">{{ t('dialog.otherOptions.rendering') }}</div>
          <label class="option-row">
            <input type="checkbox" v-model="noShadowEnabled" />
            <span>{{ t('dialog.otherOptions.noShadow') }}</span>
          </label>
          <p class="helper-text">{{ t('dialog.otherOptions.noShadowHint') }}</p>
        </div>
      </div>

      <div class="dialog-footer">
        <button class="ghost-btn" @click="closeDialog">{{ t('common.cancel') }}</button>
        <button class="primary-btn" @click="saveSettings" :disabled="isSaving">
          {{ isSaving ? t('common.saving') : t('common.saveSettings') }}
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

const noShadowEnabled = ref(false);
const isSaving = ref(false);

const loadSettings = async () => {
  try {
    const settings = await invoke('get_overlay_settings');
    if (settings && typeof settings === 'object') {
      const s = settings as any;
      if (s.core_shadow) {
        noShadowEnabled.value = !!s.core_shadow.enabled;
      } else if (s.no_shadow) {
        noShadowEnabled.value = !!s.no_shadow.enabled;
      }
    }
  } catch (error) {
    console.error('Failed to load other options:', error);
  }
};

const saveSettings = async () => {
  isSaving.value = true;
  try {
    await invoke('save_overlay_settings', {
      settings: {
        core_shadow: { enabled: noShadowEnabled.value }
      }
    });
    await message(t('dialog.otherOptions.saved'), { title: t('dialog.otherOptions.saveSuccess'), kind: 'info' });
    emit('close');
  } catch (error) {
    console.error('Failed to save other options:', error);
    await message(t('dialog.otherOptions.saveFailed', { error }), { title: t('dialog.otherOptions.saveFailedTitle'), kind: 'error' });
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
  width: min(520px, 92vw);
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

.option-row {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  color: #475569;
}

.helper-text {
  font-size: 12px;
  color: #94a3b8;
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
