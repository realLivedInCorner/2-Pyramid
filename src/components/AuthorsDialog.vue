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
}

.author-row {
  display: grid;
  grid-template-columns: 36px minmax(0, 120px) 1fr;
  gap: 10px;
  align-items: baseline;
  padding: 10px 4px;
}

.author-row + .author-row {
  border-top: 1px solid rgba(0, 0, 0, 0.05);
}

.author-id {
  font-size: 12px;
  font-weight: 700;
  color: #94a3b8;
  font-variant-numeric: tabular-nums;
}

.author-name {
  font-size: 14px;
  font-weight: 700;
  color: #1d1d1f;
}

.author-note {
  font-size: 12.5px;
  color: #64748b;
  line-height: 1.45;
}
</style>
