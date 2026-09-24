<script setup lang="ts">
import { onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

const emit = defineEmits<{ (e: "leave"): void }>();

const loading = ref(false);
const err = ref("");
const opened = ref(false);
const source = ref("");
const desc = ref("");
const packFormat = ref<number | null>(null);
const files = ref<Array<{ path: string; kind: string }>>([]);
const issues = ref<Array<{ level: string; source: string; path: string; message: string }>>([]);
const selected = ref<string>("");
const previewUtf8 = ref("");
const previewPng = ref("");

// paint
const paintPng = ref("");
const brushR = ref(2);
const brushOpacity = ref(0.8);
const brushColor = ref("#ff0000");
const eyedropper = ref(false);
const hsv = ref({ dh: 0, ds: 0, dv: 0 });

// export
const exportMode = ref<"save_as" | "in_place">("save_as");
const confirmInPlace = ref(false);

// AI
const tier = ref(1);
const aiConfig = ref({ base_url: "https://api.openai.com/v1", api_key: "", model: "gpt-4o-mini", default_tier: 1, system_prompt: "" });
const aiPreview = ref<string[]>([]);
const aiText = ref("");
const aiReport = ref("");
const aiBusy = ref(false);
const aiError = ref("");


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
    const p = await open({ multiple: false, filters: [{ name: "zip", extensions: ["zip", "mcpack"] }] });
    if (typeof p === "string") await openFromPath(p);
  } catch (e: any) {
    err.value = String(e);
  }
}

