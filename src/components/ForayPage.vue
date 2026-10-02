<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useI18n } from "vue-i18n";

const { t } = useI18n();

const emit = defineEmits<{ (e: "leave"): void; (e: "switch-page", p: string): void }>();

const loading = ref(false);
const err = ref("");
const opened = ref(false);
const source = ref("");
const desc = ref("");
const packFormat = ref<number | null>(null);
const files = ref<Array<{ path: string; kind: string }>>([]);
const issues = ref<Array<{ level: string; source: string; path: string; message: string }>>([]);
const selected = ref("");
const previewUtf8 = ref("");
const previewPng = ref("");
const paintPng = ref("");
const brushR = ref(3);
const brushOpacity = ref(0.8);
const brushColor = ref("#3b82f6");
const eyedropper = ref(false);
const hsv = ref({ dh: 0, ds: 0, dv: 0 });
const exportMode = ref<"save_as" | "in_place">("save_as");
const confirmInPlace = ref(false);
const tier = ref(1);
const aiConfig = ref({
  base_url: "https://api.openai.com/v1",
  api_key: "",
  model: "gpt-4o-mini",
  default_tier: 1,
  system_prompt: "",
});
const aiPreview = ref<string[]>([]);
const aiReport = ref("");
const aiBusy = ref(false);
const aiError = ref("");

// dialogs
const showIssues = ref(false);
const showPaint = ref(false);
const showReport = ref(false);
const lightboxSrc = ref("");
const lightboxZoom = ref(1);
const treeQuery = ref("");
const openFolders = ref<Record<string, boolean>>({});

function onLeave() {
  emit("leave");
  emit("switch-page", "home");
}

async function openFromPath(path: string) {
  loading.value = true;
  err.value = "";
  try {
    const res = await invoke<any>("foray_open_pack", { path });
    opened.value = true;
    source.value = res.source;
    desc.value = res.description;
    packFormat.value = res.pack_format;
    await refreshRom();
    await refreshAi();
  } catch (e: any) {
    err.value = String(e);
    opened.value = false;
  } finally {
    loading.value = false;
  }
}

async function refreshRom() {
  const snap = await invoke<any>("foray_get_rom");
  desc.value = snap.description;
  packFormat.value = snap.pack_format ?? snap.min_format ?? null;
  files.value = snap.files.map(([path, kind]: [string, string]) => ({ path, kind }));
  issues.value = snap.issues ?? [];
}

async function pickFile() {
  try {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const p = await open({
      multiple: false,
      filters: [{ name: "zip", extensions: ["zip", "mcpack"] }],
    });
    if (typeof p === "string") await openFromPath(p);
  } catch (e: any) {
    err.value = String(e);
  }
}

async function selectPath(path: string) {
  selected.value = path;
  previewUtf8.value = "";
  previewPng.value = "";
  paintPng.value = "";
  try {
    const f = await invoke<any>("foray_read_file", { path });
    if (f.png_base64) {
      previewPng.value = `data:image/png;base64,${f.png_base64}`;
      paintPng.value = previewPng.value;
      await openPaint(path);
    } else {
      previewUtf8.value = f.utf8 ?? "";
    }
  } catch (e: any) {
    err.value = String(e);
  }
}

async function openPaint(path: string) {
  await invoke("foray_paint_open", { path });
  await refreshPaint();
  showPaint.value = true;
}

async function refreshPaint() {
  try {
    const b64 = await invoke<string>("foray_paint_preview");
    paintPng.value = `data:image/png;base64,${b64}`;
    if (lightboxSrc.value) lightboxSrc.value = paintPng.value;
  } catch {
    paintPng.value = previewPng.value;
  }
}

function colorToRgba(hex: string): [number, number, number, number] {
  const h = hex.replace("#", "");
  return [
    parseInt(h.slice(0, 2), 16),
    parseInt(h.slice(2, 4), 16),
    parseInt(h.slice(4, 6), 16),
    255,
  ];
}

