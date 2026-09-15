<script setup lang="ts">
import { ref, watch } from "vue";
import { useI18n } from "vue-i18n";

const { t } = useI18n();
const visible = defineModel<boolean>({ required: true });
const emit = defineEmits<{ create: [name: string] }>();

const name = ref("");
watch(visible, (open) => {
  if (open) name.value = "";
});

function submit() {
  if (!name.value.trim()) return;
  emit("create", name.value.trim());
  visible.value = false;
}
</script>

<template>
  <transition name="dialog-pop-quick">
    <div v-if="visible" class="dialog-overlay" @click.self="visible = false">
      <div class="simple-dialog dialog-content">
        <h3>{{ t("overlay.createTitle") }}</h3>
        <input
          v-model="name"
          :placeholder="t('overlay.createPlaceholder')"
          class="project-input"
          @keyup.enter="submit"
        />
        <div class="dialog-footer">
          <button class="btn-text secondary" @click="visible = false">{{ t("common.cancel") }}</button>
          <button class="btn-text" :disabled="!name" @click="submit">
            {{ t("common.create") }}
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
  gap: 14px;
  text-align: left;
}
.simple-dialog h3 {
  margin: 0;
  font-size: 16px;
  font-weight: 700;
  color: #1d1d1f;
  text-align: center;
}
.project-input {
  width: 100%;
  height: 40px;
  padding: 0 12px;
  border: 1px solid rgba(0, 0, 0, 0.08);
  border-radius: 10px;
  background: rgba(0, 0, 0, 0.03);
  font-size: 14px;
  outline: none;
  font-family: inherit;
}
</style>
