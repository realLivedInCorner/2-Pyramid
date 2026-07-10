<template>
  <Transition name="notification-slide">
    <div v-if="isVisible && currentNotification" class="notification-container" :class="{ 'outside-window': !isWindowFocused }">
      <div class="notification-card" :class="`notification-${currentNotification.type}`">
        <div class="notification-header">
          <div class="notification-app-icon">
            <svg viewBox="0 0 680 680" fill="none" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
              <g stroke="#000" stroke-linecap="round">
                <line x1="260" y1="520" x2="200" y2="390" stroke-width="7"/>
                <line x1="260" y1="520" x2="500" y2="190" stroke-width="7"/>
                <line x1="200" y1="390" x2="500" y2="190" stroke-width="7"/>
                <line x1="260" y1="520" x2="374" y2="274" stroke-width="5"/>
              </g>
            </svg>
          </div>
          <div class="notification-app-name">2-Pyramid</div>
          <button class="notification-close" @click="dismiss" :aria-label="t('common.closeNotify')">
            <i class="ri-close-line"></i>
          </button>
        </div>
        <div class="notification-body">
          <div class="notification-icon">
            <i :class="iconClass" aria-hidden="true"></i>
          </div>
          <div class="notification-content">
            <div class="notification-title">{{ currentNotification.title }}</div>
            <div class="notification-text">{{ currentNotification.body }}</div>
          </div>
        </div>
      </div>
    </div>
  </Transition>
</template>

<script setup lang="ts">
import { computed, ref, onMounted, onUnmounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { useNotification } from '../composables/useNotification';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { isTauri } from '@tauri-apps/api/core';

const { t } = useI18n();
const { isVisible, currentNotification, dismiss } = useNotification();

const isWindowFocused = ref(true);

let unlistenFocus: (() => void) | null = null;

onMounted(async () => {
  if (isTauri()) {
    try {
      const window = getCurrentWindow();
      unlistenFocus = await window.onFocusChanged(({ payload }) => {
        isWindowFocused.value = payload;
      });
      isWindowFocused.value = await window.isFocused();
    } catch (e) {
      console.warn('Failed to listen window focus:', e);
    }
  }
});

onUnmounted(() => {
  if (unlistenFocus) unlistenFocus();
});

const iconClass = computed(() => {
  if (!currentNotification.value) return '';
  const icons: Record<string, string> = {
    info: 'ri-information-line',
    success: 'ri-checkbox-circle-line',
    error: 'ri-error-warning-line',
    warning: 'ri-alert-line'
  };
  return icons[currentNotification.value.type] || icons.info;
});
</script>

<style scoped>
.notification-container {
  position: fixed;
  bottom: calc(20px * var(--dpi-scale, 1));
  right: calc(20px * var(--dpi-scale, 1));
  z-index: 99999;
  pointer-events: none;
  transition: all 0.3s ease;
}

@media (min-resolution: 1.25dppx) {
  .notification-container {
    --dpi-scale: 0.9;
  }
}

@media (min-resolution: 1.5dppx) {
  .notification-container {
    --dpi-scale: 0.85;
  }
}

@media (min-resolution: 2dppx) {
  .notification-container {
    --dpi-scale: 0.8;
  }
}

.notification-container.outside-window {
  bottom: calc(20px * var(--dpi-scale, 1));
  right: calc(20px * var(--dpi-scale, 1));
}

.notification-card {
  pointer-events: auto;
  width: calc(360px * var(--dpi-scale, 1));
  background: #ffffff;
  border-radius: calc(8px * var(--dpi-scale, 1));
  box-shadow: 
    0 calc(8px * var(--dpi-scale, 1)) calc(32px * var(--dpi-scale, 1)) rgba(0, 0, 0, 0.12),
    0 calc(2px * var(--dpi-scale, 1)) calc(8px * var(--dpi-scale, 1)) rgba(0, 0, 0, 0.08);
  border: 1px solid rgba(0, 0, 0, 0.08);
  overflow: hidden;
  transition: all 0.3s cubic-bezier(0.2, 0.8, 0.2, 1);
}

.notification-card:hover {
  transform: translateY(calc(-2px * var(--dpi-scale, 1)));
  box-shadow: 
    0 calc(12px * var(--dpi-scale, 1)) calc(40px * var(--dpi-scale, 1)) rgba(0, 0, 0, 0.16),
    0 calc(4px * var(--dpi-scale, 1)) calc(12px * var(--dpi-scale, 1)) rgba(0, 0, 0, 0.1);
}

.notification-header {
  display: flex;
  align-items: center;
  padding: calc(10px * var(--dpi-scale, 1)) calc(12px * var(--dpi-scale, 1)) calc(8px * var(--dpi-scale, 1));
  gap: calc(8px * var(--dpi-scale, 1));
  background: #f8f9fa;
  border-bottom: 1px solid rgba(0, 0, 0, 0.06);
}

.notification-app-icon {
  width: calc(28px * var(--dpi-scale, 1));
  height: calc(28px * var(--dpi-scale, 1));
  display: flex;
  align-items: center;
  justify-content: center;
  color: #1e293b;
}

.notification-app-name {
  flex: 1;
  font-size: calc(12px * var(--dpi-scale, 1));
  font-weight: 600;
  color: #6c757d;
}

.notification-close {
  width: calc(24px * var(--dpi-scale, 1));
  height: calc(24px * var(--dpi-scale, 1));
  border-radius: calc(4px * var(--dpi-scale, 1));
  border: none;
  background: transparent;
  color: #adb5bd;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: calc(14px * var(--dpi-scale, 1));
  transition: all 0.2s;
}

.notification-close:hover {
  background: rgba(0, 0, 0, 0.06);
  color: #495057;
}

.notification-body {
  display: flex;
  gap: calc(12px * var(--dpi-scale, 1));
  padding: calc(12px * var(--dpi-scale, 1));
}

.notification-icon {
  width: calc(40px * var(--dpi-scale, 1));
  height: calc(40px * var(--dpi-scale, 1));
  border-radius: calc(8px * var(--dpi-scale, 1));
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: calc(20px * var(--dpi-scale, 1));
  flex-shrink: 0;
}

.notification-info .notification-icon {
  background: #e7f1ff;
  color: #0d6efd;
}

.notification-success .notification-icon {
  background: #d1e7dd;
  color: #198754;
}

.notification-error .notification-icon {
  background: #f8d7da;
  color: #dc3545;
}

.notification-warning .notification-icon {
  background: #fff3cd;
  color: #ffc107;
}

.notification-content {
  flex: 1;
  min-width: 0;
}

.notification-title {
  font-size: calc(14px * var(--dpi-scale, 1));
  font-weight: 600;
  color: #212529;
  margin-bottom: calc(4px * var(--dpi-scale, 1));
  line-height: 1.4;
}

.notification-text {
  font-size: calc(13px * var(--dpi-scale, 1));
  color: #6c757d;
  line-height: 1.5;
}

.notification-slide-enter-active {
  transition: all 0.35s cubic-bezier(0.2, 0.8, 0.2, 1);
}

.notification-slide-leave-active {
  transition: all 0.25s ease-in;
}

.notification-slide-enter-from {
  opacity: 0;
  transform: translateX(100%) scale(0.95);
}

.notification-slide-leave-to {
  opacity: 0;
  transform: translateX(100%) scale(0.95);
}
</style>