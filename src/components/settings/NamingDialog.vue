<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";

const { t } = useI18n();
const visible = defineModel<boolean>({ required: true });
const props = defineProps<{ template: string }>();
const emit = defineEmits<{ save: [value: string] }>();

const draft = ref(props.template);
watch(visible, (open) => {
  if (open) draft.value = props.template;
});

const welcome = computed(() => t("settings.outputNaming.defaultName"));
const tags = computed(() => [
  { token: "[Name]", desc: t("settings.outputNaming.nameDesc") },
  { token: "[Ver]", desc: t("settings.outputNaming.verDesc") },
  { token: "[Time]", desc: t("settings.outputNaming.timeDesc") },
]);

const preview = computed(() => {
  const rendered = draft.value
    .replace(/\[Name\]/g, welcome.value)
    .replace(/\[Ver\]/g, "[Java 1.20-1.20.1]")
    .replace(/\[Time\]/g, "20260816-101234")
    .trim();
  return (rendered || welcome.value) + ".zip";
});

function insertTag(token: string) {
  const current = draft.value.trimEnd();
  draft.value = current ? `${current} ${token}` : token;
}

function save() {
  emit("save", draft.value);
  visible.value = false;
}
</script>

<template>
  <transition name="dialog-pop">
    <div v-if="visible" class="dialog-overlay">
      <div class="dialog-content naming-dialog" @click.stop>
        <div class="dialog-header">
          <h3>{{ t("settings.outputNaming.dialogTitle") }}</h3>
          <button class="dialog-close" @click="visible = false" :aria-label="t('common.close')">×</button>
        </div>
        <div class="dialog-body">
          <input
            v-model="draft"
            class="naming-input"
            :placeholder="t('settings.outputNaming.placeholder')"
            spellcheck="false"
            maxlength="200"
            @keyup.enter="save"
          />
          <div class="naming-row">
            <span class="naming-hint-label">{{ t("settings.outputNaming.tags") }}</span>
            <button v-for="tag in tags" :key="tag.token" class="naming-tag-btn" @click="insertTag(tag.token)">
              <span class="naming-tag-token">{{ tag.token }}</span>
              <span class="naming-tag-desc">{{ tag.desc }}</span>
            </button>
          </div>
          <div class="naming-preview">
            {{ t("settings.outputNaming.preview") }}:
            <b>{{ preview }}</b>
          </div>
        </div>
        <div class="dialog-footer">
          <button class="btn-text secondary" @click="visible = false">{{ t("common.cancel") }}</button>
          <button class="btn-text" @click="save">{{ t("common.save") }}</button>
        </div>
      </div>
    </div>
  </transition>
</template>

<style scoped>
.naming-dialog { width: min(520px, 92vw); }
.naming-input {
  width: 100%;
  height: 40px;
  padding: 0 12px;
  border: 1px solid rgba(0, 0, 0, 0.08);
  border-radius: 10px;
  background: rgba(0, 0, 0, 0.03);
  font-size: 14px;
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  outline: none;
}
.naming-row {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-top: 8px;
}
.naming-hint-label { font-size: 12px; font-weight: 700; color: #64748b; }
.naming-tag-btn {
  display: flex;
  align-items: baseline;
  gap: 10px;
  padding: 8px 10px;
  border: 1px solid rgba(0, 0, 0, 0.06);
  border-radius: 10px;
  background: transparent;
  cursor: pointer;
  text-align: left;
  font-family: inherit;
}
.naming-tag-btn:hover { background: rgba(0, 0, 0, 0.03); }
.naming-tag-token {
  font-size: 12.5px;
  font-weight: 700;
  color: var(--theme-color, #007bff);
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
}
.naming-tag-desc { font-size: 12px; color: #94a3b8; }
.naming-preview {
  margin-top: 8px;
  font-size: 12.5px;
  color: #64748b;
  word-break: break-all;
}
</style>
