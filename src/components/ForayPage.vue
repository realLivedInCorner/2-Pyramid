<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

const emit = defineEmits<{ (e: "leave"): void; (e: "switch-page", p: string): void }>();

const lightboxSrc = ref("");
const issuesOpen = ref(false);
const treeQuery = ref("");
const openFolders = ref<Record<string, boolean>>({});

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
const brushR = ref(2);
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
const aiText = ref("");
const aiReport = ref("");
const aiBusy = ref(false);
const aiError = ref("");

function onPreviewClick(ev: MouseEvent) {
  // Shift / 吸管：仍走笔刷；单击：灯箱看大图
  if (eyedropper.value || ev.shiftKey) {
    void onPaintClick(ev);
    return;
  }
  const img = ev.currentTarget as HTMLImageElement;
  lightboxSrc.value = (paintPng.value || previewPng.value) as string;
  void img;
}

function closeLightbox() {
  lightboxSrc.value = "";
}

function onLeave() {
  emit("leave");
  emit("switch-page", "home");
}

const treeRoots = computed(() => {
  const q = treeQuery.value.trim().toLowerCase();
  const list = files.value.map((f) => {
    const parts = f.path.split("/");
    return { ...f, top: parts[1] || parts[0] || "", segs: parts };
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
    items: (openFolders.value[top] ? items : items.slice(0, 8)),
    expanded: !!openFolders.value[top],
  }));
});

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
}

