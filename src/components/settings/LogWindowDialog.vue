<script setup lang="ts">
import { onUnmounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import { useNotification } from "../../composables/useNotification";

const { t } = useI18n();
const { notify } = useNotification();
const visible = defineModel<boolean>({ required: true });

const logsText = ref("");
let timer: ReturnType<typeof setInterval> | null = null;

async function refresh() {
  try {
    logsText.value = await invoke<string>("get_logs");
  } catch (e) {
    logsText.value = t("settings.devMode.logError", { error: e });
  }
}

async function exportLog() {
  try {
    const defaultPath = await invoke<string | null>("get_log_path");
    const dest = await save({
      defaultPath: defaultPath || undefined,
      filters: [{ name: "Log", extensions: ["log", "txt"] }],
    });
    if (!dest) return;
    const result = await invoke<string>("export_logs", { dest });
    await notify({
      title: t("common.success"),
      body: t("settings.devMode.exportSuccess", { path: result }),
      type: "success",
      source: "system",
    });
  } catch (e) {
    await notify({
      title: t("common.error"),
      body: t("settings.devMode.exportFailed", { error: String(e) }),
      type: "error",
      source: "system",
    });
  }
}

function stopTimer() {
  if (timer) {
    clearInterval(timer);
    timer = null;
  }
}

function close() {
  visible.value = false;
  stopTimer();
}

watch(visible, async (open) => {
  if (!open) {
    stopTimer();
    return;
  }
  await refresh();
  if (timer) clearInterval(timer);
  timer = setInterval(refresh, 1200);
});

onUnmounted(stopTimer);
</script>

<template>
  <transition name="dialog-pop">
    <div v-if="visible" class="dialog-overlay" @click="close">
      <div class="dialog-content log-dialog" @click.stop>
        <div class="dialog-header">
          <h3>{{ t("settings.devMode.logWindowTitle") }}</h3>
          <button class="dialog-close" @click="close" :aria-label="t('common.close')">×</button>
        </div>
        <div class="dialog-body">
          <div class="log-toolbar">
            <button class="btn-text secondary" @click="refresh">{{ t("common.refresh") }}</button>
            <button class="btn-text" @click="exportLog">{{ t("settings.devMode.exportLog") }}</button>
          </div>
          <pre class="log-output">{{ logsText || t("common.noLogs") }}</pre>
        </div>
      </div>
    </div>
  </transition>
</template>

<style scoped>
.log-dialog { width: min(720px, 92vw); }
.log-toolbar {
  display: flex;
  gap: 8px;
  justify-content: flex-end;
  margin-bottom: 8px;
}
.log-output {
  min-height: 220px;
  max-height: 50vh;
  overflow: auto;
  padding: 12px;
  border-radius: 12px;
  background: #0f172a;
  color: #e2e8f0;
  font-size: 11.5px;
  line-height: 1.55;
  text-align: left;
  white-space: pre-wrap;
  word-break: break-all;
}
</style>
