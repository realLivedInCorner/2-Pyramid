<script setup lang="ts">
/**
 * The in-app file browser.
 *
 * One component, three modes (`files` / `dir` / `save`) — they share the
 * listing, the navigation and the rail, and differ only in what the confirm
 * button produces. Splitting them would triplicate the navigation logic.
 *
 * Reads directories through the `list_dir` / `fs_locations` commands; it
 * never touches the filesystem itself.
 */
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { useI18n } from 'vue-i18n';
import { useFilePickerState } from '../../composables/useFilePicker';

const { t } = useI18n();
const picker = useFilePickerState();

interface DirEntry {
  name: string;
  path: string;
  is_dir: boolean;
  size: number;
  modified: number | null;
  hidden: boolean;
}
interface DirListing {
  path: string;
  parent: string | null;
  entries: DirEntry[];
  truncated: boolean;
}
interface FsLocation {
  id: string;
  label: string;
  label_key: string | null;
  path: string;
  kind: string;
}

/**
 * Set by the host page so "use the system dialog" can reach the plugin.
 * Kept as an injected callback rather than importing the plugin here: this
 * component should stay usable, and testable, without Tauri's dialog present.
 */
const nativeFallback = ref<(() => void) | null>(null);

const locations = ref<FsLocation[]>([]);
const listing = ref<DirListing | null>(null);
const loading = ref(false);
const error = ref<string | null>(null);

const filter = ref('');
const showHidden = ref(false);
const selected = ref<Set<string>>(new Set());
const saveName = ref('');
const listEl = ref<HTMLElement | null>(null);

const mode = computed(() => picker.pending.value?.options.mode ?? 'files');
const multiple = computed(() => picker.pending.value?.options.multiple ?? false);
const extensions = computed(() => picker.pending.value?.options.extensions ?? []);

/**
 * Window title. Defaults by mode so a call site only has to pass `titleKey`
 * when it wants something more specific than "Select files / folder / Save as".
 *
 * Note the keys carry no inner dots: vue-i18n treats `.` as a path separator,
 * so a flat key named `title.files` inside `fs` would resolve as fs→title→files
 * and render the raw key instead of the translation.
 */
const titleKey = computed(() => {
  const explicit = picker.pending.value?.options.titleKey;
  if (explicit) return explicit;
  if (mode.value === 'dir') return 'fs.titleDir';
  if (mode.value === 'save') return 'fs.titleSave';
  return 'fs.titleFiles';
});

function label(loc: FsLocation) {
  return loc.label_key ? t(loc.label_key) : loc.label;
}

/** Extension test, lower-cased. An empty filter list means "everything". */
function matchesExtension(name: string, isDir: boolean) {
  if (isDir) return true;
  const exts = extensions.value;
  if (!exts.length) return true;
  const dot = name.lastIndexOf('.');
  if (dot < 0) return false;
  return exts.includes(name.slice(dot + 1).toLowerCase());
}

/**
 * Filtered entries.
 *
 * Non-matching *files* are kept but marked unselectable, so the folder never
 * looks emptier than it is — if a pack is missing the user can see it is
 * there and merely excluded, rather than wondering whether the browse went
 * to the wrong place.
 */
const visible = computed(() => {
  const all = listing.value?.entries ?? [];
  const needle = filter.value.trim().toLowerCase();
  return all
    .filter((e) => showHidden.value || !e.hidden)
    .filter((e) => !needle || e.name.toLowerCase().includes(needle));
});

function selectable(e: DirEntry) {
  if (e.is_dir) return mode.value === 'dir';
  return mode.value !== 'dir' && matchesExtension(e.name, false);
}

function formatSize(bytes: number) {
  if (bytes < 1024) return `${bytes} B`;
  const units = ['KB', 'MB', 'GB', 'TB'];
  let v = bytes / 1024;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i += 1;
  }
  return `${v < 10 ? v.toFixed(1) : Math.round(v)} ${units[i]}`;
}

