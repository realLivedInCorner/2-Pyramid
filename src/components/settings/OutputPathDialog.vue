<script setup lang="ts">
import { ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { open } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";

const { t } = useI18n();
const visible = defineModel<boolean>({ required: true });
const props = defineProps<{ path: string; mode: "follow" | "fixed" }>();
const emit = defineEmits<{ save: [path: string] }>();

const draft = ref(props.path);
watch(visible, (openDlg) => {
  if (openDlg) draft.value = props.path;
});

async function browse() {
  try {
    const dir = await open({ directory: true, multiple: false });
    if (dir && typeof dir === "string") draft.value = dir;
  } catch (e) {
    console.error("pickOutputFolder failed", e);
  }
}

async function save() {
  if (props.mode === "fixed" && draft.value.trim().length > 0) {
    try {
      await invoke("create_dir", { path: draft.value });
    } catch (e) {
      console.error("create_dir failed", e);
    }
  }
  emit("save", draft.value);
  visible.value = false;
}
</script>

<template>
  <transition name="dialog-pop">
    <div v-if="visible" class="dialog-overlay" @click="visible = false">
      <div class="dialog-content" @click.stop>
        <div class="dialog-header">
          <h3>{{ t("settings.outputPath.dialogTitle") }}</h3>
          <button class="dialog-close" @click="visible = false" :aria-label="t('common.close')">×</button>
        </div>
        <div class="dialog-body">
          <label class="dialog-label">{{ t("settings.outputPath.dialogLabel") }}</label>
          <div class="dialog-input-row">
            <input class="dialog-input" v-model="draft" :placeholder="t('settings.outputPath.dialogPlaceholder')" />
            <button class="btn-text" @click="browse">{{ t("common.choose") }}</button>
          </div>
          <p class="dialog-hint">{{ t("settings.outputPath.dialogHint") }}</p>
        </div>
        <div class="dialog-footer">
          <button class="btn-text secondary" @click="visible = false">{{ t("common.cancel") }}</button>
          <button class="btn-text" @click="save">{{ t("common.save") }}</button>
        </div>
      </div>
    </div>
  </transition>
</template>
