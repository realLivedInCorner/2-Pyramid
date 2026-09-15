<script setup lang="ts">
import { useI18n } from "vue-i18n";

const { t } = useI18n();

const visible = defineModel<boolean>({ required: true });
const props = defineProps<{
  version: string;
  build: string;
  isBeta: boolean;
  devHint?: string;
}>();

const emit = defineEmits<{ tapVersion: [] }>();
</script>

<template>
  <transition name="dialog-pop-quick">
    <div v-if="visible" class="dialog-overlay" @click="visible = false">
      <div class="dialog-content version-dialog" @click.stop>
        <div class="dialog-header">
          <h3>{{ t("settings.versionInfo.dialogTitle") }}</h3>
          <button class="dialog-close" @click="visible = false">×</button>
        </div>
        <div class="dialog-body">
          <div class="version-hero">
            <img src="/favicon-192.png" class="version-logo" alt="2-Pyramid logo" />
            <div class="version-title-row">
              <button class="version-tag" @click="emit('tapVersion')">
                2-Pyramid v{{ props.version }}
              </button>
              <span v-if="props.isBeta" class="version-build-mode beta">Beta 版本 · 测试渠道</span>
            </div>
            <div v-if="props.devHint" class="dev-hint">{{ props.devHint }}</div>
          </div>
          <div class="version-facts">
            <div class="fact-row">
              <span class="fact-label">{{ t("settings.versionInfo.mainVersion") }}</span>
              <span class="fact-value">{{ props.version }}</span>
            </div>
            <div class="fact-row">
              <span class="fact-label">{{ t("settings.versionInfo.buildNumber") }}</span>
              <span class="fact-value">{{ props.build }}</span>
            </div>
          </div>
        </div>
        <div class="dialog-footer">
          <button class="btn-text" @click="visible = false">{{ t("common.close") }}</button>
        </div>
      </div>
    </div>
  </transition>
</template>

<style scoped>
.version-dialog {
  max-width: 500px;
}

.version-hero {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 14px;
  padding: 20px 0 22px;
  border-bottom: 1px solid #f1f5f9;
  margin-bottom: 20px;
}

.version-logo {
  width: 80px;
  height: 80px;
}

.version-title-row {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
  justify-content: center;
}

.version-tag {
  border: none;
  background: transparent;
  font-size: 20px;
  font-weight: 800;
  color: #0f172a;
  cursor: pointer;
  font-family: inherit;
  padding: 0;
  letter-spacing: -0.02em;
  transition: color 0.15s ease;
}

.version-tag:hover {
  color: var(--theme-color, #0f172a);
}

.version-build-mode {
  display: inline-flex;
  align-items: center;
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.04em;
  padding: 3px 10px;
  border-radius: 999px;
  background: rgba(249, 115, 22, 0.12);
  color: #ea580c;
}

.dev-hint {
  font-size: 12px;
  font-weight: 600;
  color: var(--theme-color, #64748b);
}

.version-facts {
  display: flex;
  flex-direction: column;
  border: 1px solid #eef2f7;
  border-radius: 14px;
  background: rgba(255, 255, 255, 0.65);
  overflow: hidden;
}

.fact-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 13px 16px;
}

.fact-row + .fact-row {
  border-top: 1px solid #eef2f7;
}

.fact-label {
  font-size: 13px;
  color: #64748b;
  font-weight: 600;
}

.fact-value {
  font-size: 13px;
  font-weight: 800;
  color: #0f172a;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-variant-numeric: tabular-nums;
  letter-spacing: 0.2px;
}
</style>
