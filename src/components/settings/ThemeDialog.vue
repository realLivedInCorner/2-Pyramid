<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import HsvColorPicker from "../HsvColorPicker.vue";

const { t } = useI18n();
const visible = defineModel<boolean>({ required: true });
const props = defineProps<{
  color: string;
  defaultColor: string;
  showReset?: boolean;
  startReset?: boolean;
}>();
const emit = defineEmits<{
  confirm: [color: string];
  reset: [];
}>();

const draft = ref(props.color);
const showReset = ref(false);

watch(visible, (open) => {
  if (open) {
    draft.value = props.color;
    showReset.value = !!props.startReset;
  }
});

const rgba = computed({
  get: () => {
    const n = draft.value.replace("#", "");
    return {
      r: parseInt(n.slice(0, 2), 16) / 255,
      g: parseInt(n.slice(2, 4), 16) / 255,
      b: parseInt(n.slice(4, 6), 16) / 255,
      a: 1,
    };
  },
  set: (c: { r: number; g: number; b: number }) => {
    const to = (v: number) =>
      Math.round(Math.min(1, Math.max(0, v)) * 255)
        .toString(16)
        .padStart(2, "0");
    draft.value = `#${to(c.r)}${to(c.g)}${to(c.b)}`;
  },
});

function confirm() {
  emit("confirm", draft.value);
  visible.value = false;
}

function confirmReset() {
  emit("reset");
  showReset.value = false;
  visible.value = false;
}
</script>

<template>
  <transition name="dialog-pop">
    <div v-if="visible" class="dialog-overlay" @click="visible = false">
      <div class="dialog-content theme-dialog" @click.stop>
        <template v-if="!showReset">
          <div class="dialog-header">
            <h3>{{ t("settings.theme.dialogTitle") }}</h3>
            <button class="dialog-close" @click="visible = false" :aria-label="t('common.close')">×</button>
          </div>
          <div class="dialog-body">
            <label class="dialog-label">{{ t("settings.theme.currentColor") }}</label>
            <div class="theme-preview">
              <div class="preview-chip" :style="{ background: draft }"></div>
              <div class="preview-text">
                <div class="preview-title">2-Pyramid Theme</div>
                <div class="preview-sub">{{ t("settings.theme.previewSubtitle") }}</div>
              </div>
            </div>
            <div class="picker-area">
              <HsvColorPicker v-model="rgba" />
            </div>
            <p class="dialog-hint">{{ t("settings.theme.applyHint") }}</p>
          </div>
          <div class="dialog-footer">
            <button v-if="props.showReset" class="btn-text secondary" @click="showReset = true">
              {{ t("settings.theme.resetTitle") }}
            </button>
            <span class="footer-spacer"></span>
            <button class="btn-text secondary" @click="visible = false">{{ t("common.back") }}</button>
            <button class="btn-text" @click="confirm">{{ t("common.confirm") }}</button>
          </div>
        </template>
        <template v-else>
          <div class="dialog-header">
            <h3>{{ t("settings.theme.resetTitle") }}</h3>
            <button class="dialog-close" @click="showReset = false" :aria-label="t('common.close')">×</button>
          </div>
          <div class="dialog-body">
            <p class="dialog-hint">{{ t("settings.theme.resetBody") }}</p>
            <div class="theme-preview">
              <div class="preview-chip" :style="{ background: defaultColor }"></div>
              <div class="preview-text">
                <div class="preview-title">{{ t("settings.theme.resetPreview") }}</div>
                <div class="preview-sub">{{ defaultColor }}</div>
              </div>
            </div>
          </div>
          <div class="dialog-footer">
            <button class="btn-text secondary" @click="showReset = false">{{ t("common.back") }}</button>
            <button class="btn-text" @click="confirmReset">{{ t("common.confirm") }}</button>
          </div>
        </template>
      </div>
    </div>
  </transition>
</template>

<style scoped>
.theme-dialog { width: min(480px, 92vw); }
.theme-preview {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 14px;
  border-radius: 14px;
  background: rgba(0, 0, 0, 0.03);
  text-align: left;
}
.preview-chip {
  width: 48px;
  height: 48px;
  border-radius: 14px;
  box-shadow: inset 0 0 0 1px rgba(0, 0, 0, 0.06);
  flex-shrink: 0;
}
.preview-text { min-width: 0; }
.preview-title { font-size: 14px; font-weight: 700; color: #1d1d1f; }
.preview-sub { font-size: 12px; color: #94a3b8; margin-top: 2px; }
.picker-area { width: 100%; display: flex; justify-content: center; padding: 8px 0; }
.footer-spacer { flex: 1; }
</style>