function formatDate(ms: number | null) {
  if (!ms) return '—';
  const d = new Date(ms);
  const p = (n: number) => String(n).padStart(2, '0');
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}`;
}

/** Breadcrumb segments, each clickable. Drive roots keep their trailing `\`. */
const crumbs = computed(() => {
  const p = listing.value?.path;
  if (!p) return [] as { label: string; path: string }[];
  const isWin = /^[A-Za-z]:\\/.test(p);
  const sep = isWin ? '\\' : '/';
  const parts = p.split(sep).filter(Boolean);
  const out: { label: string; path: string }[] = [];
  let acc = isWin ? '' : '/';
  for (const part of parts) {
    acc = acc === '' ? `${part}\\` : acc === '/' ? `/${part}` : `${acc}${sep}${part}`;
    out.push({ label: part, path: acc });
  }
  return out;
});

async function loadLocations() {
  try {
    locations.value = await invoke<FsLocation[]>('fs_locations');
  } catch {
    // Shortcuts are a convenience; their absence must not block browsing.
    locations.value = [];
  }
}

async function navigate(path: string) {
  loading.value = true;
  error.value = null;
  try {
    const res = await invoke<DirListing>('list_dir', { path });
    listing.value = res;
    picker.setLastDir(res.path);
    selected.value = new Set();
    await nextTick();
    listEl.value?.scrollTo({ top: 0 });
  } catch (e) {
    error.value = String(e);
    // Keep the previous listing on screen: a failed jump is easier to
    // recover from when you can still see where you were.
  } finally {
    loading.value = false;
  }
}

function goUp() {
  const parent = listing.value?.parent;
  if (parent) navigate(parent);
}

function toggle(e: DirEntry, additive: boolean) {
  if (!selectable(e)) return;

  let next: Set<string>;
  if (additive && multiple.value) {
    // Ctrl/Cmd 只在多选模式下是「追加」，其它模式下它没有意义，
    // 当作普通点击处理 —— 否则 `dir` 模式会选出一个永远用不到的集合。
    next = new Set(selected.value);
    if (next.has(e.path)) next.delete(e.path);
    else next.add(e.path);
  } else {
    next = new Set([e.path]);
  }
  selected.value = next;

  // Save mode mirrors the highlighted file into the name box, which is what
  // makes "overwrite this one" a two-click operation.
  if (mode.value === 'save' && !e.is_dir) saveName.value = e.name;
}

function enterEntry(e: DirEntry) {
  if (e.is_dir) navigate(e.path);
  else confirm();
}

async function confirm() {
  if (mode.value === 'dir') {
    const one = [...selected.value][0];
    picker.finish(one ?? null);
    return;
  }
  if (mode.value === 'save') {
    const name = saveName.value.trim();
    const dir = listing.value?.path;
    if (!name || !dir) return;
    const sep = dir.endsWith('\\') || dir.endsWith('/') ? '' : navigator.userAgent.includes('Windows') ? '\\' : '/';
    picker.finish(`${dir}${sep}${name}`);
    return;
  }
  picker.finish(selected.value.size ? [...selected.value] : null);
}

function cancel() {
  picker.finish(null);
}

const canConfirm = computed(() => {
  if (mode.value === 'dir') return selected.value.size > 0;
  if (mode.value === 'save') return saveName.value.trim().length > 0;
  return selected.value.size > 0;
});

function onKeydown(ev: KeyboardEvent) {
  if (!picker.open.value) return;
  if (ev.key === 'Escape') {
    ev.preventDefault();
    cancel();
  } else if (ev.key === 'Enter') {
    ev.preventDefault();
    if (canConfirm.value) confirm();
  } else if (ev.key === 'Backspace' && !(ev.target as HTMLElement)?.closest('input')) {
    ev.preventDefault();
    goUp();
  }
}

/**
 * Resolve the starting directory.
 *
 * Order: an explicit `startDir` from the call site, then whatever was last
 * browsed, then the first shortcut that exists. Falling straight to a drive
 * root would drop the user in `C:\` every time, which is the least useful
 * place on the machine.
 */
async function initialDir() {
  const explicit = picker.pending.value?.options.startDir;
  if (explicit) return explicit;
  if (picker.lastDir.value) return picker.lastDir.value;
  if (!locations.value.length) await loadLocations();
  const preferred = ['downloads', 'desktop', 'home'];
  for (const kind of preferred) {
    const hit = locations.value.find((l) => l.kind === kind);
    if (hit) return hit.path;
  }
  return locations.value[0]?.path ?? null;
}

/**
 * Reset per-invocation state.
 *
 * Watches the *pending entry*, not `open`. Opening a second picker while one
 * is already up leaves `open` at `true`, so an `open` watcher never fires and
 * the new invocation inherits the previous one's filter, selection and file
 * name. The entry object is replaced on every `begin()`, so watching it
 * catches both the first open and every re-entry.
 */
watch(
  () => picker.pending.value,
  async (entry) => {
    if (!entry) return;
    filter.value = '';
    selected.value = new Set();
    error.value = null;
    listing.value = null;
    saveName.value = entry.options.defaultName ?? '';
    await loadLocations();
    const dir = await initialDir();
    if (dir) await navigate(dir);
  }
);

onMounted(() => window.addEventListener('keydown', onKeydown));
onUnmounted(() => window.removeEventListener('keydown', onKeydown));

defineExpose({ nativeFallback });
</script>

<template>
  <Transition name="fb-fade">
    <div v-if="picker.open.value" class="dialog-overlay fb-overlay" @click.self="cancel">
      <div class="dialog-content fb" @click.stop>
        <div class="dialog-header">
          <div>
            <h3>{{ t(titleKey) }}</h3>
            <p class="dialog-sub">{{ t('fs.subtitle') }}</p>
          </div>
          <button class="dialog-close" @click="cancel" :aria-label="t('common.close')">×</button>
        </div>

        <!-- 导航行：上级 + 面包屑 + 名称过滤 -->
        <div class="fb-nav">
          <button class="fb-up" :disabled="!listing?.parent" @click="goUp" :title="t('fs.up')">
            <i class="ri-arrow-up-line" aria-hidden="true"></i>
          </button>
          <div class="fb-crumbs">
            <template v-for="(c, i) in crumbs" :key="c.path">
              <span v-if="i" class="fb-sep">›</span>
              <button class="fb-crumb" @click="navigate(c.path)">{{ c.label }}</button>
            </template>
          </div>
          <label class="fb-filter">
            <i class="ri-search-line" aria-hidden="true"></i>
            <input v-model="filter" type="text" :placeholder="t('fs.filter')" spellcheck="false" />
          </label>
        </div>

        <div class="fb-body">
          <!-- 快捷位置 -->
          <nav class="fb-rail">
            <button
              v-for="loc in locations"
              :key="loc.id"
              class="fb-loc"
              :class="{ on: listing?.path === loc.path }"
              @click="navigate(loc.path)"
            >
              <i
                :class="loc.kind === 'drive' ? 'ri-hard-drive-2-line' : 'ri-folder-line'"
                aria-hidden="true"
              ></i>
              <span>{{ label(loc) }}</span>
            </button>
          </nav>

          <!-- 列表 -->
          <div class="fb-list-wrap">
            <div class="fb-head">
              <span class="c-name">{{ t('fs.name') }}</span>
              <span class="c-size">{{ t('fs.size') }}</span>
              <span class="c-time">{{ t('fs.modified') }}</span>
            </div>

            <div ref="listEl" class="fb-list">
              <p v-if="loading" class="fb-state">{{ t('fs.loading') }}</p>
              <p v-else-if="error" class="fb-state err">{{ error }}</p>
              <p v-else-if="!visible.length" class="fb-state">{{ t('fs.empty') }}</p>

              <!-- v-for 不能和 v-else 写在同一个元素上：Vue 3 里 v-if 的优先级
                   高于 v-for，那个组合不会渲染出任何行。必须用 template 包一层。 -->
              <template v-else>
                <button
                  v-for="e in visible"
                  :key="e.path"
                  class="fb-row"
                  :class="{
                    on: selected.has(e.path),
                    dim: !selectable(e),
                    dir: e.is_dir,
                  }"
                  @click="toggle(e, $event.ctrlKey || $event.metaKey)"
                  @dblclick="enterEntry(e)"
                >
                  <span class="c-name">
                    <i :class="e.is_dir ? 'ri-folder-fill' : 'ri-file-line'" aria-hidden="true"></i>
                    {{ e.name }}
                  </span>
                  <span class="c-size">{{ e.is_dir ? '—' : formatSize(e.size) }}</span>
                  <span class="c-time">{{ formatDate(e.modified) }}</span>
                </button>
              </template>

              <p v-if="listing?.truncated" class="fb-state warn">
                {{ t('fs.truncated', { max: visible.length }) }}
              </p>
            </div>
          </div>
        </div>

        <div class="fb-foot dialog-footer">
          <label class="fb-hidden">
            <input v-model="showHidden" type="checkbox" />
            <span>{{ t('fs.showHidden') }}</span>
          </label>

          <span class="fb-count">
            {{ selected.size ? t('fs.selected', { n: selected.size }) : '' }}
          </span>

          <input
            v-if="mode === 'save'"
            v-model="saveName"
            class="fb-name"
            type="text"
            :placeholder="t('fs.fileName')"
            spellcheck="false"
          />

          <!-- 退路：应用内浏览器拿不到的东西（网络驱动器、超长路径）仍有系统对话框可用 -->
          <button
            v-if="nativeFallback"
            class="btn-text"
            :title="t('fs.useNative')"
            @click="nativeFallback()"
          >
            <i class="ri-external-link-line" aria-hidden="true"></i>
          </button>

          <button class="btn-text" @click="cancel">{{ t('common.cancel') }}</button>
          <button class="btn-text primary" :disabled="!canConfirm" @click="confirm">
            {{ mode === 'save' ? t('common.save') : t('fs.select') }}
          </button>
        </div>
      </div>
    </div>
  </Transition>
</template>

<style scoped>
/*
 * 宽变体。文件列表需要横向空间，420px 会把「名称/大小/时间」三列挤成两行。
 * 覆盖 .dialog-content 的宽度而不是另起一套外壳 —— 遮罩、圆角、投影、
 * 发丝边都还来自 shared.css。
 */
.fb {
  width: min(880px, 94vw);
  height: min(620px, 86vh);
  padding: 18px 20px 16px;
}

.fb-nav {
  display: flex;
  align-items: center;
  gap: 8px;
  padding-bottom: 12px;
  border-bottom: 1px solid rgba(120, 110, 95, 0.14);
  flex-shrink: 0;
}

.fb-up {
  width: 28px;
  height: 28px;
  flex-shrink: 0;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: 1px solid rgba(120, 110, 95, 0.2);
  border-radius: 6px;
  background: transparent;
  color: #5f5a51;
  cursor: pointer;
  transition: background 0.15s ease;
}
.fb-up:hover:not(:disabled) { background: rgba(120, 110, 95, 0.08); color: #14171a; }
.fb-up:disabled { opacity: 0.35; cursor: default; }

/* 面包屑：过长时横向滚动而不是折行，路径永远是一行 */
.fb-crumbs {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 2px;
  overflow-x: auto;
  scrollbar-width: none;
}
.fb-crumbs::-webkit-scrollbar { display: none; }
.fb-crumb {
  flex-shrink: 0;
  padding: 3px 6px;
  border: none;
  border-radius: 5px;
  background: transparent;
  font-family: inherit;
  font-size: 12.5px;
  color: #5f5a51;
  cursor: pointer;
  white-space: nowrap;
}
.fb-crumb:hover { background: rgba(120, 110, 95, 0.09); color: #14171a; }
.fb-sep { color: #b8b2a6; font-size: 12px; }

.fb-filter {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
  width: 160px;
  height: 28px;
  padding: 0 9px;
  border: 1px solid rgba(120, 110, 95, 0.2);
  border-radius: 8px;
  color: #9c968c;
}
.fb-filter input {
  flex: 1;
  min-width: 0;
  border: none;
  outline: none;
  background: transparent;
  font-family: inherit;
  font-size: 12.5px;
  color: #14171a;
}

.fb-body {
  flex: 1;
  min-height: 0;
  display: grid;
  grid-template-columns: 168px minmax(0, 1fr);
  gap: 14px;
  padding-top: 12px;
}

.fb-rail {
  display: flex;
  flex-direction: column;
  gap: 1px;
  overflow-y: auto;
  padding-right: 2px;
}
.fb-loc {
  display: flex;
  align-items: center;
  gap: 8px;
  min-height: 30px;
  padding: 0 9px;
  border: none;
  border-radius: 6px;
  background: transparent;
  font-family: inherit;
  font-size: 12.5px;
  color: #5f5a51;
  text-align: left;
  cursor: pointer;
  transition: background 0.15s ease;
}
.fb-loc i { font-size: 14px; color: #9c968c; flex-shrink: 0; }
.fb-loc span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.fb-loc:hover { background: rgba(120, 110, 95, 0.07); }
/* 选中 = 墨色，和全局「静默优先、选中才亮」一致 */
.fb-loc.on { background: rgba(20, 23, 26, 0.06); color: #14171a; font-weight: 500; }
.fb-loc.on i { color: #14171a; }

.fb-list-wrap { display: flex; flex-direction: column; min-height: 0; }

/* 表头与行共用同一组列宽，不会错位 */
.fb-head,
.fb-row {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 84px 132px;
  align-items: center;
  gap: 10px;
}

.fb-head {
  padding: 0 8px 7px;
  border-bottom: 1px solid rgba(120, 110, 95, 0.2);
  font-size: 10px;
  font-weight: 400;
  letter-spacing: 0.16em;
  text-transform: uppercase;
  color: #9c968c;
  flex-shrink: 0;
}
.fb-head .c-size,
.fb-head .c-time { text-align: right; }

.fb-list { flex: 1; min-height: 0; overflow-y: auto; }

.fb-row {
  width: 100%;
  min-height: 34px;
  padding: 0 8px;
  border: none;
  border-bottom: 1px solid rgba(120, 110, 95, 0.08);
  border-radius: 6px;
  background: transparent;
  font-family: inherit;
  font-size: 13px;
  color: #14171a;
  text-align: left;
  cursor: pointer;
  transition: background 0.12s ease;
}
.fb-row:hover { background: rgba(120, 110, 95, 0.05); }
.fb-row.on { background: rgba(20, 23, 26, 0.08); }
/* 被扩展名过滤掉的文件仍在列表里，只是选不中 —— 见模板注释 */
.fb-row.dim { color: #b8b2a6; cursor: default; }
.fb-row.dim .c-name i { color: #cfc9bf; }
.fb-row.dim:hover { background: transparent; }

.c-name {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.c-name i { flex-shrink: 0; font-size: 14px; color: #9c968c; }
.fb-row.dir .c-name i { color: #7d776d; }
.c-size,
.c-time {
  text-align: right;
  font-size: 11.5px;
  color: #9c968c;
  font-variant-numeric: tabular-nums;
}

.fb-state {
  margin: 0;
  padding: 28px 8px;
  text-align: center;
  font-size: 12.5px;
  color: #9c968c;
}
.fb-state.err { color: #b3261e; }
.fb-state.warn { padding: 10px 8px; color: #b45309; }

.fb-foot { gap: 10px; }
.fb-hidden {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  margin-right: auto;
  font-size: 12px;
  color: #7d776d;
  cursor: pointer;
  user-select: none;
}
.fb-hidden input { accent-color: #14171a; }
.fb-count { font-size: 12px; color: #9c968c; font-variant-numeric: tabular-nums; }

.fb-name {
  width: 220px;
  height: 32px;
  padding: 0 10px;
  border: 1px solid rgba(120, 110, 95, 0.22);
  border-radius: 8px;
  background: #fff;
  font-family: inherit;
  font-size: 13px;
  color: #14171a;
  outline: none;
}
.fb-name:focus { border-color: rgba(20, 23, 26, 0.4); }

/* fill-mode 用 backwards：both / forwards 会永久保留 translate: 0 0，
   元素成为包含块，里面的 fixed 浮层会错位。基线里也不放 opacity: 0。 */
.fb-fade-enter-active { transition: opacity 0.18s ease; }
.fb-fade-leave-active { transition: opacity 0.14s ease; }
.fb-fade-enter-from,
.fb-fade-leave-to { opacity: 0; }

@media (prefers-reduced-motion: reduce) {
  .fb-fade-enter-active,
  .fb-fade-leave-active { transition: none; }
  .fb-row, .fb-loc, .fb-up { transition: none; }
}
</style>
