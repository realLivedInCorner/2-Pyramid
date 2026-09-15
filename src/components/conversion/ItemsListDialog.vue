<script setup lang="ts">
import { useI18n } from "vue-i18n";

const { t } = useI18n();
const visible = defineModel<boolean>({ required: true });
defineProps<{
  items: Array<{ name: string; size?: string }>;
}>();
const emit = defineEmits<{ remove: [index: number] }>();
</script>

<template>
  <transition name="dialog-pop">
    <div v-if="visible" class="dialog-overlay" @click="visible = false">
      <div class="dialog-content items-dialog" @click.stop>
        <div class="dialog-header">
          <h3>{{ t("conversion.fullImportList", { count: items.length }) }}</h3>
          <button class="dialog-close" @click="visible = false" :aria-label="t('common.close')">×</button>
        </div>
        <div class="items-dialog-list">
          <div v-for="(item, idx) in items" :key="`${item.name}-${idx}`" class="item-row">
            <div class="item-info">
              <span class="item-icon"><i class="ri-file-3-line" aria-hidden="true"></i></span>
              <span class="item-name">{{ item.name }}</span>
            </div>
            <div class="item-actions">
              <span class="item-size">{{ item.size }}</span>
              <button class="remove-item-btn" @click.stop="emit('remove', idx)">×</button>
            </div>
          </div>
        </div>
        <div class="dialog-footer">
          <button class="btn-text secondary" @click="visible = false">{{ t("common.close") }}</button>
        </div>
      </div>
    </div>
  </transition>
</template>

<style scoped>
.items-dialog {
  width: min(560px, 92vw);
}

.items-dialog-list {
  max-height: 50vh;
  overflow: auto;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.item-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  padding: 10px 12px;
  border-radius: 10px;
  background: rgba(0, 0, 0, 0.02);
  text-align: left;
}

.item-info {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}

.item-name {
  font-size: 13px;
  font-weight: 600;
  color: #1d1d1f;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.item-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}

.item-size {
  font-size: 11.5px;
  color: #94a3b8;
}

.remove-item-btn {
  border: none;
  background: transparent;
  color: #94a3b8;
  cursor: pointer;
  font-size: 16px;
  line-height: 1;
  padding: 2px 6px;
  border-radius: 6px;
}

.remove-item-btn:hover {
  color: #dc2626;
  background: rgba(239, 68, 68, 0.08);
}
</style>
