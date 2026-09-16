<script setup lang="ts">
import { useI18n } from "vue-i18n";
import { openUrl } from "@tauri-apps/plugin-opener";

const { t } = useI18n();
const visible = defineModel<boolean>({ required: true });
const emit = defineEmits<{ confirm: [] }>();

function openBug() {
  void openUrl("https://bugs.mojang.com/browse/MC-311807").catch(() => {});
}
</script>

<template>
  <transition name="dialog-pop">
    <div v-if="visible" class="dialog-overlay" @click.self="visible = false">
      <div class="dialog-content" @click.stop>
        <div class="dialog-header">
          <h3>{{ t("conversion.java263Warn.title") }}</h3>
          <button class="dialog-close" @click="visible = false" :aria-label="t('common.close')">×</button>
        </div>
        <div class="dialog-body">
          <p class="warn-text">{{ t("conversion.java263Warn.body") }}</p>
          <p class="warn-text">
            <a class="warn-link" href="#" @click.prevent="openBug">MC-311807</a>
          </p>
        </div>
        <div class="dialog-footer">
          <button class="btn-text secondary" @click="visible = false">{{ t("common.cancel") }}</button>
          <button class="btn-text" @click="emit('confirm')">
            {{ t("conversion.java263Warn.confirm") }}
          </button>
        </div>
      </div>
    </div>
  </transition>
</template>

<style scoped>
.warn-text {
  text-align: left;
  line-height: 1.65;
  color: #374151;
  font-size: 13.5px;
}
.warn-link {
  color: var(--theme-color, #007bff);
  text-decoration: underline;
  text-underline-offset: 2px;
}
</style>
