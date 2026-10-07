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
  /** `files` mode only. */
  multiple?: boolean;
  /** `save` mode only: the name to pre-fill. */
  defaultName?: string;
  /** Where to start. Falls back to the remembered directory. */
  startDir?: string | null;
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

  return new Promise<PickResult>((resolve) => {
    const entry: PendingPick = {
      options: {
        multiple: options.mode === 'files',
        ...options,
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

export function pickFiles(options: Omit<PickerOptions, 'mode'> = {}) {
  return begin({ mode: 'files', ...options });
}

export function pickDirectory(options: Omit<PickerOptions, 'mode'> = {}) {
  return begin({ mode: 'dir', ...options });
}

export function pickSavePath(options: Omit<PickerOptions, 'mode'> = {}) {
  return begin({ mode: 'save', ...options });
}
