/**
 * In-app file picker — the front door for every file/folder choice.
 *
 * Replaces `@tauri-apps/plugin-dialog`'s `open` / `save`. The app used to
 * hand those choices to the OS dialog, which cannot be themed, cannot filter
 * the way the app wants, and looks like a different program.
 *
 * Shaped as a module-level singleton rather than a per-component state:
 * only one picker can be open at a time anyway, and a singleton means the
 * component is mounted once in `App.vue` instead of inside all ten call
 * sites. Call sites just `await pickFile(...)`.
 *
 *   const files = await pickFiles({ extensions: ['zip', 'mcpack'] })
 *   const dir   = await pickDirectory()
 *   const dest  = await pickSavePath({ defaultName: 'pack.zip' })
 *
 * Every one of them resolves to `null` when the user cancels — the same
 * contract the previous native calls had, so the call sites keep working.
 */
import { readonly, ref } from 'vue';

export type PickerMode = 'files' | 'dir' | 'save';

export interface PickerOptions {
  mode: PickerMode;
  /** i18n key for the window title. */
  titleKey?: string;
  /**
   * Lower-case extensions without the dot. Files that do not match are shown
   * but not selectable — hiding them makes the folder look empty and the
   * user cannot tell whether the file is missing or merely filtered.
   */
  extensions?: string[];
  /** Multi-select. Defaults to on for `files`, off for `dir` and `save`. */
  multiple?: boolean;  /** `save` mode only: the name to pre-fill. */
  defaultName?: string;
  /**
   * `save` mode only: a full path whose folder becomes the start directory
   * and whose basename pre-fills the name box.
   *
   * Exists so a call site can pass `get_log_path()`'s result straight
   * through, exactly as the native `save({ defaultPath })` accepted it —
   * the wiring stays a substitution rather than a rewrite.
   */
  defaultPath?: string;
  /** Where to start. Falls back to the remembered directory. */
  startDir?: string | null;
}

/** Split a path into its folder and leaf, tolerating both separators. */
function splitPath(path: string): { dir: string | null; base: string } {
  const match = /^(.*)[\\/]([^\\/]*)$/.exec(path);
  if (!match) return { dir: null, base: path };
  // Keep the trailing separator on drive roots ("C:\") — dropping it turns
  // the root into the drive letter, which then fails to list.
  return { dir: match[1] || null, base: match[2] };
}

/** What the picker resolves to. `files` yields a list, the others a path. */
export type PickResult = string | string[] | null;
interface PendingPick {
  options: Required<Pick<PickerOptions, 'mode' | 'multiple'>> & PickerOptions;
  resolve: (value: PickResult) => void;
}

const open = ref(false);
const pending = ref<PendingPick | null>(null);

/**
 * Remembered across invocations so the second pick starts where the first
 * one ended. Cleared only when it points at something that no longer exists,
 * which the component discovers when the listing fails.
 */
const lastDir = ref<string | null>(null);

let current: PendingPick | null = null;

function begin(options: PickerOptions): Promise<PickResult> {
  // A second pick while one is open would orphan the first promise. Cancel
  // it instead of leaving a caller awaiting forever.
  if (current) {
    current.resolve(null);
    current = null;
  }

  // Normalise `defaultPath` here so the component only ever deals with a
  // start directory and a name.
  let { startDir = null, defaultName } = options;
  if (options.defaultPath) {
    const { dir, base } = splitPath(options.defaultPath);
    if (dir) startDir = dir;
    if (base && !defaultName) defaultName = base;
  }

  return new Promise<PickResult>((resolve) => {
    const entry: PendingPick = {
      options: {
        // Spread first, then the derived defaults — the other way round an
        // explicit `multiple: undefined` in the caller's object would win.
        ...options,
        // Defaults to false, matching the native plugin this replaces.
        // Several call sites test `typeof x === 'string'`; defaulting to
        // multi-select would hand them an array and break them silently.
        multiple: options.multiple ?? false,
        startDir,
        defaultName,
      },
      resolve: (value) => {
        // Any terminal path clears the slot so the next pick starts clean.
        if (current === entry) {
          current = null;
          pending.value = null;
          open.value = false;
        }
        resolve(value);
      },
    };
    current = entry;
    pending.value = entry;
    open.value = true;
  });
}

/** Read-only view for `FileBrowser.vue`. */
export function useFilePickerState() {
  return {
    open: readonly(open),
    pending: readonly(pending),
    lastDir: readonly(lastDir),
    setLastDir: (dir: string | null) => {
      lastDir.value = dir;
    },
    /** Settle the open picker. Safe to call when nothing is open. */
    finish: (value: PickResult) => {
      current?.resolve(value);
    },
  };
}

/**
 * Result type follows `multiple`, exactly as the native plugin's did:
 * multi-select yields a list, single-select a bare path, cancel `null`.
 * The overloads exist so call sites keep their existing `typeof x ===
 * 'string'` / `Array.isArray(x)` checks without a cast.
 */
export function pickFiles(
  options: Omit<PickerOptions, 'mode'> & { multiple: true },
): Promise<string[] | null>;
export function pickFiles(options?: Omit<PickerOptions, 'mode'>): Promise<string | null>;
export function pickFiles(options: Omit<PickerOptions, 'mode'> = {}) {
  return begin({ mode: 'files', ...options }) as Promise<string | string[] | null>;
}

export function pickDirectory(
  options: Omit<PickerOptions, 'mode'> & { multiple: true },
): Promise<string[] | null>;
export function pickDirectory(options?: Omit<PickerOptions, 'mode'>): Promise<string | null>;
export function pickDirectory(options: Omit<PickerOptions, 'mode'> = {}) {
  return begin({ mode: 'dir', ...options }) as Promise<string | string[] | null>;
}

export function pickSavePath(options: Omit<PickerOptions, 'mode'> = {}) {
  return begin({ mode: 'save', ...options }) as Promise<string | null>;
}
