<script setup lang="ts">
import { useI18n } from "vue-i18n";

const { t } = useI18n();
const visible = defineModel<boolean>({ required: true });
const props = defineProps<{ name: string }>();
const emit = defineEmits<{ confirm: [] }>();
</script>

<template>
  <transition name="dialog-pop-quick">
    <div v-if="visible" class="dialog-overlay" @click.self="visible = false">
      <div class="simple-dialog dialog-content">
        <h3>{{ t("overlay.deleteTitle") }}</h3>
        <p class="dialog-desc">{{ t("overlay.deleteConfirm", { name: props.name }) }}</p>
        <div class="dialog-footer">
          <button class="btn-text secondary" @click="visible = false">{{ t("common.cancel") }}</button>
          <button class="btn-text danger" @click="emit('confirm')">
            {{ t("overlay.deleteConfirmBtn") }}
          </button>
        </div>
      </div>
    </div>
  </transition>
</template>

<style scoped>
.simple-dialog {
  width: min(420px, 92vw);
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
  font-size: 13px;
  color: #6b7280;
  line-height: 1.6;
}
</style>
