<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { resolveImageUrl } from "../../utils/assetUrl";
import { useNotification } from "../../composables/useNotification";

const { t } = useI18n();
const { notify } = useNotification();

type Fit = "cover" | "contain" | "stretch" | "tile";

const visible = defineModel<boolean>({ required: true });
const props = defineProps<{
  path: string | null;
  fit: Fit;
  opacity: number;
}>();
const emit = defineEmits<{
  applied: [payload: { path: string | null; fit: Fit; opacity: number; themeColor: string | null }];
  removed: [];
}>();

const draftFile = ref<string | null>(null);
const draftPreview = ref("");
const draftFit = ref<Fit>("cover");
const draftOpacity = ref(80);
const extractColor = ref(true);
const syncing = ref(false);

const previewStyle = computed(() => {
  if (!draftPreview.value) return {};
  return {
    backgroundImage: `url(${draftPreview.value})`,
    backgroundSize: draftFit.value === "stretch" ? "100% 100%" : draftFit.value,
    backgroundRepeat: draftFit.value === "tile" ? "repeat" : "no-repeat",
    backgroundPosition: "center",
    opacity: draftOpacity.value / 100,
  };
});

watch(visible, (open) => {
  if (!open) return;
  draftFile.value = null;
  draftPreview.value = "";
  draftFit.value = props.fit;
  draftOpacity.value = Math.round(props.opacity * 100);
  extractColor.value = true;
  syncing.value = false;
  if (props.path) {
    resolveImageUrl(props.path)
      .then((url) => {
        draftPreview.value = url;
      })
      .catch(() => {});
  }
});

function close() {
  if (syncing.value) return;
  visible.value = false;
}

async function pickImage() {
  try {
    const selected = await open({
      multiple: false,
      filters: [
        {
          name: t("settings.background.filter"),
          extensions: ["png", "jpg", "jpeg", "webp", "gif", "bmp"],
        },
      ],
    });
    if (selected && typeof selected === "string") {
      draftFile.value = selected;
      draftPreview.value = await resolveImageUrl(selected);
    }
  } catch (e) {
    console.error("[background] pick failed:", e);
  }
}

async function submit() {
  if (syncing.value || (!draftFile.value && !props.path)) return;
  syncing.value = true;
  try {
    if (draftFile.value) {
      const result = await invoke<{ background_path: string | null; theme_color: string | null }>(
        "set_background",
        {
          filePath: draftFile.value,
          fit: draftFit.value,
          opacity: draftOpacity.value / 100,
          extractColor: extractColor.value,
        },
      );
      emit("applied", {
        path: result.background_path,
        fit: draftFit.value,
        opacity: draftOpacity.value / 100,
        themeColor: result.theme_color,
      });
    } else {
      await invoke("update_background_settings", {
        fit: draftFit.value,
        opacity: draftOpacity.value / 100,
      });
      emit("applied", {
        path: props.path,
        fit: draftFit.value,
        opacity: draftOpacity.value / 100,
        themeColor: null,
      });
    }
    visible.value = false;
    notify({
      title: t("settings.background.dialogTitle"),
      body: t("settings.background.success"),
      type: "success",
      source: "system",
    });
  } catch (e) {
    notify({
      title: t("settings.background.dialogTitle"),
      body: t("settings.background.failed", { error: String(e) }),
      type: "error",
      source: "system",
    });
  } finally {
    syncing.value = false;
  }
}

async function remove() {
  if (syncing.value) return;
  syncing.value = true;
  try {
    await invoke("clear_background");
    draftFile.value = null;
    draftPreview.value = "";
    emit("removed");
    notify({
      title: t("settings.background.dialogTitle"),
      body: t("settings.background.removed"),
      type: "success",
      source: "system",
    });
  } catch (e) {
    notify({
      title: t("settings.background.dialogTitle"),
      body: t("settings.background.failed", { error: String(e) }),
      type: "error",
      source: "system",
    });
  } finally {
    syncing.value = false;
  }
}
</script>