async function selectPath(path: string) {
  selected.value = path;
  previewUtf8.value = "";
  previewPng.value = "";
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

async function eyedropAt(x: number, y: number) {
  // 从预览 canvas 取色（合成 1px）
  const img = document.querySelector(".paint-img") as HTMLImageElement | null;
  if (!img) return;
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
}

function colorToRgba(hex: string): [number, number, number, number] {
  const h = hex.replace("#", "");
  const r = parseInt(h.slice(0, 2), 16);
  const g = parseInt(h.slice(2, 4), 16);
  const b = parseInt(h.slice(4, 6), 16);
  return [r, g, b, 255];
}

async function onPaintClick(ev: MouseEvent) {
  const img = ev.currentTarget as HTMLImageElement;
  const rect = img.getBoundingClientRect();
  const scaleX = img.naturalWidth / rect.width;
  const scaleY = img.naturalHeight / rect.height;
  const x = Math.floor((ev.clientX - rect.left) * scaleX);
  const y = Math.floor((ev.clientY - rect.top) * scaleY);
  // Shift+点击 或 吸管模式：取色
  if (eyedropper.value || ev.shiftKey) {
    await eyedropAt(x, y);
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

function restoreDefaultPrompt() {
  aiConfig.value.system_prompt = "";
  // 后端在空字符串时使用内置默认；这里再拉一次连接配置即可
}

async function copyReport() {
  try {
    await navigator.clipboard.writeText(aiReport.value);
    alert("报告已复制");
  } catch (e) {
    console.warn(e);
  }
}

async function saveAiConfig() {
  await invoke("foray_ai_config_set", { config: aiConfig.value });
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
  // 支持 URL ?path= 直接打开
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
    <header class="foray-top">
      <button class="btn ghost" @click="emit('leave')">← 返回</button>
      <h2>Foray 工作台</h2>
      <span class="mono muted">{{ source || "未打开" }}</span>
      <div class="spacer" />
      <button class="btn" :disabled="loading" @click="pickFile">打开 zip</button>
    </header>

    <p v-if="err" class="error">{{ err }}</p>
    <p v-if="!opened" class="hint">
      Editor Mode 已开启。从主页拖入 zip，或在此选择资源包进入分析。普通转换已在此模式下停用。
    </p>

    <div v-if="opened" class="foray-grid">
      <aside class="pane">
        <h3>ROM 树</h3>
        <ul class="tree">
          <li v-for="f in files" :key="f.path">
            <button class="link" :class="{ active: f.path === selected }" @click="selectPath(f.path)">
              {{ f.path }}
            </button>
            <span class="badge">{{ f.kind }}</span>
          </li>
        </ul>
      </aside>

      <main class="pane">
        <h3>概览</h3>
        <p>
          {{ desc || "（无描述）" }}
          · format {{ packFormat ?? "?" }}
          · {{ files.length }} files
        </p>
        <h3>探针 / 问题</h3>
        <ul class="issues">
          <li v-for="(i, idx) in issues" :key="idx" :class="i.level">
            <b>{{ i.source }}</b> {{ i.path }} — {{ i.message }}
          </li>
          <li v-if="!issues.length" class="muted">无 issue</li>
        </ul>

        <h3>预览 {{ selected }}</h3>
        <pre v-if="previewUtf8" class="code">{{ previewUtf8.slice(0, 4000) }}</pre>
        <img v-if="paintPng || previewPng" class="paint-img" :src="paintPng || previewPng" @click="onPaintClick" />

        <template v-if="paintPng">
          <h3>轻量编辑</h3>
          <div class="row">
            <label>笔刷 <input v-model.number="brushR" type="number" min="1" max="32" /></label>
            <label>透明 <input v-model.number="brushOpacity" type="number" min="0" max="1" step="0.05" /></label>
            <label>颜色 <input v-model="brushColor" type="color" /></label>
            <label><input v-model="eyedropper" type="checkbox" /> 吸管（或 Shift+点击）</label>
            <button class="btn" @click="undoPaint">撤销</button>
            <button class="btn primary" @click="commitPaint">应用到 ROM</button>
          </div>
          <div class="row">
            <label>H <input v-model.number="hsv.dh" type="number" min="-180" max="180" /></label>
            <label>S <input v-model.number="hsv.ds" type="number" min="-100" max="100" /></label>
            <label>V <input v-model.number="hsv.dv" type="number" min="-100" max="100" /></label>
            <button class="btn" @click="applyHsv">HSV</button>
          </div>
        </template>

        <h3>导出</h3>
        <div class="row">
          <label><input v-model="exportMode" type="radio" value="save_as" /> 另存副本（默认）</label>
          <label><input v-model="exportMode" type="radio" value="in_place" /> 原地覆盖</label>
          <label v-if="exportMode === 'in_place'">
            <input v-model="confirmInPlace" type="checkbox" /> 确认覆盖（写 .bak）
          </label>
          <button class="btn primary" @click="doExport">导出</button>
        </div>
      </main>

      <aside class="pane">
        <h3>AI 分析（OpenAI 兼容）</h3>
        <label>baseURL <input v-model="aiConfig.base_url" /></label>
        <label>API Key <input v-model="aiConfig.api_key" type="password" /></label>
        <label>模型 <input v-model="aiConfig.model" /></label>
        <div class="row">
          <button class="btn" @click="restoreDefaultPrompt">还原默认提示词</button>
          <button class="btn" :disabled="!aiReport" @click="copyReport">复制报告</button>
        </div>
        <label>提示词 <textarea v-model="aiConfig.system_prompt" rows="4" style="width:100%"></textarea></label>
        <label>档位
          <select v-model.number="tier">
            <option :value="0">0 无</option>
            <option :value="1">1 目录树</option>
            <option :value="2">2 +mcmeta</option>
            <option :value="3">3 +JSON（选中）</option>
            <option :value="4">4 +着色器（选中）</option>
            <option :value="5">5 +贴图概括</option>
          </select>
        </label>
        <p class="muted small">≥3 仅发送当前选中文件副本；贴图只发概括不发像素。外部 API 与作者无关。</p>
        <div class="row">
          <button class="btn" @click="prepareAi">将发送预览</button>
          <button class="btn primary" :disabled="aiBusy" @click="runAi">
            {{ aiBusy ? "分析中…" : "开始分析" }}
          </button>
        </div>
        <ul class="issues">
          <li v-for="(p, i) in aiPreview" :key="i">{{ p }}</li>
        </ul>
        <p v-if="aiError" class="error">{{ aiError }}</p>
        <pre v-if="aiReport" class="code">{{ aiReport }}</pre>

        <h3>Ifaso</h3>
        <p class="muted small">互链入口占位（2.5.0 不实现互通）。跨软件按对方 License。</p>
      </aside>
    </div>
  </div>
</template>

<style scoped>
.foray-page {
  padding: 12px 16px 32px;
  color: var(--text, #e8eefc);
}
.foray-top {
  display: flex;
  gap: 10px;
  align-items: center;
  margin-bottom: 12px;
}
.foray-top h2 {
  margin: 0;
  font-size: 18px;
}
.spacer {
  flex: 1;
}
.foray-grid {
  display: grid;
  grid-template-columns: 280px 1fr 300px;
  gap: 12px;
}
@media (max-width: 1100px) {
  .foray-grid {
    grid-template-columns: 1fr;
  }
}
.pane {
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid rgba(148, 163, 184, 0.2);
  border-radius: 12px;
  padding: 12px;
  min-height: 320px;
}
.pane h3 {
  margin: 10px 0 6px;
  font-size: 13px;
  opacity: 0.8;
  text-transform: uppercase;
  letter-spacing: 0.04em;
}
.tree {
  list-style: none;
  margin: 0;
  padding: 0;
  max-height: 70vh;
  overflow: auto;
  font-size: 12px;
}
.tree li {
  margin: 3px 0;
  word-break: break-all;
}
.link {
  background: none;
  border: none;
  color: inherit;
  cursor: pointer;
  text-align: left;
  padding: 0;
  text-decoration: underline;
}
.link.active {
  color: #3b82f6;
  font-weight: 700;
}
.badge {
  font-size: 10px;
  opacity: 0.7;
  margin-left: 6px;
}
.issues {
  margin: 0;
  padding-left: 1.1em;
  font-size: 12px;
}
.issues .danger {
  color: #ef4444;
}
.issues .warn {
  color: #f59e0b;
}
.paint-img {
  max-width: 100%;
  image-rendering: pixelated;
  cursor: crosshair;
  background: #111;
  border-radius: 8px;
}
.code {
  max-height: 240px;
  overflow: auto;
  background: rgba(0, 0, 0, 0.25);
  border-radius: 8px;
  padding: 8px;
  font-size: 12px;
}
.row {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  align-items: center;
  margin: 6px 0;
}
label {
  display: inline-flex;
  gap: 4px;
  align-items: center;
  font-size: 12px;
}
input,
select {
  background: rgba(0, 0, 0, 0.25);
  border: 1px solid rgba(148, 163, 184, 0.25);
  color: inherit;
  border-radius: 6px;
  padding: 4px 6px;
}
.btn {
  border: 1px solid rgba(148, 163, 184, 0.3);
  background: rgba(255, 255, 255, 0.06);
  color: inherit;
  border-radius: 8px;
  padding: 6px 10px;
  cursor: pointer;
}
.btn.primary {
  background: #3b82f6;
  border-color: #3b82f6;
}
.btn.ghost {
  background: transparent;
}
.muted {
  opacity: 0.65;
}
.small {
  font-size: 11px;
}
.error {
  color: #f87171;
}
.hint {
  opacity: 0.75;
}
</style>