async function onPaintClick(ev: MouseEvent) {
  const img = ev.currentTarget as HTMLImageElement;
  const rect = img.getBoundingClientRect();
  const scaleX = img.naturalWidth / rect.width;
  const scaleY = img.naturalHeight / rect.height;
  const x = Math.floor((ev.clientX - rect.left) * scaleX);
  const y = Math.floor((ev.clientY - rect.top) * scaleY);
  if (eyedropper.value || ev.shiftKey) {
    const c = document.createElement("canvas");
    c.width = img.naturalWidth;
    c.height = img.naturalHeight;
    const ctx = c.getContext("2d");
    if (!ctx) return;
    ctx.drawImage(img, 0, 0);
    const d = ctx.getImageData(x, y, 1, 1).data;
    brushColor.value =
      "#" + [d[0], d[1], d[2]].map((v) => v.toString(16).padStart(2, "0")).join("");
    eyedropper.value = false;
    return;
  }
  await invoke("foray_paint_brush", {
    stamp: {
      x,
      y,
      radius: brushR.value,
      opacity: brushOpacity.value,
      color: colorToRgba(brushColor.value),
    },
  });
  await refreshPaint();
}

async function applyHsv() {
  await invoke("foray_paint_hsv", { adj: hsv.value });
  await refreshPaint();
}

async function undoPaint() {
  await invoke("foray_paint_undo");
  await refreshPaint();
}

async function commitPaint() {
  await invoke("foray_paint_commit");
  await refreshRom();
  await selectPath(selected.value);
}

function openLightbox() {
  if (!paintPng.value && !previewPng.value) return;
  lightboxSrc.value = (paintPng.value || previewPng.value) as string;
  lightboxZoom.value = 2;
}

function closeLightbox() {
  lightboxSrc.value = "";
  lightboxZoom.value = 1;
}

async function doExport() {
  err.value = "";
  try {
    if (exportMode.value === "in_place" && !confirmInPlace.value) {
      err.value = t('foray.export.confirmRequired');
      return;
    }
    const res = await invoke<any>("foray_export", {
      mode: exportMode.value === "in_place" ? "in_place" : "save_as",
      destDir: null,
    });
    alert(t('foray.export.done', { path: res.path }));
    confirmInPlace.value = false;
  } catch (e: any) {
    err.value = String(e);
  }
}

async function refreshAi() {
  try {
    aiConfig.value = await invoke("foray_ai_config_get");
  } catch {
    /* ignore */
  }
}

async function saveAiConfig() {
  await invoke("foray_ai_config_set", { config: aiConfig.value });
}

function restoreDefaultPrompt() {
  aiConfig.value.system_prompt = "";
}

async function copyReport() {
  try {
    await navigator.clipboard.writeText(aiReport.value);
    err.value = t('foray.report.copied');
    setTimeout(() => {
      if (err.value === t('foray.report.copied')) err.value = "";
    }, 1500);
  } catch (e) {
    console.warn(e);
  }
}

async function prepareAi() {
  aiError.value = "";
  try {
    await saveAiConfig();
    const r = await invoke<any>("foray_ai_prepare", {
      tier: tier.value,
      selected: selected.value ? [selected.value] : [],
    });
    aiPreview.value = r.preview;
  } catch (e: any) {
    aiError.value = String(e);
  }
}

async function runAi() {
  aiBusy.value = true;
  aiError.value = "";
  aiReport.value = "";
  try {
    await prepareAi();
    if (tier.value === 0) {
      aiError.value = t('foray.ai.tier0Notice');
      return;
    }
    aiReport.value = await invoke<string>("foray_ai_analyze", {
      tier: tier.value,
      selected: selected.value ? [selected.value] : [],
    });
    showReport.value = true;
  } catch (e: any) {
    aiError.value = String(e);
  } finally {
    aiBusy.value = false;
  }
}

const treeRoots = computed(() => {
  const q = treeQuery.value.trim().toLowerCase();
  const list = files.value.map((f) => {
    const parts = f.path.split("/");
    return { ...f, top: parts[1] || parts[0] || "" };
  });
  const groups = new Map<string, typeof list>();
  for (const f of list) {
    if (q && !f.path.toLowerCase().includes(q)) continue;
    const g = groups.get(f.top) ?? [];
    g.push(f);
    groups.set(f.top, g);
  }
  return [...groups.entries()].map(([top, items]) => ({
    top,
    count: items.length,
    items: (openFolders.value[top] || q ? items : items.slice(0, 12)),
    expanded: !!openFolders.value[top],
  }));
});

