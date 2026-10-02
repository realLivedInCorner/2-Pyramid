<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { useNotification } from "../../composables/useNotification";

const { t } = useI18n();
const { notify } = useNotification();

interface LayerInfo {
  directory: string;
  formats: number[];
  exists: boolean;
  fileCount: number;
  overrideCount: number;
  overridePreview: string[];
}

export interface PackAnalysis {
  source: string;
  shapes: string[];
  packFormat: number | null;
  supportedFormats: string | null;
  rootPrefix: string;
  multiRoots: string[];
  layers: LayerInfo[];
  foldingDirs: LayerInfo[];
  totalFiles: number;
  warnings: string[];
}

const visible = defineModel<boolean>({ required: true });
const props = defineProps<{ analysis: PackAnalysis | null }>();

const json = computed(() =>
  props.analysis ? JSON.stringify(props.analysis, null, 2) : ""
);

async function copy() {
  if (!json.value) return;
  try {
    await navigator.clipboard.writeText(json.value);
    await notify({ title: t("common.success"), body: t("settings.devMode.analysisCopied"), type: "success", source: "system" });
  } catch (e) {
    await notify({ title: t("common.error"), body: String(e), type: "error", source: "system" });
  }
}
</script>

<template>
  <transition name="dialog-pop">
    <div v-if="visible" class="dialog-overlay" @click="visible = false">
      <div class="dialog-content analysis-dialog" @click.stop>
        <div class="dialog-header">
          <h3>{{ t("settings.devMode.analyzePackTitle") }}</h3>
          <button class="dialog-close" @click="visible = false" :aria-label="t('common.close')">×</button>
        </div>
        <div class="dialog-body" v-if="analysis">
          <div class="shape-row">
            <span
              v-for="s in analysis.shapes"
              :key="s"
              class="shape-badge"
              :class="{ warn: s !== 'singleVersion' }"
            >{{ t(`settings.devMode.packShapes.${s}`) }}</span>
          </div>

          <div class="meta-grid">
            <div><b>{{ t("settings.devMode.analysisFormat") }}</b><span>{{ analysis.packFormat ?? "—" }}</span></div>
            <div><b>{{ t("settings.devMode.analysisSupported") }}</b><span>{{ analysis.supportedFormats ?? "—" }}</span></div>
            <div><b>{{ t("settings.devMode.analysisRoot") }}</b><span>{{ analysis.rootPrefix || "/" }}</span></div>
            <div><b>{{ t("settings.devMode.analysisFiles") }}</b><span>{{ analysis.totalFiles }}</span></div>
          </div>

          <div v-if="analysis.layers.length > 1" class="section">
            <div class="section-title">{{ t("settings.devMode.analysisLayers") }}</div>
            <div v-for="l in analysis.layers" :key="l.directory || 'base'" class="layer-row">
              <span class="layer-name">{{ l.directory || t("settings.devMode.analysisBaseLayer") }}</span>
              <span class="layer-fmt">{{ l.formats.length ? l.formats.join(",") : "—" }}</span>
              <span class="layer-stat">{{ t("settings.devMode.analysisFilesShort", { n: l.fileCount }) }}</span>
              <span class="layer-stat" :class="{ hit: l.overrideCount > 0 }">
                {{ t("settings.devMode.analysisOverrides", { n: l.overrideCount }) }}
              </span>
            </div>
          </div>

          <div v-if="analysis.foldingDirs.length" class="section">
            <div class="section-title">{{ t("settings.devMode.analysisFolding") }}</div>
            <div v-for="d in analysis.foldingDirs" :key="d.directory" class="layer-row">
              <span class="layer-name">{{ d.directory }}</span>
              <span class="layer-stat">{{ t("settings.devMode.analysisFilesShort", { n: d.fileCount }) }}</span>
            </div>
          </div>

          <div v-if="analysis.multiRoots.length" class="section">
            <div class="section-title">{{ t("settings.devMode.analysisMultiRoots") }}</div>
            <div v-for="r in analysis.multiRoots" :key="r" class="layer-row">
              <span class="layer-name">{{ r || "/" }}</span>
            </div>
          </div>

          <div v-if="analysis.warnings.length" class="section warnings">
            <div class="section-title">{{ t("settings.devMode.analysisWarnings") }}</div>
            <div v-for="(w, i) in analysis.warnings" :key="i" class="warn-row">· {{ w }}</div>
          </div>

          <details class="raw">
            <summary>{{ t("settings.devMode.analysisRaw") }}</summary>
            <pre>{{ json }}</pre>
          </details>
        </div>
        <div class="dialog-footer">
          <button class="btn-text secondary" @click="copy">{{ t("common.copy") }}</button>
          <button class="btn-text" @click="visible = false">{{ t("common.close") }}</button>
        </div>
      </div>
    </div>
  </transition>
</template>

<style scoped>
.analysis-dialog { width: min(760px, 94vw); }
.dialog-body { max-height: 62vh; overflow-y: auto; text-align: left; }
.shape-row { display: flex; gap: 6px; flex-wrap: wrap; margin-bottom: 10px; }
.shape-badge {
  padding: 3px 10px; border-radius: 999px; font-size: 11.5px; font-weight: 700;
  background: rgba(0, 123, 255, 0.12); color: #007bff;
}
.shape-badge.warn { background: rgba(217, 119, 6, 0.14); color: #b45309; }
.meta-grid {
  display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
  gap: 8px; margin-bottom: 12px;
}
.meta-grid > div { display: flex; flex-direction: column; gap: 2px; }
.meta-grid b { font-size: 11px; color: #86868b; font-weight: 600; }
.meta-grid span { font-size: 13px; color: #1a1a2e; font-family: ui-monospace, Consolas, monospace; }
.section { margin: 12px 0; }
.section-title { font-size: 12px; font-weight: 700; color: #6b7280; margin-bottom: 6px; }
.layer-row {
  display: flex; gap: 10px; align-items: center; padding: 5px 8px;
  border-radius: 8px; background: rgba(0, 0, 0, 0.03); margin-bottom: 4px; font-size: 12px;
}
.layer-name { flex: 1; min-width: 0; word-break: break-all; color: #1a1a2e; }
.layer-fmt { font-family: ui-monospace, Consolas, monospace; color: #007bff; }
.layer-stat { color: #6b7280; white-space: nowrap; }
.layer-stat.hit { color: #b45309; font-weight: 700; }
.warnings .warn-row { font-size: 12px; color: #b45309; line-height: 1.6; }
.raw { margin-top: 10px; }
.raw summary { cursor: pointer; font-size: 12px; color: #6b7280; }
.raw pre {
  max-height: 220px; overflow: auto; margin-top: 6px; padding: 10px;
  background: #0f172a; color: #e2e8f0; border-radius: 10px; font-size: 11px;
}
</style>
