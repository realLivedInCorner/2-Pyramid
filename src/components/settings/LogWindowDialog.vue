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

// 未脱敏导出（仅开发者模式）：两步确认，避免顺手把原文发出去
const devMode = ref(false);
const confirmRaw = ref(false);

interface LogExportResult {
  path: string;
  redacted: number;
  redaction: boolean;
  scope: string;
  files: number;
}

async function refresh() {
  try {
    logsText.value = await invoke<string>("get_logs");
  } catch (e) {
    logsText.value = t("settings.devMode.logError", { error: e });
  }
}

/// 导出日志：`scope` = session（内存会话日志）| disk（logs 目录下全部 .log）
/// `raw` = true 时导出未脱敏原文（仅开发者模式，后端也会校验）
async function exportLog(scope: "session" | "disk" = "session", raw = false) {
  try {
    const defaultPath = await invoke<string | null>("get_log_path");
    const dest = await save({
      defaultPath: defaultPath || undefined,
      filters: [{ name: "Log", extensions: ["log", "txt"] }],
    });
    if (!dest) return;
    const result = await invoke<LogExportResult>("export_logs", {
      dest,
      raw: raw ? true : undefined,
      scope,
    });
    let body: string;
    if (!result.redaction) {
      body =
        result.scope === "disk"
          ? t("settings.devMode.exportDiskSuccessRaw", { path: result.path, files: result.files })
          : t("settings.devMode.exportSuccess", { path: result.path });
    } else if (result.scope === "disk") {
      body = t("settings.devMode.exportDiskSuccessRedacted", {
        path: result.path,
        count: result.redacted,
        files: result.files,
      });
    } else {
      body = t("settings.devMode.exportSuccessRedacted", {
        path: result.path,
        count: result.redacted,
      });
    }
    await notify({
      title: t("common.success"),
      body,
      type: "success",
      source: "system",
    });
    confirmRaw.value = false;
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
    confirmRaw.value = false;
    return;
  }
  try {
    devMode.value = await invoke<boolean>("get_dev_mode");
  } catch {
    devMode.value = false;
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
            <button class="btn-text" @click="exportLog('session', false)">{{ t("settings.devMode.exportLog") }}</button>
            <button class="btn-text" @click="exportLog('disk', false)">{{ t("settings.devMode.exportDiskLog") }}</button>
            <button
              v-if="devMode"
              class="btn-text raw-export-btn"
              @click="confirmRaw = !confirmRaw"
            >{{ t("settings.devMode.exportRawLog") }}</button>
          </div>
          <div v-if="confirmRaw" class="raw-warn">
            <i class="ri-alert-line" aria-hidden="true"></i>
            <span>{{ t("settings.devMode.exportRawWarn") }}</span>
            <button class="btn-text danger" @click="exportLog('session', true)">
              {{ t("settings.devMode.exportRawSessionConfirm") }}
            </button>
            <button class="btn-text danger" @click="exportLog('disk', true)">
              {{ t("settings.devMode.exportRawDiskConfirm") }}
            </button>
            <button class="btn-text secondary" @click="confirmRaw = false">{{ t("common.cancel") }}</button>
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
.raw-export-btn { color: #b45309; }
.raw-warn {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
  margin-bottom: 8px;
  padding: 8px 12px;
  border-radius: 10px;
  background: rgba(217, 119, 6, 0.1);
  border: 1px solid rgba(217, 119, 6, 0.25);
  color: #b45309;
  font-size: 12px;
  text-align: left;
}
.raw-warn i { font-size: 15px; }
.raw-warn span { flex: 1; min-width: 200px; }
.btn-text.danger { color: #dc2626; }
.btn-text.danger:hover { background: rgba(220, 38, 38, 0.1); }
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
