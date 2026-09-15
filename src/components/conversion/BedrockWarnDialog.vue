<script setup lang="ts">
import { useI18n } from "vue-i18n";

const { t } = useI18n();
const visible = defineModel<boolean>({ required: true });
const emit = defineEmits<{ confirm: [] }>();
</script>

<template>
  <transition name="dialog-pop">
    <div v-if="visible" class="dialog-overlay" @click.self="visible = false">
      <div class="dialog-content" @click.stop>
        <div class="dialog-header">
          <h3>{{ t("conversion.bedrockWarn.title") }}</h3>
          <button class="dialog-close" @click="visible = false" :aria-label="t('common.close')">×</button>
        </div>
        <div class="dialog-body">
          <p class="bedrock-warn-text">{{ t("conversion.bedrockWarn.body") }}</p>
        </div>
        <div class="dialog-footer">
          <button class="btn-text secondary" @click="visible = false">{{ t("common.cancel") }}</button>
          <button
            class="btn-text danger"
            @click="emit('confirm')"
          >
            {{ t("conversion.bedrockWarn.confirm") }}
          </button>
        </div>
      </div>
    </div>
  </transition>
</template>

<style scoped>
.bedrock-warn-text {
  text-align: left;
  line-height: 1.65;
  color: #374151;
  font-size: 13.5px;
}
</style>
