<script setup lang="ts">
import { useI18n } from "vue-i18n";

const { t } = useI18n();
const visible = defineModel<boolean>({ required: true });
const props = defineProps<{ code: string }>();
const emit = defineEmits<{ copy: [] }>();
</script>

<template>
  <transition name="dialog-pop">
    <div v-if="visible" class="dialog-overlay" @click.self="visible = false">
      <div class="simple-dialog dialog-content">
        <h3>{{ t("overlay.exportTitle") }}</h3>
        <p class="dialog-desc">{{ t("overlay.exportDesc") }}</p>
        <div class="share-code-box">
          <code>{{ props.code }}</code>
        </div>
        <div class="dialog-footer">
          <button class="btn-text secondary" @click="visible = false">{{ t("common.close") }}</button>
          <button class="btn-text" @click="emit('copy')">{{ t("overlay.copyCode") }}</button>
        </div>
      </div>
    </div>
  </transition>
</template>

<style scoped>
.simple-dialog {
  width: min(480px, 92vw);
  display: flex;
  flex-direction: column;
  gap: 12px;
  text-align: left;
}
.simple-dialog h3 {
  margin: 0;
  font-size: 16px;
  font-weight: 700;
  color: #1d1d1f;
  text-align: center;
}
.dialog-desc {
  font-size: 12.5px;
  color: #94a3b8;
  line-height: 1.55;
}
.share-code-box {
  padding: 12px 14px;
  border-radius: 12px;
  background: rgba(0, 0, 0, 0.04);
  word-break: break-all;
}
.share-code-box code {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 12.5px;
  color: #1d1d1f;
}
</style>
