<script setup lang="ts">
import { ref, watch, computed } from "vue";
import { useI18n } from "vue-i18n";

const { t } = useI18n();
const visible = defineModel<boolean>({ required: true });
const emit = defineEmits<{ import: [code: string] }>();

const code = ref("");
watch(visible, (open) => {
  if (open) code.value = "";
});

const canImport = computed(
  () => code.value.startsWith("2PYR-") || code.value.startsWith("HRCN-"),
);

function submit() {
  if (!canImport.value) return;
  emit("import", code.value.trim());
  visible.value = false;
}
</script>

<template>
  <transition name="dialog-pop-quick">
    <div v-if="visible" class="dialog-overlay" @click.self="visible = false">
      <div class="simple-dialog dialog-content">
        <h3>{{ t("overlay.importTitle") }}</h3>
        <p class="dialog-desc">{{ t("overlay.importDesc") }}</p>
        <textarea
          v-model="code"
          :placeholder="t('overlay.importPlaceholder')"
          class="share-textarea"
        ></textarea>
        <div class="dialog-footer">
          <button class="btn-text secondary" @click="visible = false">{{ t("common.cancel") }}</button>
          <button class="btn-text" :disabled="!canImport" @click="submit">
            {{ t("common.import") }}
          </button>
        </div>
      </div>
    </div>
  </transition>
</template>

<style scoped>
.simple-dialog {
  width: min(440px, 92vw);
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
.share-textarea {
  width: 100%;
  min-height: 110px;
  resize: vertical;
  padding: 10px 12px;
  border: 1px solid rgba(0, 0, 0, 0.08);
  border-radius: 10px;
  background: rgba(0, 0, 0, 0.03);
  font-size: 13px;
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  outline: none;
}
</style>
