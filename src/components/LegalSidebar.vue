<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { renderMarkdown } from "../utils/markdown";
import { useSidebarSlide } from "../composables/useSidebarSlide";

const { t } = useI18n();
const visible = defineModel<boolean>({ required: true });

const { onBeforeEnter, onEnter, onAfterEnter, onBeforeLeave, onLeave, onAfterLeave } =
  useSidebarSlide({ shadow: "-12px 0 36px rgba(0, 0, 0, 0.12)" });

const selectedFile = ref("");
const content = ref("");
const error = ref("");
const loading = ref(false);

const files = [
  { file: "EULA.md", label: "EULA" },
  { file: "PRIVACY.md", label: "Privacy" },
  { file: "DISCLAIMER.md", label: "Disclaimer" },
  { file: "THIRD-PARTY-NOTICES.md", label: "Third-Party" },
  { file: "SECURITY.md", label: "Security" },
  { file: "CONTRIBUTING.md", label: "Contributing" },
];

function openAndLoad() {
  if (!selectedFile.value) void loadFile("EULA.md");
}

async function loadFile(name: string) {
  selectedFile.value = name;
  loading.value = true;
  error.value = "";
  content.value = "";
  try {
    const raw = await invoke<string>("read_legal_file", { filename: name });
    content.value = renderMarkdown(raw);
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

async function openFolder() {
  try {
    const exeDir = await invoke<string | null>("get_install_dir");
    const candidates = [exeDir ? `${exeDir}\\legal` : "", "legal"].filter(Boolean);
    let lastErr: unknown = null;
    for (const p of candidates) {
      try {
        await invoke("open_folder", { path: p });
        return;
      } catch (e) {
        lastErr = e;
      }
    }
    console.error("open legal folder failed", lastErr);
  } catch (e) {
    console.error("open legal folder failed", e);
  }
}

function onMdClick(ev: MouseEvent) {
  const link = (ev.target as HTMLElement | null)?.closest?.("a[data-ext-link]") as HTMLAnchorElement | null;
  if (!link) return;
  ev.preventDefault();
  const href = link.getAttribute("href");
  if (href && /^https?:\/\//i.test(href)) {
    void openUrl(href).catch(() => {});
  }
}

// 打开时预载第一份文件
import { watch } from "vue";
watch(visible, (v) => {
  if (v) openAndLoad();
});
</script>

<template>
  <transition name="sidebar-overlay-fade">
    <div v-if="visible" class="sidebar-overlay" @click="visible = false"></div>
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
      class="sidebar-content legal-sidebar"
      @click.stop
      tabindex="-1"
    >
      <div class="legal-header">
        <div class="legal-header-text">
          <h3>{{ t("settings.legal.title") }}</h3>
          <p>{{ t("settings.legal.hint") }}</p>
        </div>
        <button class="btn-text" @click="openFolder" :title="t('settings.legal.openFolder')">
          <i class="ri-folder-open-line"></i>
        </button>
      </div>

      <div class="legal-body">
        <ul class="legal-file-list">
          <li
            v-for="f in files"
            :key="f.file"
            :class="{ active: selectedFile === f.file }"
            @click="loadFile(f.file)"
          >
            <i class="ri-file-text-line"></i>
            <span>{{ f.label }}</span>
          </li>
        </ul>
        <div class="legal-file-view">
          <div v-if="loading" class="legal-loading">
            <i class="ri-loader-4-line spin"></i>
          </div>
          <div
            v-else-if="content"
            class="legal-md"
            v-html="content"
            @click="onMdClick"
          ></div>
          <div v-else class="legal-empty">{{ error || t("settings.legal.pickFile") }}</div>
        </div>
      </div>

      <div class="legal-footer">
        <button class="legal-back-btn" @click="visible = false">
          <i class="ri-arrow-go-back-line"></i>
          <span>{{ t("common.back") }}</span>
        </button>
      </div>
    </aside>
  </transition>
</template>

<style scoped>
/* 遮罩：点击空白关闭（此前无样式，点不到） */
.sidebar-overlay {
  position: fixed;
  inset: 0;
  background: rgba(15, 23, 42, 0.28);
  z-index: 200;
  backdrop-filter: blur(4px);
}

/* 定位与全局 .sidebar-content 一致（scoped 下需自带） */
.legal-sidebar.sidebar-content {
  position: fixed;
  top: 0;
  right: 0;
  width: min(560px, 94vw);
  height: 100vh;
  background: #fff;
  box-shadow: -12px 0 36px rgba(0, 0, 0, 0.08);
  display: flex;
  flex-direction: column;
  outline: none;
  z-index: 201;
  transform: translateX(100%);
  opacity: 1 !important;
  will-change: transform, box-shadow;
}

.legal-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  padding: 1.25rem 1.5rem 1rem;
  border-bottom: 1px solid rgba(0, 0, 0, 0.06);
  flex-shrink: 0;
}
.legal-header-text { min-width: 0; flex: 1; }
.legal-header h3 {
  margin: 0;
  font-size: 18px;
  font-weight: 700;
  color: #1d1d1f;
}
.legal-header p {
  margin: 4px 0 0;
  font-size: 12px;
  color: #86868b;
  line-height: 1.5;
  max-width: 42ch;
}

.legal-body {
  flex: 1;
  min-height: 0;
  display: grid;
  grid-template-columns: 150px minmax(0, 1fr);
}

.legal-file-list {
  list-style: none;
  margin: 0;
  padding: 10px;
  border-right: 1px solid rgba(0, 0, 0, 0.06);
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.legal-file-list li {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
  border-radius: 10px;
  cursor: pointer;
  font-size: 13px;
  font-weight: 600;
  color: #475569;
  transition: background 0.15s ease, color 0.15s ease;
}
.legal-file-list li:hover { background: rgba(0, 0, 0, 0.04); color: #1d1d1f; }
.legal-file-list li.active {
  background: color-mix(in srgb, var(--theme-color, #007bff) 10%, #fff);
  color: var(--theme-color, #007bff);
}

.legal-file-view {
  min-height: 0;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.legal-loading,
.legal-empty {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #94a3b8;
  font-size: 13px;
}

.legal-md {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 16px 18px 20px;
  font-size: 13px;
  line-height: 1.65;
  color: #334155;
}
.legal-md :deep(h1),
.legal-md :deep(h2),
.legal-md :deep(h3),
.legal-md :deep(h4) {
  margin: 1.1em 0 0.5em;
  color: #1d1d1f;
  font-weight: 700;
  line-height: 1.3;
}
.legal-md :deep(h1) { font-size: 18px; }
.legal-md :deep(h2) { font-size: 16px; }
.legal-md :deep(h3) { font-size: 14px; }
.legal-md :deep(h4) { font-size: 13px; }
.legal-md :deep(p) { margin: 0 0 0.75em; }
.legal-md :deep(ul),
.legal-md :deep(ol) { margin: 0 0 0.75em; padding-left: 1.4em; }
.legal-md :deep(li) { margin: 0.25em 0; }
.legal-md :deep(blockquote) {
  margin: 0 0 0.75em;
  padding: 8px 12px;
  border-left: 3px solid color-mix(in srgb, var(--theme-color, #007bff) 40%, transparent);
  background: rgba(0, 0, 0, 0.03);
  border-radius: 0 8px 8px 0;
  color: #475569;
}
.legal-md :deep(code) {
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 12px;
  padding: 1px 5px;
  border-radius: 4px;
  background: rgba(0, 0, 0, 0.06);
}
.legal-md :deep(pre) {
  margin: 0 0 0.75em;
  padding: 12px 14px;
  border-radius: 10px;
  background: #0f172a;
  color: #e2e8f0;
  overflow-x: auto;
}
.legal-md :deep(pre code) {
  padding: 0;
  background: transparent;
  color: inherit;
  font-size: 12px;
}
.legal-md :deep(hr) {
  border: none;
  border-top: 1px solid rgba(0, 0, 0, 0.08);
  margin: 1em 0;
}
.legal-md :deep(a) {
  color: var(--theme-color, #007bff);
  text-decoration: underline;
  text-underline-offset: 2px;
}

.legal-footer {
  flex-shrink: 0;
  display: flex;
  justify-content: flex-end;
  padding: 12px 16px;
  border-top: 1px solid rgba(0, 0, 0, 0.06);
  background: #fff;
}
.legal-back-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  height: 36px;
  padding: 0 16px;
  border: none;
  border-radius: 10px;
  background: rgba(0, 0, 0, 0.04);
  color: #64748b;
  font-family: inherit;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: background 0.15s ease, color 0.15s ease;
}
.legal-back-btn:hover {
  background: rgba(0, 0, 0, 0.08);
  color: #1d1d1f;
}
</style>