<template>
  <transition name="dialog-pop">
    <div v-if="visible" class="dialog-overlay">
      <div class="dialog-content background-dialog" @click.stop>
        <div class="dialog-header">
          <h3>{{ t("settings.background.dialogTitle") }}</h3>
          <button class="dialog-close" @click="close" :aria-label="t('common.close')">×</button>
        </div>
        <div class="dialog-body">
          <div class="bg-preview" :style="previewStyle">
            <div v-if="!draftPreview" class="bg-preview-empty">
              <i class="ri-image-add-line" aria-hidden="true"></i>
              <span>{{ t("settings.background.noImage") }}</span>
            </div>
          </div>
          <div class="dialog-input-row">
            <button class="btn-text" @click="pickImage" :disabled="syncing">
              {{ t("settings.background.choose") }}
            </button>
            <button
              v-if="path"
              class="btn-text danger"
              @click="remove"
              :disabled="syncing"
            >
              {{ t("settings.background.remove") }}
            </button>
          </div>

          <div class="bg-row">
            <span class="bg-row-label">{{ t("settings.background.fit") }}</span>
            <div class="segmented">
              <button class="seg-btn" :class="{ active: draftFit === 'cover' }" @click="draftFit = 'cover'">{{ t("settings.background.fitCover") }}</button>
              <button class="seg-btn" :class="{ active: draftFit === 'contain' }" @click="draftFit = 'contain'">{{ t("settings.background.fitContain") }}</button>
              <button class="seg-btn" :class="{ active: draftFit === 'stretch' }" @click="draftFit = 'stretch'">{{ t("settings.background.fitStretch") }}</button>
              <button class="seg-btn" :class="{ active: draftFit === 'tile' }" @click="draftFit = 'tile'">{{ t("settings.background.fitTile") }}</button>
            </div>
          </div>

          <div class="bg-row">
            <span class="bg-row-label">{{ t("settings.background.opacity") }}</span>
            <input type="range" min="20" max="100" step="5" v-model.number="draftOpacity" class="bg-range" />
            <span class="bg-opacity-val">{{ draftOpacity }}%</span>
          </div>

          <div class="bg-row">
            <span class="bg-row-label">{{ t("settings.background.extractColor") }}</span>
            <label class="switch">
              <input type="checkbox" v-model="extractColor" />
              <span class="slider"></span>
            </label>
          </div>
          <p class="dialog-hint">{{ t("settings.background.extractColorDesc") }}</p>
        </div>
        <div class="dialog-footer">
          <button class="btn-text secondary" @click="close" :disabled="syncing">
            {{ t("common.cancel") }}
          </button>
          <button class="btn-text" @click="submit" :disabled="syncing || (!draftFile && !path)">
            {{ t("settings.background.apply") }}
          </button>
        </div>
        <div v-if="syncing" class="bg-syncing">
          <i class="ri-loader-4-line ri-spin" aria-hidden="true"></i>
          <span>Syncing...</span>
        </div>
      </div>
    </div>
  </transition>
</template>

<style scoped>
.background-dialog {
  width: 520px;
  max-width: 92vw;
  position: relative;
}
.bg-preview {
  width: 100%;
  height: 160px;
  border-radius: 14px;
  background: rgba(0, 0, 0, 0.04);
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
}
.bg-preview-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  color: #94a3b8;
  font-size: 13px;
}
.bg-preview-empty i { font-size: 28px; }
.bg-row {
  display: flex;
  align-items: center;
  gap: 12px;
  text-align: left;
}
.bg-row-label {
  width: 72px;
  flex-shrink: 0;
  font-size: 12.5px;
  font-weight: 700;
  color: #64748b;
}
.bg-range { flex: 1; accent-color: var(--theme-color, #007bff); }
.bg-opacity-val {
  width: 40px;
  text-align: right;
  font-size: 12px;
  color: #64748b;
  font-variant-numeric: tabular-nums;
}
.bg-syncing {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 10px;
  background: rgba(255, 255, 255, 0.72);
  backdrop-filter: blur(4px);
  border-radius: 20px;
  font-size: 13px;
  font-weight: 600;
  color: #64748b;
  z-index: 2;
}
.bg-syncing i { font-size: 28px; color: var(--theme-color, #007bff); }
</style>
