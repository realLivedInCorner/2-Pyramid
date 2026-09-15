<script setup lang="ts">
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";

const { t } = useI18n();
const visible = defineModel<boolean>({ required: true });

async function confirmClear() {
  visible.value = false;
  try {
    await invoke<string>("clear_config");
    localStorage.clear();
    setTimeout(() => {
      window.location.reload();
    }, 600);
  } catch (e) {
    console.error("[dev] clear_config failed:", e);
  }
}
</script>

<template>
  <transition name="dialog-pop">
    <div v-if="visible" class="dialog-overlay" @click="visible = false">
      <div class="dialog-content confirm-dialog" @click.stop>
        <div class="dialog-header">
          <h3>{{ t("settings.devMode.clearConfigConfirmTitle") }}</h3>
          <button class="dialog-close" @click="visible = false" :aria-label="t('common.close')">×</button>
        </div>
        <div class="dialog-body">
          <p>{{ t("settings.devMode.clearConfigConfirmBody") }}</p>
        </div>
        <div class="dialog-footer">
          <button class="btn-text secondary" @click="visible = false">
            {{ t("settings.devMode.clearConfigConfirmCancel") }}
          </button>
          <button class="btn-text danger" @click="confirmClear">
            {{ t("settings.devMode.clearConfigConfirmOk") }}
          </button>
        </div>
      </div>
    </div>
  </transition>
</template>

<style scoped>
.confirm-dialog { width: min(420px, 92vw); }
</style>
