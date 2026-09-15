<script setup lang="ts">
import { ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { useNotification } from "../../composables/useNotification";

const { t } = useI18n();
const { notify } = useNotification();
const visible = defineModel<boolean>({ required: true });
const emit = defineEmits<{ reset: [] }>();

const deep = ref(false);
const busy = ref(false);

watch(visible, (open) => {
  if (!open) deep.value = false;
});

async function confirm() {
  if (busy.value) return;
  busy.value = true;
  const useDeep = deep.value;
  deep.value = false;
  try {
    if (useDeep) {
      await invoke("factory_reset_deep");
    } else {
      await invoke("factory_reset");
    }
    localStorage.removeItem("sourceHandling");
    localStorage.removeItem("openOutputAfterConvert");
    localStorage.removeItem("animationSpeed");
    localStorage.removeItem("animationEnabled");
    localStorage.removeItem("themeColor");
    localStorage.removeItem("language");
    visible.value = false;
    busy.value = false;
    emit("reset");
  } catch (e) {
    await notify({
      title: t("settings.factoryReset.failedTitle"),
      body: String(e),
      type: "error",
      source: "system",
    });
    busy.value = false;
  }
}
</script>

<template>
  <transition name="dialog-pop-quick">
    <div v-if="visible" class="dialog-overlay" @click.self="visible = false">
      <div class="dialog-content confirm-dialog" @click.stop>
        <div class="dialog-header">
          <h3>{{ t("settings.factoryReset.confirmTitle") }}</h3>
          <button class="dialog-close" @click="visible = false" :aria-label="t('common.close')">×</button>
        </div>
        <div class="dialog-body">
          <p>{{ t("settings.factoryReset.confirmBody") }}</p>
          <label class="dialog-checkbox-row" @click.stop>
            <input type="checkbox" v-model="deep" />
            <span class="dialog-checkbox-text">{{ t("settings.factoryReset.deepLabel") }}</span>
            <span class="dialog-checkbox-hint">{{ t("settings.factoryReset.deepHint") }}</span>
          </label>
        </div>
        <div class="dialog-footer">
          <button class="btn-text secondary" @click="visible = false">
            {{ t("settings.factoryReset.cancelBtn") }}
          </button>
          <button class="btn-text danger" @click="confirm" :disabled="busy">
            {{ busy ? t("common.loading") : t("settings.factoryReset.confirmBtn") }}
          </button>
        </div>
      </div>
    </div>
  </transition>
</template>

<style scoped>
.confirm-dialog { width: min(440px, 92vw); }
.dialog-checkbox-row {
  display: grid;
  grid-template-columns: auto 1fr;
  gap: 4px 10px;
  align-items: start;
  text-align: left;
  cursor: pointer;
  margin-top: 8px;
}
.dialog-checkbox-row input { margin-top: 3px; }
.dialog-checkbox-text {
  grid-column: 2;
  font-size: 13.5px;
  font-weight: 600;
  color: #1d1d1f;
}
.dialog-checkbox-hint {
  grid-column: 2;
  font-size: 12px;
  color: #94a3b8;
  line-height: 1.5;
}
</style>