async function refreshPaint() {
  try {
    const b64 = await invoke<string>("foray_paint_preview");
    paintPng.value = `data:image/png;base64,${b64}`;
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

async function doExport() {
  err.value = "";
  try {
    if (exportMode.value === "in_place" && !confirmInPlace.value) {
      err.value = "原地覆盖需勾选确认，并会写 .bak 备份";
      return;
    }
    const res = await invoke<any>("foray_export", {
      mode: exportMode.value === "in_place" ? "in_place" : "save_as",
      destDir: null,
    });
    alert("已导出：\n" + res.path);
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
    err.value = "报告已复制";
    setTimeout(() => {
      if (err.value === "报告已复制") err.value = "";
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
    aiText.value = r.payload_text;
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
      aiError.value = "档位 0 不会调用外部 API";
      return;
    }
    aiReport.value = await invoke<string>("foray_ai_analyze", {
      tier: tier.value,
      selected: selected.value ? [selected.value] : [],
    });
  } catch (e: any) {
    aiError.value = String(e);
  } finally {
    aiBusy.value = false;
  }
}

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
        返回
      </button>
      <div class="title-group">
        <h1 class="page-title">Foray 工作台</h1>
        <p class="page-subtitle">{{ source || "打开资源包开始分析" }}</p>
      </div>
      <div class="header-actions">
        <button class="start-conversion-button" type="button" :disabled="loading" @click="pickFile">
          {{ loading ? "解析中…" : "打开 zip" }}
        </button>
      </div>
    </header>

    <p v-if="err" class="page-error">{{ err }}</p>

    <div v-if="!opened" class="empty-state card">
      <i class="ri-folder-zip-line empty-icon" aria-hidden="true"></i>
      <h2>Editor Mode 已启用</h2>
      <p>从主页拖入 zip，或点击右上角「打开 zip」进入分析。普通转换在 EM 下已停用。</p>
    </div>

    <div v-else class="foray-grid">
      <section class="card pane tree-pane">
        <h2 class="card-title">ROM 树</h2>
        <input v-model="treeQuery" class="tree-filter" type="search" placeholder="过滤路径…" />
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
              <li v-if="!g.expanded && g.count > g.items.length" class="muted small">
                … 还有 {{ g.count - g.items.length }} 项，点击文件夹展开
              </li>
            </ul>
          </div>
        </div>
      </section>

      <section class="card pane main-pane">
        <h2 class="card-title">概览</h2>
        <p class="overview-line">
          <b>{{ desc || "（无描述）" }}</b>
          <span class="pill">format {{ packFormat ?? "?" }}</span>
          <span class="pill">{{ files.length }} files</span>
        </p>

        <h2 class="card-title row-title">
          <span>探针 / 问题</span>
          <button class="ghost-btn sm" type="button" @click="issuesOpen = !issuesOpen">
            {{ issuesOpen ? "收起" : "展开" }}（{{ issues.length }}）
          </button>
        </h2>
        <ul v-if="issuesOpen" class="issue-list scroll-box">
          <li v-for="(i, idx) in issues" :key="idx" :class="i.level">
            <span class="issue-src">{{ i.source }}</span>
            <span class="issue-path">{{ i.path }}</span>
            <span class="issue-msg">{{ i.message }}</span>
          </li>
          <li v-if="!issues.length" class="muted">暂无 issue</li>
        </ul>
        <p v-else class="muted small">{{ issues.length }} 条问题（点击展开）</p>

        <h2 class="card-title">预览</h2>
        <p class="muted small">{{ selected || "选择左侧文件" }}</p>
        <p class="muted small">单击图片可放大预览；Shift+点击进入涂抹。</p>
        <pre v-if="previewUtf8" class="code-block">{{ previewUtf8.slice(0, 4000) }}</pre>
        <img
          v-if="paintPng || previewPng"
          class="paint-img"
          :src="paintPng || previewPng"
          alt="preview"
          @click="onPreviewClick"
        />

        <template v-if="paintPng">
          <h2 class="card-title">轻量编辑</h2>
          <div class="toolbar">
            <label>笔刷 <input v-model.number="brushR" type="number" min="1" max="32" /></label>
            <label>透明 <input v-model.number="brushOpacity" type="number" min="0" max="1" step="0.05" /></label>
            <label>颜色 <input v-model="brushColor" type="color" /></label>
            <label class="check"><input v-model="eyedropper" type="checkbox" /> 吸管</label>
            <button class="ghost-btn" type="button" @click="undoPaint">撤销</button>
            <button class="start-conversion-button sm" type="button" @click="commitPaint">应用</button>
          </div>
          <div class="toolbar">
            <label>H <input v-model.number="hsv.dh" type="number" min="-180" max="180" /></label>
            <label>S <input v-model.number="hsv.ds" type="number" min="-100" max="100" /></label>
            <label>V <input v-model.number="hsv.dv" type="number" min="-100" max="100" /></label>
            <button class="ghost-btn" type="button" @click="applyHsv">应用 HSV</button>
          </div>
        
  <div v-if="lightboxSrc" class="lightbox-mask" @click.self="closeLightbox">
    <button class="ghost-btn lightbox-close" type="button" @click="closeLightbox">关闭</button>
    <img class="lightbox-img" :src="lightboxSrc" alt="preview-large" />
    <div class="lightbox-bar">
      <button class="ghost-btn" type="button" @click="closeLightbox">返回编辑</button>
    </div>
  </div>
</template>

        <h2 class="card-title">导出</h2>
        <div class="toolbar">
          <label class="check">
            <input v-model="exportMode" type="radio" value="save_as" />
            另存副本（默认）
          </label>
          <label class="check">
            <input v-model="exportMode" type="radio" value="in_place" />
            原地覆盖
          </label>
          <label v-if="exportMode === 'in_place'" class="check warn">
            <input v-model="confirmInPlace" type="checkbox" />
            确认覆盖（写 .bak）
          </label>
          <button class="start-conversion-button sm" type="button" @click="doExport">导出</button>
        </div>
      </section>

      <section class="card pane ai-pane">
        <h2 class="card-title">AI 分析（OpenAI 兼容）</h2>
        <label class="field">
          <span>Base URL</span>
          <input v-model="aiConfig.base_url" type="text" />
        </label>
        <label class="field">
          <span>API Key</span>
          <input v-model="aiConfig.api_key" type="password" autocomplete="off" />
        </label>
        <label class="field">
          <span>模型</span>
          <input v-model="aiConfig.model" type="text" />
        </label>
        <label class="field">
          <span>档位</span>
          <select v-model.number="tier">
            <option :value="0">0 · 无</option>
            <option :value="1">1 · 目录树</option>
            <option :value="2">2 · +mcmeta</option>
            <option :value="3">3 · +JSON（选中）</option>
            <option :value="4">4 · +着色器（选中）</option>
            <option :value="5">5 · +贴图概括</option>
          </select>
        </label>
        <label class="field">
          <span>提示词</span>
          <textarea v-model="aiConfig.system_prompt" rows="4" placeholder="留空使用内置默认"></textarea>
        </label>
        <div class="toolbar">
          <button class="ghost-btn" type="button" @click="restoreDefaultPrompt">还原默认提示词</button>
          <button class="ghost-btn" type="button" :disabled="!aiReport" @click="copyReport">复制报告</button>
        </div>
        <p class="muted small">
          ≥3 仅发送当前选中文件副本；贴图只发概括不发像素。外部 API 与作者无关。
        </p>
        <div class="toolbar">
          <button class="ghost-btn" type="button" @click="prepareAi">将发送预览</button>
          <button class="start-conversion-button sm" type="button" :disabled="aiBusy" @click="runAi">
            {{ aiBusy ? "分析中…" : "开始分析" }}
          </button>
        </div>
        <ul v-if="aiPreview.length" class="issue-list compact">
          <li v-for="(p, i) in aiPreview" :key="i">{{ p }}</li>
        </ul>
        <p v-if="aiError" class="page-error">{{ aiError }}</p>
        <pre v-if="aiReport" class="code-block ai-report">{{ aiReport }}</pre>

        <h2 class="card-title">IFASO</h2>
        <p class="muted small">互链入口占位（2.5.0 不实现互通）。跨软件按对方 License。</p>
      </section>
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
  position: relative;
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
  transition: transform 0.15s ease, filter 0.15s ease;
  font-family: inherit;
}
.start-conversion-button:hover:not(:disabled) {
  filter: brightness(1.05);
}
.start-conversion-button:active:not(:disabled) {
  transform: scale(0.98);
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
.ghost-btn:hover:not(:disabled) {
  background: rgba(255, 255, 255, 0.95);
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
  opacity: 0.85;
}
.empty-state h2 {
  margin: 0;
  font-size: 18px;
  font-weight: 800;
  color: #1d1d1f;
}
.empty-state p {
  margin: 0;
  color: #6b7280;
  max-width: 420px;
  font-size: 13px;
}

.foray-grid {
  flex: 1;
  min-height: 0;
  z-index: 2;
  padding: 12px 40px 28px;
  display: grid;
  grid-template-columns: minmax(220px, 0.9fr) minmax(0, 1.4fr) minmax(260px, 1fr);
  gap: 14px;
}

.pane {
  padding: 18px 18px 22px;
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
  flex-shrink: 0;
}

.overview-line {
  margin: 0;
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  color: #374151;
}

.pill {
  display: inline-flex;
  align-items: center;
  padding: 2px 10px;
  border-radius: 999px;
  font-size: 12px;
  font-weight: 600;
  color: color-mix(in srgb, var(--theme-color, #007bff) 80%, #000);
  background: color-mix(in srgb, var(--theme-color, #007bff) 12%, transparent);
}

.tree {
  list-style: none;
  margin: 0;
  padding: 0;
  overflow: auto;
  font-size: 12px;
  min-height: 0;
}
.tree li {
  margin: 2px 0;
  display: flex;
  align-items: baseline;
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
  text-decoration: none;
}
.tree-link:hover {
  color: var(--theme-color, #007bff);
  text-decoration: underline;
}
.tree-link.active {
  color: var(--theme-color, #007bff);
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
  gap: 6px;
}
.issue-list li {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  align-items: baseline;
}
.issue-list.compact li {
  color: #6b7280;
}
.issue-src {
  font-weight: 700;
  color: #6b7280;
}
.issue-path {
  font-family: ui-monospace, Consolas, monospace;
  color: #374151;
}
.issue-msg {
  color: #b91c1c;
}
.issue-list li.warn .issue-msg {
  color: #b45309;
}
.issue-list li.danger .issue-msg {
  color: #b91c1c;
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
  line-height: 1.45;
  max-height: 220px;
  overflow: auto;
  color: #374151;
  white-space: pre-wrap;
  word-break: break-all;
}

.paint-img {
  max-width: 100%;
  image-rendering: pixelated;
  cursor: crosshair;
  border-radius: var(--ui-radius-card, 12px);
  border: 1px solid rgba(0, 0, 0, 0.08);
  background: #fff;
  align-self: flex-start;
}

.toolbar {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  align-items: center;
  font-size: 12px;
  color: #374151;
}
.toolbar label {
  display: inline-flex;
  align-items: center;
  gap: 6px;
}
.toolbar input[type="number"],
.toolbar select,
.toolbar input[type="text"] {
  width: 72px;
  padding: 6px 8px;
  border-radius: 8px;
  border: 1px solid rgba(0, 0, 0, 0.1);
  background: rgba(255, 255, 255, 0.8);
  color: #1d1d1f;
  font: inherit;
}
.toolbar input[type="color"] {
  width: 36px;
  height: 28px;
  padding: 0;
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
  font-weight: 500;
  font-family: inherit;
  resize: vertical;
}
.field input:focus,
.field select:focus,
.field textarea:focus {
  outline: none;
  border-color: color-mix(in srgb, var(--theme-color, #007bff) 45%, transparent);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--theme-color, #007bff) 16%, transparent);
  background: #fff;
}


.row-title {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}
.ghost-btn.sm {
  padding: 4px 10px;
  font-size: 12px;
}
.scroll-box {
  max-height: 180px;
  overflow: auto;
  padding-right: 4px;
}
.ai-report {
  max-height: 280px;
  overflow: auto;
}
.tree-filter {
  width: 100%;
  box-sizing: border-box;
  padding: 8px 10px;
  border-radius: var(--ui-radius-btn, 10px);
  border: 1px solid rgba(0,0,0,0.08);
  background: rgba(255,255,255,0.8);
  color: #1d1d1f;
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
  background: rgba(0,0,0,0.03);
  border-radius: 8px;
  padding: 6px 8px;
  cursor: pointer;
  color: #1d1d1f;
  font: inherit;
  font-size: 12px;
  font-weight: 700;
}
.tree-group-btn:hover {
  background: rgba(0,0,0,0.06);
}
.tree ul {
  margin: 4px 0 0;
  padding: 0 0 0 10px;
  list-style: none;
  border-left: 1px solid rgba(0,0,0,0.06);
}
.lightbox-mask {
  position: fixed;
  inset: 0;
  z-index: 9999;
  background: rgba(15,23,42,0.55);
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
}
.lightbox-img {
  max-width: min(92vw, 1100px);
  max-height: min(82vh, 900px);
  image-rendering: pixelated;
  background: #fff;
  border-radius: 12px;
  box-shadow: 0 20px 50px rgba(0,0,0,0.35);
}
.lightbox-close {
  position: absolute;
  top: 16px;
  right: 20px;
}
.paint-img {
  max-height: 220px;
  width: auto;
}
@media (max-width: 1100px) {
  .foray-grid {
    grid-template-columns: 1fr;
    overflow: auto;
  }
  .pane {
    min-height: 240px;
  }
  .header-section {
    padding: 20px 20px 8px;
  }
  .foray-grid {
    padding: 8px 20px 20px;
  }
  .page-error {
    margin: 0 20px 8px;
  }
}
</style>
