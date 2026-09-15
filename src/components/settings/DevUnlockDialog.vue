<script setup lang="ts">
import { ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";

const { t } = useI18n();
const visible = defineModel<boolean>({ required: true });
const emit = defineEmits<{ unlocked: [] }>();

const input = ref("");
const error = ref("");

watch(visible, (open) => {
  if (!open) {
    input.value = "";
    error.value = "";
  }
});

async function confirm() {
  error.value = "";
  if (input.value.trim() !== "DeveloperEnable") {
    error.value = t("settings.devMode.error");
    return;
  }
  visible.value = false;
  input.value = "";
  emit("unlocked");
  try {
    await invoke("set_dev_mode", { enabled: true });
  } catch {
    /* ignore */
  }
}

function cancel() {
  visible.value = false;
  input.value = "";
  error.value = "";
}
</script>

<template>
  <transition name="dialog-pop">
    <div v-if="visible" class="dialog-overlay" @click="cancel">
      <div class="dialog-content dev-dialog" @click.stop>
        <div class="dialog-header">
          <h3>{{ t("settings.devMode.title") }}</h3>
          <button class="dialog-close" @click="cancel" :aria-label="t('common.close')">×</button>
        </div>
        <div class="dialog-body">
          <div class="dialog-hint">{{ t("settings.devMode.hint") }}</div>
          <div class="dev-code">DeveloperEnable</div>
          <div class="dialog-input-row">
            <input
              class="dialog-input"
              v-model="input"
              :placeholder="t('settings.devMode.placeholder')"
              @keyup.enter="confirm"
            />
          </div>
          <div v-if="error" class="dev-error">{{ error }}</div>
        </div>
        <div class="dialog-footer">
          <button class="btn-text secondary" @click="cancel">{{ t("common.cancel") }}</button>
          <button class="btn-text" @click="confirm">{{ t("settings.devMode.enter") }}</button>
        </div>
      </div>
    </div>
  </transition>
</template>

<style scoped>
.dev-dialog { width: min(480px, 92vw); }
.dev-code {
  margin: 8px 0;
  padding: 10px 12px;
  border-radius: 10px;
  background: rgba(0, 0, 0, 0.04);
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 13px;
  font-weight: 700;
  color: #1d1d1f;
  text-align: center;
  user-select: all;
}
.dev-error {
  font-size: 12.5px;
  color: #dc2626;
  text-align: left;
}
</style>