onMounted(() => {
  const q = new URLSearchParams(window.location.search);
  const p = q.get("path") || localStorage.getItem("foray.pendingPath");
  if (p) {
    localStorage.removeItem("foray.pendingPath");
    openFromPath(p);
  }
});
</script>

<template>
  <div class="foray-page">
    <header class="header-section">
      <button class="ghost-btn back-btn" type="button" @click="onLeave">
        <i class="ri-arrow-left-line" aria-hidden="true"></i>
        {{ t('common.back') }}
      </button>
      <div class="title-group">
        <h1 class="page-title">{{ t('foray.header.title') }}</h1>
        <p class="page-subtitle">{{ source || t('foray.header.subtitle') }}</p>
      </div>
      <div class="header-actions">
        <button class="ghost-btn" type="button" :disabled="!opened || !issues.length" @click="showIssues = true">
          {{ t('foray.header.issues', { count: issues.length }) }}
        </button>
        <button class="ghost-btn" type="button" :disabled="!aiReport" @click="showReport = true">
          {{ t('foray.report.title') }}
        </button>
        <button class="start-conversion-button sm" type="button" :disabled="loading" @click="pickFile">
          {{ loading ? t('foray.header.opening') : t('foray.header.openZip') }}
        </button>
      </div>
    </header>

    <p v-if="err" class="page-error">{{ err }}</p>

    <div v-if="!opened" class="empty-state card">
      <i class="ri-folder-zip-line empty-icon" aria-hidden="true"></i>
      <h2>{{ t('foray.empty.title') }}</h2>
      <p>{{ t('foray.empty.desc') }}</p>
    </div>

    <div v-else class="foray-grid">
      <section class="card pane tree-pane">
        <h2 class="card-title">{{ t('foray.tree.title') }}</h2>
        <input v-model="treeQuery" class="tree-filter" type="search" :placeholder="t('foray.tree.filterPlaceholder')" />
        <div class="tree">
          <div v-for="g in treeRoots" :key="g.top" class="tree-group">
            <button class="tree-group-btn" type="button" @click="openFolders[g.top] = !openFolders[g.top]">
              <i :class="g.expanded ? 'ri-arrow-down-s-line' : 'ri-arrow-right-s-line'" />
              <b>{{ g.top }}</b>
              <span class="kind-tag">{{ g.count }}</span>
            </button>
            <ul v-if="g.expanded || treeQuery">
              <li v-for="f in g.items" :key="f.path">
                <button
                  class="tree-link"
                  :class="{ active: f.path === selected }"
                  type="button"
                  :title="f.path"
                  @click="selectPath(f.path)"
                >
                  {{ f.path.split("/").slice(2).join("/") || f.path }}
                </button>
                <span class="kind-tag">{{ f.kind }}</span>
              </li>
            </ul>
          </div>
        </div>
      </section>

      <section class="card pane main-pane">
        <h2 class="card-title">{{ t('foray.overview.title') }}</h2>
        <p class="overview-line">
          <b>{{ desc || t('foray.overview.noDescription') }}</b>
          <span class="pill">{{ t('foray.overview.format', { version: packFormat ?? "?" }) }}</span>
          <span class="pill">{{ t('foray.overview.files', { count: files.length }) }}</span>
          <button class="ghost-btn sm" type="button" @click="showIssues = true">
            {{ t('foray.overview.viewIssues', { count: issues.length }) }}
          </button>
        </p>

        <h2 class="card-title">{{ t('foray.preview.title') }}</h2>
        <p class="muted small">{{ selected || t('foray.preview.selectHint') }}</p>
        <pre v-if="previewUtf8" class="code-block">{{ previewUtf8.slice(0, 2500) }}</pre>
        <div v-if="paintPng || previewPng" class="preview-wrap">
          <img
            class="paint-img"
            :src="paintPng || previewPng"
            alt="preview"
            @click="openLightbox"
          />
          <div class="toolbar">
            <button class="ghost-btn" type="button" @click="openLightbox">{{ t('foray.preview.zoomIn') }}</button>
            <button class="start-conversion-button sm" type="button" @click="openPaint(selected)">
              {{ t('foray.paint.title') }}
            </button>
          </div>
        </div>

        <h2 class="card-title">{{ t('foray.export.title') }}</h2>
        <div class="toolbar">
          <label class="check">
            <input v-model="exportMode" type="radio" value="save_as" />
            {{ t('foray.export.saveAs') }}
          </label>
          <label class="check">
            <input v-model="exportMode" type="radio" value="in_place" />
            {{ t('foray.export.inPlace') }}
          </label>
          <label v-if="exportMode === 'in_place'" class="check warn">
            <input v-model="confirmInPlace" type="checkbox" />
            {{ t('foray.export.confirmBak') }}
          </label>
          <button class="start-conversion-button sm" type="button" @click="doExport">{{ t('common.export') }}</button>
        </div>
      </section>

      <section class="card pane ai-pane">
        <h2 class="card-title">{{ t('foray.ai.title') }}</h2>
        <label class="field">
          <span>Base URL</span>
          <input v-model="aiConfig.base_url" type="text" />
        </label>
        <label class="field">
          <span>API Key</span>
          <input v-model="aiConfig.api_key" type="password" autocomplete="off" />
        </label>
        <label class="field">
          <span>{{ t('foray.ai.model') }}</span>
          <input v-model="aiConfig.model" type="text" />
        </label>
        <label class="field">
          <span>{{ t('foray.ai.tier') }}</span>
          <select v-model.number="tier">
            <option :value="0">{{ t('foray.ai.tier0') }}</option>
            <option :value="1">{{ t('foray.ai.tier1') }}</option>
            <option :value="2">{{ t('foray.ai.tier2') }}</option>
            <option :value="3">{{ t('foray.ai.tier3') }}</option>
            <option :value="4">{{ t('foray.ai.tier4') }}</option>
            <option :value="5">{{ t('foray.ai.tier5') }}</option>
          </select>
        </label>
        <label class="field">
          <span>{{ t('foray.ai.prompt') }}</span>
          <textarea v-model="aiConfig.system_prompt" rows="3" :placeholder="t('foray.ai.promptPlaceholder')"></textarea>
        </label>
        <div class="toolbar">
          <button class="ghost-btn" type="button" @click="restoreDefaultPrompt">{{ t('foray.ai.restoreDefault') }}</button>
          <button class="ghost-btn" type="button" @click="prepareAi">{{ t('foray.ai.prepare') }}</button>
          <button class="start-conversion-button sm" type="button" :disabled="aiBusy" @click="runAi">
            {{ aiBusy ? t('foray.ai.analyzing') : t('foray.ai.analyze') }}
          </button>
        </div>
        <p class="muted small">{{ t('foray.ai.privacyNotice') }}</p>
        <ul v-if="aiPreview.length" class="issue-list compact">
          <li v-for="(p, i) in aiPreview" :key="i">{{ p }}</li>
        </ul>
        <p v-if="aiError" class="page-error">{{ aiError }}</p>
        <button v-if="aiReport" class="ghost-btn" type="button" @click="showReport = true">
          {{ t('foray.ai.viewReport') }}
        </button>

        <h2 class="card-title">IFASO</h2>
        <p class="muted small">{{ t('foray.ifaso.note') }}</p>
      </section>
    </div>

    <!-- 问题弹窗 -->
    <div v-if="showIssues" class="modal-mask" @click.self="showIssues = false">
      <div class="modal panel-issues">
        <header class="modal-head">
          <h3>{{ t('foray.issues.title', { count: issues.length }) }}</h3>
          <button class="ghost-btn sm" type="button" @click="showIssues = false">{{ t('common.close') }}</button>
        </header>
        <div class="modal-body">
          <ul class="issue-list">
            <li v-for="(i, idx) in issues" :key="idx" :class="i.level">
              <span class="issue-src">{{ i.source }}</span>
              <span class="issue-path">{{ i.path }}</span>
              <span class="issue-msg">{{ i.message }}</span>
            </li>
            <li v-if="!issues.length" class="muted">{{ t('foray.issues.empty') }}</li>
          </ul>
        </div>
      </div>
    </div>

    <!-- 编辑弹窗 -->
    <div v-if="showPaint" class="modal-mask" @click.self="showPaint = false">
      <div class="modal panel-paint">
        <header class="modal-head">
          <h3>{{ t('foray.paint.title') }} · {{ selected.split("/").pop() }}</h3>
          <button class="ghost-btn sm" type="button" @click="showPaint = false">{{ t('common.close') }}</button>
        </header>
        <div class="modal-body paint-body">
          <img
            v-if="paintPng || previewPng"
            class="paint-canvas"
            :src="paintPng || previewPng"
            alt="paint"
            @click="onPaintClick"
          />
          <div class="toolbar">
            <label>{{ t('foray.paint.brush') }} <input v-model.number="brushR" type="number" min="1" max="32" /></label>
            <label>{{ t('foray.paint.opacity') }} <input v-model.number="brushOpacity" type="number" min="0" max="1" step="0.05" /></label>
            <label>{{ t('foray.paint.color') }} <input v-model="brushColor" type="color" /></label>
            <label class="check"><input v-model="eyedropper" type="checkbox" /> {{ t('foray.paint.eyedropper') }}</label>
            <button class="ghost-btn" type="button" @click="undoPaint">{{ t('foray.paint.undo') }}</button>
            <button class="start-conversion-button sm" type="button" @click="commitPaint">{{ t('foray.paint.apply') }}</button>
          </div>
          <div class="toolbar">
            <label>H <input v-model.number="hsv.dh" type="number" min="-180" max="180" /></label>
            <label>S <input v-model.number="hsv.ds" type="number" min="-100" max="100" /></label>
            <label>V <input v-model.number="hsv.dv" type="number" min="-100" max="100" /></label>
            <button class="ghost-btn" type="button" @click="applyHsv">{{ t('foray.paint.applyHsv') }}</button>
            <button class="ghost-btn" type="button" @click="openLightbox">{{ t('foray.paint.fullscreen') }}</button>
          </div>
        </div>
      </div>
    </div>

    <!-- AI 报告弹窗 -->
    <div v-if="showReport" class="modal-mask" @click.self="showReport = false">
      <div class="modal panel-report">
        <header class="modal-head">
          <h3>{{ t('foray.report.title') }}</h3>
          <div>
            <button class="ghost-btn sm" type="button" @click="copyReport">{{ t('common.copy') }}</button>
            <button class="ghost-btn sm" type="button" @click="showReport = false">{{ t('common.close') }}</button>
          </div>
        </header>
        <div class="modal-body">
          <pre class="code-block report-full">{{ aiReport }}</pre>
        </div>
      </div>
    </div>

    <!-- 图片灯箱 -->
    <div v-if="lightboxSrc" class="modal-mask lightbox" @click.self="closeLightbox">
      <div class="lightbox-toolbar">
        <button class="ghost-btn" type="button" @click="lightboxZoom = Math.max(0.5, lightboxZoom - 0.25)">−</button>
        <span class="muted">{{ t('foray.lightbox.zoomPercent', { percent: (lightboxZoom * 100).toFixed(0) }) }}</span>
        <button class="ghost-btn" type="button" @click="lightboxZoom = Math.min(8, lightboxZoom + 0.25)">+</button>
        <button class="ghost-btn" type="button" @click="lightboxZoom = 2">2×</button>
        <button class="ghost-btn" type="button" @click="closeLightbox">{{ t('common.close') }}</button>
      </div>
      <div class="lightbox-scroll">
        <img
          class="lightbox-img"
          :src="lightboxSrc"
          alt="preview-large"
          :style="{ width: `${lightboxZoom * 100}%`, maxWidth: `${lightboxZoom * 100}%` }"
        />
      </div>
    </div>
  </div>
