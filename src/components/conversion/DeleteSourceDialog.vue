<script setup lang="ts">
import { useI18n } from "vue-i18n";

const { t } = useI18n();
const visible = defineModel<boolean>({ required: true });
const emit = defineEmits<{ confirm: [] }>();
</script>

<template>
  <transition name="dialog-pop">
    <div v-if="visible" class="dialog-overlay" @click="visible = false">
      <div class="dialog-content" @click.stop>
        <div class="dialog-header">
          <h3>{{ t("settings.sourceHandling.deleteTitle") }}</h3>
          <button class="dialog-close" @click="visible = false">×</button>
        </div>
        <div class="dialog-body">
          <p>{{ t("settings.sourceHandling.deleteBody") }}</p>
        </div>
        <div class="dialog-footer">
          <button class="btn-text secondary" @click="visible = false">{{ t("common.cancel") }}</button>
          <button class="btn-text danger" @click="emit('confirm')">
            {{ t("settings.sourceHandling.delete") }}
          </button>
        </div>
      </div>
    </div>
  </transition>
</template>
