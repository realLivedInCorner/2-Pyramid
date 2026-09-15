<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { useSidebarSlide } from "../../composables/useSidebarSlide";
import {
  MINECRAFT_VERSIONS,
  groupVersionsByEra,
  type VersionEntry,
} from "../../data/minecraftVersions";

const { t } = useI18n();
const visible = defineModel<boolean>({ required: true });
const props = defineProps<{ selected: string }>();
const emit = defineEmits<{ pick: [entry: VersionEntry] }>();

const {
  onBeforeEnter,
  onEnter,
  onAfterEnter,
  onBeforeLeave,
  onLeave,
  onAfterLeave,
} = useSidebarSlide({ shadow: "-12px 0 36px rgba(0, 0, 0, 0.08)" });

const groups = computed(() => groupVersionsByEra(MINECRAFT_VERSIONS));
const selectedEntry = computed(
  () =>
    MINECRAFT_VERSIONS.find((v) => v.label === props.selected) ??
    MINECRAFT_VERSIONS[MINECRAFT_VERSIONS.length - 1],
);

function pick(entry: VersionEntry) {
  emit("pick", entry);
}
</script>

<template>
  <transition name="sidebar-overlay-fade">
    <div
      v-if="visible"
      class="sidebar-overlay"
      @click="visible = false"
      @keydown.esc="visible = false"
    ></div>
  </transition>

  <transition
    :css="false"
    @before-enter="onBeforeEnter"
    @enter="onEnter"
    @after-enter="onAfterEnter"
    @before-leave="onBeforeLeave"
    @leave="onLeave"
    @after-leave="onAfterLeave"
  >
    <aside
      v-if="visible"
      class="sidebar-content version-sidebar"
      @click.stop
      tabindex="-1"
    >
      <div class="sidebar-header">
        <div class="sidebar-header-text">
          <h3>{{ t("conversion.selectVersion") }}</h3>
          <p class="sidebar-hint">
            {{ selectedEntry.label }} ·
            {{ t("conversion.packFormat", { n: selectedEntry.packFormat }) }}
          </p>
        </div>
        <button
          class="sidebar-close"
          @click="visible = false"
          :aria-label="t('common.close')"
        >
          <i class="ri-close-line" aria-hidden="true"></i>
        </button>
      </div>
      <div class="sidebar-body">
        <div v-for="(group, gi) in groups" :key="group.era" class="version-era">
          <div class="version-era-header">
            <span class="version-era-name">{{ t(`conversion.versionEras.${group.era}`) }}</span>
            <span class="version-era-count">{{ group.items.length }}</span>
          </div>
          <div class="version-list">
            <button
              v-for="(v, i) in group.items"
              :key="v.label"
              class="version-row"
              :class="{
                active: v.label === selected,
                'has-status': v.status,
              }"
              :style="{ '--card-delay': `${gi * 40 + i * 16}ms` }"
              @click="pick(v)"
            >
              <div class="version-row-main">
                <span class="version-row-label">{{ v.label }}</span>
                <span class="version-row-meta">{{ t("conversion.packFormat", { n: v.packFormat }) }}</span>
              </div>
              <div class="version-row-tail">
                <span v-if="v.status" class="version-status" :class="`status-${v.status}`">
                  {{ t(`conversion.versionStatus.${v.status}`) }}
                </span>
                <i
                  v-if="v.label === selected"
                  class="ri-check-line version-row-check"
                  aria-hidden="true"
                ></i>
              </div>
            </button>
          </div>
        </div>
      </div>
    </aside>
  </transition>
</template>

<style scoped>
.sidebar-overlay {
  position: fixed;
  inset: 0;
  background: rgba(15, 23, 42, 0.28);
  z-index: 200;
  backdrop-filter: blur(4px);
}

.sidebar-content {
  position: fixed;
  top: 0;
  right: 0;
  bottom: 0;
  width: min(420px, 92vw);
  z-index: 210;
  background: rgba(255, 255, 255, 0.96);
  border-left: 1px solid rgba(0, 0, 0, 0.06);
  display: flex;
  flex-direction: column;
  opacity: 1 !important;
  outline: none;
}

.sidebar-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  padding: 22px 22px 14px;
  border-bottom: 1px solid rgba(0, 0, 0, 0.05);
}

.sidebar-header-text h3 {
  margin: 0;
  font-size: 16px;
  font-weight: 700;
  color: #1d1d1f;
}

.sidebar-hint {
  margin-top: 4px;
  font-size: 12px;
  color: #94a3b8;
}

.sidebar-close {
  border: none;
  background: rgba(0, 0, 0, 0.04);
  width: 32px;
  height: 32px;
  border-radius: 10px;
  cursor: pointer;
  color: #64748b;
  display: inline-flex;
  align-items: center;
  justify-content: center;
}

.sidebar-close:hover {
  background: rgba(0, 0, 0, 0.08);
  color: #1d1d1f;
}

.sidebar-body {
  flex: 1;
  overflow: auto;
  padding: 12px 14px 20px;
}

.version-era + .version-era {
  margin-top: 14px;
}

.version-era-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 8px 8px;
}

.version-era-name {
  font-size: 12px;
  font-weight: 700;
  color: #64748b;
  letter-spacing: 0.02em;
}

.version-era-count {
  font-size: 11px;
  color: #94a3b8;
  font-variant-numeric: tabular-nums;
}

.version-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.version-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  width: 100%;
  padding: 12px 14px;
  border: 1px solid rgba(0, 0, 0, 0.05);
  border-radius: 12px;
  background: rgba(255, 255, 255, 0.7);
  cursor: pointer;
  text-align: left;
  font-family: inherit;
  transition: border-color 0.15s ease, background 0.15s ease, transform 0.15s ease;
  animation: card-in 0.35s cubic-bezier(0.22, 1, 0.36, 1) both;
  animation-delay: var(--card-delay, 0ms);
}

@keyframes card-in {
  from {
    opacity: 0;
    transform: translateY(8px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.version-row:hover {
  border-color: color-mix(in srgb, var(--theme-color, #007bff) 35%, transparent);
  background: color-mix(in srgb, var(--theme-color, #007bff) 6%, white);
}

.version-row.active {
  border-color: color-mix(in srgb, var(--theme-color, #007bff) 55%, transparent);
  background: color-mix(in srgb, var(--theme-color, #007bff) 10%, white);
}

.version-row-main {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.version-row-label {
  font-size: 13.5px;
  font-weight: 700;
  color: #1d1d1f;
}

.version-row-meta {
  font-size: 11.5px;
  color: #94a3b8;
}

.version-row-tail {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}

.version-status {
  font-size: 10.5px;
  font-weight: 700;
  padding: 2px 7px;
  border-radius: 999px;
}

.status-latest {
  background: color-mix(in srgb, var(--theme-color, #007bff) 12%, transparent);
  color: var(--theme-color, #007bff);
}

.status-stable {
  background: #ecfdf5;
  color: #059669;
}

.status-beta {
  background: #fff7ed;
  color: #c2410c;
}

.version-row-check {
  color: var(--theme-color, #007bff);
  font-size: 16px;
}

@media (prefers-reduced-motion: reduce) {
  .version-row {
    animation: none;
  }
}

body.motion-reduced .version-row {
  animation: none;
}
</style>