</template>

<style scoped>
.foray-page {
  width: 100%;
  height: 100%;
  min-height: 0;
  color: #1d1d1f;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", "PingFang SC",
    "Microsoft YaHei", sans-serif;
}
.header-section {
  z-index: 5;
  padding: 28px 40px 10px;
  display: flex;
  align-items: center;
  gap: 16px;
  flex-shrink: 0;
}
.back-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
}
.title-group {
  display: flex;
  flex-direction: column;
  min-width: 0;
  flex: 1;
}
.page-title {
  font-size: 26px;
  font-weight: 800;
  letter-spacing: -0.6px;
  margin: 0;
  color: #1d1d1f;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.page-subtitle {
  margin: 4px 0 0;
  color: #86868b;
  font-size: 13px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.header-actions {
  display: flex;
  gap: 10px;
  flex-shrink: 0;
}
.start-conversion-button {
  border: none;
  background: var(--theme-color, #007bff);
  color: #fff;
  font-weight: 700;
  font-size: 14px;
  padding: 12px 22px;
  border-radius: var(--ui-radius-btn, 12px);
  cursor: pointer;
  box-shadow: 0 8px 20px color-mix(in srgb, var(--theme-color, #007bff) 28%, transparent);
}
.start-conversion-button:hover:not(:disabled) {
  filter: brightness(1.05);
}
.start-conversion-button:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}
.start-conversion-button.sm {
  padding: 8px 14px;
  font-size: 13px;
}
.ghost-btn {
  border: 1px solid rgba(0, 0, 0, 0.1);
  background: rgba(255, 255, 255, 0.72);
  color: #1d1d1f;
  font-weight: 600;
  font-size: 13px;
  padding: 8px 12px;
  border-radius: var(--ui-radius-btn, 10px);
  cursor: pointer;
  font-family: inherit;
}
.ghost-btn.sm {
  padding: 4px 10px;
  font-size: 12px;
}
.ghost-btn:hover:not(:disabled) {
  background: #fff;
}
.ghost-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.page-error {
  margin: 0 40px 8px;
  color: #b91c1c;
  font-size: 13px;
  font-weight: 600;
}
.empty-state {
  margin: 12px 40px 28px;
  padding: 48px 28px;
  text-align: center;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 10px;
}
.empty-icon {
  font-size: 42px;
  color: var(--theme-color, #007bff);
}
.foray-grid {
  flex: 1;
  min-height: 0;
  padding: 12px 40px 28px;
  display: grid;
  grid-template-columns: minmax(220px, 0.9fr) minmax(0, 1.35fr) minmax(280px, 1fr);
  gap: 14px;
}
.pane {
  padding: 18px;
  display: flex;
  flex-direction: column;
  gap: 10px;
  min-height: 0;
  overflow: auto;
}
.card-title {
  font-size: 13px;
  font-weight: 700;
  color: #6b7280;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  margin: 6px 0 0;
}
.overview-line {
  margin: 0;
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
  font-size: 13px;
}
.pill {
  display: inline-flex;
  padding: 2px 10px;
  border-radius: 999px;
  font-size: 12px;
  font-weight: 600;
  color: color-mix(in srgb, var(--theme-color, #007bff) 80%, #000);
  background: color-mix(in srgb, var(--theme-color, #007bff) 12%, transparent);
}
.tree-filter {
  width: 100%;
  box-sizing: border-box;
  padding: 8px 10px;
  border-radius: var(--ui-radius-btn, 10px);
  border: 1px solid rgba(0, 0, 0, 0.08);
  background: rgba(255, 255, 255, 0.8);
  font-size: 12px;
}
.tree {
  overflow: auto;
  min-height: 0;
  font-size: 12px;
}
.tree-group {
  margin: 6px 0;
}
.tree-group-btn {
  width: 100%;
  display: flex;
  align-items: center;
  gap: 6px;
  border: none;
  background: rgba(0, 0, 0, 0.03);
  border-radius: 8px;
  padding: 6px 8px;
  cursor: pointer;
  color: #1d1d1f;
  font: inherit;
  font-size: 12px;
  font-weight: 700;
}
.tree ul {
  margin: 4px 0 0;
  padding: 0 0 0 10px;
  list-style: none;
}
.tree li {
  margin: 2px 0;
  display: flex;
  gap: 8px;
  word-break: break-all;
}
.tree-link {
  border: none;
  background: none;
  color: #1d1d1f;
  cursor: pointer;
  text-align: left;
  padding: 2px 0;
  font: inherit;
}
.tree-link:hover,
.tree-link.active {
  color: var(--theme-color, #007bff);
}
.tree-link.active {
  font-weight: 700;
}
.kind-tag {
  font-size: 10px;
  color: #9ca3af;
  flex-shrink: 0;
}
.issue-list {
  margin: 0;
  padding: 0;
  list-style: none;
  font-size: 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.issue-list li {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  align-items: baseline;
}
.issue-src {
  font-weight: 700;
  color: #6b7280;
}
.issue-path {
  font-family: ui-monospace, Consolas, monospace;
}
.issue-msg {
  color: #b91c1c;
}
.issue-list li.warn .issue-msg {
  color: #b45309;
}
.muted {
  color: #9ca3af;
}
.small {
  font-size: 12px;
  margin: 0;
}
.code-block {
  margin: 0;
  padding: 12px 14px;
  background: rgba(15, 23, 42, 0.04);
  border: 1px solid rgba(0, 0, 0, 0.06);
  border-radius: var(--ui-radius-card, 14px);
  font-size: 12px;
  line-height: 1.5;
  max-height: 200px;
  overflow: auto;
  white-space: pre-wrap;
  word-break: break-word;
}
.preview-wrap {
  display: flex;
  flex-direction: column;
  gap: 10px;
  align-items: flex-start;
}
.paint-img {
  max-width: 100%;
  max-height: 280px;
  width: auto;
  image-rendering: pixelated;
  cursor: zoom-in;
  border-radius: var(--ui-radius-card, 12px);
  border: 1px solid rgba(0, 0, 0, 0.08);
  background: #fff;
}
.toolbar {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  align-items: center;
  font-size: 12px;
}
.toolbar label {
  display: inline-flex;
  align-items: center;
  gap: 6px;
}
.toolbar input[type="number"] {
  width: 68px;
  padding: 6px 8px;
  border-radius: 8px;
  border: 1px solid rgba(0, 0, 0, 0.1);
  background: rgba(255, 255, 255, 0.8);
  font: inherit;
}
.toolbar input[type="color"] {
  width: 36px;
  height: 28px;
  border: 1px solid rgba(0, 0, 0, 0.1);
  border-radius: 8px;
  background: #fff;
}
.check {
  user-select: none;
}
.check.warn {
  color: #b45309;
  font-weight: 600;
}
.field {
  display: flex;
  flex-direction: column;
  gap: 6px;
  font-size: 12px;
  color: #6b7280;
  font-weight: 600;
}
.field input,
.field select,
.field textarea {
  width: 100%;
  box-sizing: border-box;
  padding: 10px 12px;
  border-radius: var(--ui-radius-btn, 10px);
  border: 1px solid rgba(0, 0, 0, 0.08);
  background: rgba(255, 255, 255, 0.78);
  color: #1d1d1f;
  font-size: 13px;
  font-family: inherit;
  resize: vertical;
}

/* dialogs */
.modal-mask {
  position: fixed;
  inset: 0;
  z-index: 9000;
  background: rgba(15, 23, 42, 0.45);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px;
}
.modal {
  background: rgba(255, 255, 255, 0.96);
  border-radius: 16px;
  border: 1px solid rgba(0, 0, 0, 0.08);
  box-shadow: 0 24px 60px rgba(0, 0, 0, 0.28);
  display: flex;
  flex-direction: column;
  max-height: min(86vh, 900px);
  width: min(920px, 96vw);
  overflow: hidden;
}
.panel-paint {
  width: min(1100px, 96vw);
}
.panel-report {
  width: min(980px, 96vw);
}
.modal-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 16px 18px;
  border-bottom: 1px solid rgba(0, 0, 0, 0.06);
}
.modal-head h3 {
  margin: 0;
  font-size: 16px;
  font-weight: 800;
  color: #1d1d1f;
}
.modal-body {
  padding: 16px 18px 20px;
  overflow: auto;
}
.paint-body {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.paint-canvas {
  max-width: 100%;
  max-height: 58vh;
  width: auto;
  image-rendering: pixelated;
  cursor: crosshair;
  border-radius: 12px;
  border: 1px solid rgba(0, 0, 0, 0.08);
  background: #fff;
  align-self: center;
}
.report-full {
  max-height: none;
  min-height: 280px;
  font-size: 13px;
  line-height: 1.55;
}
.lightbox {
  flex-direction: column;
  gap: 12px;
}
.lightbox-toolbar {
  display: flex;
  gap: 8px;
  align-items: center;
  background: rgba(255, 255, 255, 0.9);
  border-radius: 999px;
  padding: 6px 10px;
}
.lightbox-scroll {
  width: min(96vw, 1400px);
  height: min(80vh, 900px);
  overflow: auto;
  background: rgba(255, 255, 255, 0.92);
  border-radius: 12px;
}
.lightbox-img {
  image-rendering: pixelated;
  display: block;
  margin: 0 auto;
  transform-origin: top left;
}

@media (max-width: 1100px) {
  .foray-grid {
    grid-template-columns: 1fr;
    overflow: auto;
    padding: 8px 20px 20px;
  }
  .header-section {
    padding: 20px 20px 8px;
    flex-wrap: wrap;
  }
}
</style>
