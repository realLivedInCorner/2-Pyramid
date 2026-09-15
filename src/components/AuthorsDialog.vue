<script setup lang="ts">
import { useI18n } from "vue-i18n";

const { t } = useI18n();
const visible = defineModel<boolean>({ required: true });

const authors = [
  { id: 0, nameKey: "settings.versionInfo.author0Name", noteKey: "settings.versionInfo.author0Note" },
  { id: 1, nameKey: "settings.versionInfo.author1Name", noteKey: "settings.versionInfo.author1Note" },
  { id: 2, nameKey: "settings.versionInfo.author2Name", noteKey: "settings.versionInfo.author2Note" },
  { id: 3, nameKey: "settings.versionInfo.author3Name", noteKey: "settings.versionInfo.author3Note" },
];
</script>

<template>
  <transition name="dialog-pop-quick">
    <div v-if="visible" class="dialog-overlay" @click="visible = false">
      <div class="dialog-content authors-dialog" @click.stop>
        <div class="dialog-header">
          <h3>{{ t("settings.versionInfo.authors") }}</h3>
          <button class="dialog-close" @click="visible = false">×</button>
        </div>
        <div class="dialog-body">
          <div class="authors-list">
            <div v-for="a in authors" :key="a.id" class="author-row">
              <span class="author-id">#{{ a.id }}</span>
              <span class="author-name">{{ t(a.nameKey) }}</span>
              <span class="author-note">{{ t(a.noteKey) }}</span>
            </div>
          </div>
        </div>
        <div class="dialog-footer">
          <button class="btn-text" @click="visible = false">{{ t("common.close") }}</button>
        </div>
      </div>
    </div>
  </transition>
</template>

<style scoped>
.authors-dialog {
  max-width: min(520px, 92vw);
}

.authors-list {
  display: flex;
  flex-direction: column;
  border: 1px solid #eef2f7;
  border-radius: 14px;
  background: rgba(255, 255, 255, 0.65);
  overflow: hidden;
}

.author-row {
  display: grid;
  grid-template-columns: 40px 1fr;
  column-gap: 10px;
  row-gap: 2px;
  padding: 12px 16px;
  align-items: baseline;
}

.author-row + .author-row {
  border-top: 1px solid #eef2f7;
}

.author-id {
  grid-row: 1 / span 2;
  font-size: 12px;
  font-weight: 700;
  color: color-mix(in srgb, var(--theme-color, #64748b) 75%, #000);
  font-variant-numeric: tabular-nums;
}

.author-name {
  font-size: 14px;
  font-weight: 700;
  color: #1d1d1f;
  word-break: break-word;
}

.author-note {
  font-size: 12px;
  color: #6b7280;
  line-height: 1.45;
}
</style>
