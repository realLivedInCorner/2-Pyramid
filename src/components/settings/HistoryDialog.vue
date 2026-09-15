<script setup lang="ts">
import { onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { useNotification } from "../../composables/useNotification";

const { t } = useI18n();
const { notify } = useNotification();
const visible = defineModel<boolean>({ required: true });

interface HistoryEntry {
  input: string;
  output: string | null;
  status: string;
  error: string | null;
  time: string;
  duration_s: number;
}

const entries = ref<HistoryEntry[]>([]);

const fileName = (p: string) => p.split(/[\\/]/).pop() ?? p;

async function load() {
  try {
    entries.value = (await invoke<HistoryEntry[]>("get_conversion_history")) ?? [];
  } catch {
    entries.value = [];
  }
}

function openOutput(path: string) {
  invoke("open_folder", { path }).catch(() => {});
}

async function clearAll() {
  try {
    await invoke("clear_conversion_history");
    entries.value = [];
    notify({
      title: t("settings.conversionHistory.groupTitle"),
      body: t("settings.conversionHistory.cleared"),
      type: "success",
      source: "system",
    });
  } catch {
    /* ignore */
  }
}

watch(visible, (open) => {
  if (open) void load();
});

onMounted(() => {
  if (visible.value) void load();
});
</script>

<template>
  <transition name="dialog-pop">
    <div v-if="visible" class="dialog-overlay">
      <div class="dialog-content history-dialog" @click.stop>
        <div class="dialog-header">
          <h3>{{ t("settings.conversionHistory.groupTitle") }}</h3>
          <button class="dialog-close" @click="visible = false" :aria-label="t('common.close')">×</button>
        </div>
        <div class="dialog-body">
          <div class="history-list" v-if="entries.length > 0">
            <div class="history-item" v-for="(h, i) in entries" :key="i">
              <i
                class="history-status"
                :class="h.status === 'success' ? 'ok' : h.status === 'cancelled' ? 'cancelled' : 'fail'"
                aria-hidden="true"
              ></i>
              <div class="history-info">
                <div class="history-name">{{ fileName(h.input) }}</div>
                <div class="history-meta">{{ h.time }} · {{ h.duration_s.toFixed(1) }}s</div>
              </div>
              <button
                v-if="h.status === 'success' && h.output"
                class="btn-text"
                @click="openOutput(h.output)"
              >{{ t("settings.conversionHistory.openOutput") }}</button>
            </div>
          </div>
          <div class="history-empty" v-else>{{ t("settings.conversionHistory.empty") }}</div>
        </div>
        <div class="dialog-footer">
          <button class="btn-text secondary" @click="clearAll" :disabled="entries.length === 0">
            {{ t("settings.conversionHistory.clear") }}
          </button>
          <button class="btn-text" @click="visible = false">{{ t("common.close") }}</button>
        </div>
      </div>
    </div>
  </transition>
</template>

<style scoped>
.history-dialog { width: 520px; max-width: 92vw; }
.history-list { display: flex; flex-direction: column; gap: 4px; }
.history-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 8px;
  border-radius: 10px;
}
.history-item:hover { background: rgba(0, 0, 0, 0.02); }
.history-status {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
  background: #94a3b8;
}
.history-status.ok { background: #10b981; }
.history-status.fail { background: #ef4444; }
.history-status.cancelled { background: #f59e0b; }
.history-info { flex: 1; min-width: 0; text-align: left; }
.history-name {
  font-size: 13px;
  font-weight: 600;
  color: #1d1d1f;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.history-meta { font-size: 11.5px; color: #94a3b8; }
.history-empty {
  padding: 28px 12px;
  text-align: center;
  font-size: 13px;
  color: #94a3b8;
}
</style>
