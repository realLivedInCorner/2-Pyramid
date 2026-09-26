import { ref, readonly } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export type NotificationType = 'info' | 'success' | 'error' | 'warning';
export type NotificationSource = 'conversion' | 'overlay' | 'update' | 'system';
export type NotificationMode = 'system' | 'app' | 'both';

/**
 * Action button descriptor attached to a toast. The toast page renders
 * one button per entry; clicking it closes the toast, fires
 * `run_toast_action(id)` on the Rust side, and Rust then emits a
 * `toast-action` event the main window listens for via
 * `registerToastAction`.
 */
export interface ToastAction {
  /** Stable identifier routed back to the registered handler. */
  id: string;
  /** Button label, e.g. "打开目录" / "Open". */
  label: string;
  /** Optional Remix Icon class for visual affordance. */
  icon?: string;
}

export interface NotificationOptions {
  title: string;
  body: string;
  type?: NotificationType;
  source?: NotificationSource;
  silent?: boolean;
  /**
   * 忽略「通知已关闭」开关强制发送。供设置页的「测试通知」按钮使用：
   * 测试按钮的目的是预览效果，即使用户关闭了通知也应当能看到结果，
   * 否则点击毫无反馈（此前就是静默返回导致“无法使用”）。
   */
  ignoreDisabled?: boolean;
  /**
   * Optional action buttons rendered inside the toast. Clicking one
   * closes the toast and fires the corresponding handler registered
   * via `registerToastAction`.
   */
  actions?: ToastAction[];
}

interface NotificationItem {
  id: number;
  title: string;
  body: string;
  type: NotificationType;
  source: NotificationSource;
  timestamp: number;
}

const notifications = ref<NotificationItem[]>([]);
const isVisible = ref(false);
const currentNotification = ref<NotificationItem | null>(null);
const currentPage = ref<string>('home');
const notificationEnabled = ref(true);
const notificationMode = ref<NotificationMode>('app');
// Desktop toast auto-dismiss duration (ms). Default 8000; synced from
// the user's settings by App.vue via `setToastDuration`.
const toastDuration = ref(8000);
let nextId = 0;
let showTimer: ReturnType<typeof setTimeout> | null = null;
let queue: NotificationItem[] = [];

/**
 * Action click registry. Each entry maps an action id to a handler
 * that runs in the main window when the user clicks the corresponding
 * button on a toast. Handlers are registered by any caller via
 * `registerToastAction` and torn down via `unregisterToastAction`.
 */
const toastActionHandlers = new Map<string, () => void | Promise<void>>();

/**
 * Lazily-initialised global listener for the `toast-action` event
 * emitted by Rust when a user clicks an action button. We install it
 * once on first call into `notify()` so the registration cost is paid
 * at most once per app session.
 */
let toastActionListener: UnlistenFn | null = null;
async function ensureToastActionListener(): Promise<void> {
  if (toastActionListener) return;
  toastActionListener = await listen<string>('toast-action', (event) => {
    const handler = toastActionHandlers.get(event.payload);
    if (handler) {
      Promise.resolve()
        .then(() => handler())
        .catch((e) => console.warn('toast action handler failed:', e));
    } else {
      console.warn('toast action clicked but no handler registered for id=', event.payload);
    }
  });
}

const NOTIFICATION_DURATION = 4500;
const ANIMATION_DURATION = 350;

function processQueue() {
  if (queue.length === 0) {
    isVisible.value = false;
    currentNotification.value = null;
    return;
  }

  const next = queue.shift()!;
  currentNotification.value = next;
  isVisible.value = true;

  if (showTimer) clearTimeout(showTimer);
  showTimer = setTimeout(() => {
    dismissCurrent();
  }, NOTIFICATION_DURATION);
}

function dismissCurrent() {
  isVisible.value = false;
  if (showTimer) {
    clearTimeout(showTimer);
    showTimer = null;
  }

  setTimeout(() => {
    processQueue();
  }, ANIMATION_DURATION);
}

async function logNotification(type: string, title: string, body: string) {
  try {
    await invoke('log_notification', { notificationType: type, title, body });
  } catch (e) {
    console.warn('Failed to log notification:', e);
  }
}

/**
 * Desktop always-on-top toast window (primary in-app channel).
 * Returns false when the Rust command failed so callers can fall back
 * to the legacy in-app overlay queue.
 */
async function fireDesktopToast(opts: {
  title: string;
  body: string;
  type: NotificationType;
  actions: ToastAction[];
  durationMs: number;
}): Promise<boolean> {
  if (opts.actions.length > 0) {
    await Promise.race([
      ensureToastActionListener(),
      new Promise((resolve) => setTimeout(resolve, 500)),
    ]);
  }
  try {
    await invoke('show_toast', {
      payload: {
        title: opts.title,
        body: opts.body,
        kind: opts.type,
        // Rust accepts duration_ms (alias durationMs). Send only one key.
        duration_ms: opts.durationMs,
        actions: opts.actions,
      },
    });
    return true;
  } catch (e) {
    console.warn('Top-level toast failed:', e);
    return false;
  }
}

function queueInAppFallback(opts: {
  title: string;
  body: string;
  type: NotificationType;
  source: NotificationSource;
}) {
  const item: NotificationItem = {
    id: nextId++,
    title: opts.title,
    body: opts.body,
    type: opts.type,
    source: opts.source,
    timestamp: Date.now(),
  };
  notifications.value.push(item);
  queue.push(item);
  if (!isVisible.value) {
    processQueue();
  }
}

/**
 * OS notification via the Rust notification plugin.
 *
 * IMPORTANT: do NOT use `@tauri-apps/plugin-notification`'s
 * `sendNotification` / `requestPermission` on this project — that JS
 * API currently shells out to `window.Notification` inside WebView2,
 * which on Windows often silently does nothing. `show_system_notification`
 * goes through notify-rust / winrt instead.
 */
async function fireSystemNotification(title: string, body: string): Promise<boolean> {
  try {
    await invoke('show_system_notification', {
      title: `2-Pyramid - ${title}`,
      body,
    });
    return true;
  } catch (e) {
    console.warn('System notification failed (Rust plugin path):', e);
    // Last-ditch: plugin JS path (may still work if WebView Notification
    // permission is granted on some environments).
    try {
      await invoke('plugin:notification|notify', {
        options: { title: `2-Pyramid - ${title}`, body },
      });
      return true;
    } catch (e2) {
      console.warn('System notification failed (plugin:notify):', e2);
      return false;
    }
  }
}

export function useNotification() {
  const notify = async (options: NotificationOptions) => {
    const {
      title,
      body,
      type = 'info',
      source = 'system',
      silent = false,
      ignoreDisabled = false,
      actions = []
    } = options;

    // 测试通知（ignoreDisabled）等显式请求除外：通知关闭时静默跳过
    if (!notificationEnabled.value && !ignoreDisabled) return;

    await logNotification(type, title, body);

    const mode = notificationMode.value;
    const wantSystem = mode === 'system' || mode === 'both';
    const wantApp = mode === 'app' || mode === 'both';
    const durationMs =
      actions.length > 0 ? Math.max(toastDuration.value, 10000) : toastDuration.value;

    let desktopOk = false;
    let systemOk = false;

    // Strict channel split — never cross-fallback into the other style.
    //   app    → custom desktop toast window only (toast.html on the monitor)
    //   system → OS notification only
    //   both   → fire both
    // In-app overlay is a last resort ONLY when the selected channel itself fails.

    // App-styled channel: custom-drawn toast window ON THE DESKTOP/MONITOR.
    if (wantApp && !silent) {
      desktopOk = await fireDesktopToast({ title, body, type, actions, durationMs });
    }

    // System channel: Windows / OS toast.
    if (wantSystem && !silent) {
      systemOk = await fireSystemNotification(title, body);
    }

    if (silent) {
      return;
    }

    // Per-channel last resort: if the channel the user asked for failed,
    // show the in-app overlay. Do NOT call the other channel (that is
    // how "app only" was leaking Windows toasts).
    const appFailed = wantApp && !desktopOk;
    const systemFailed = wantSystem && !systemOk;
    if (appFailed || systemFailed) {
      if (!desktopOk && !systemOk) {
        console.warn('Selected toast channel failed, using in-app overlay fallback');
        queueInAppFallback({ title, body, type, source });
      }
    }
  };

  const setCurrentPage = (page: string) => {
    currentPage.value = page;
  };

  const setNotificationEnabled = (enabled: boolean) => {
    notificationEnabled.value = enabled;
  };

  const setNotificationMode = (mode: NotificationMode) => {
    notificationMode.value = mode;
  };

  const setToastDuration = (ms: number) => {
    if (Number.isFinite(ms) && ms >= 4000 && ms <= 15000) {
      toastDuration.value = ms;
    }
  };

  const dismiss = () => {
    dismissCurrent();
  };

  const clearAll = () => {
    queue = [];
    dismissCurrent();
    notifications.value = [];
  };

  /**
   * Register a handler that runs in the main window when the user
   * clicks a toast action button with the matching id. Returns an
   * unregister function for cleanup (call it from onUnmounted in
   * components that register handlers).
   */
  const registerToastAction = (id: string, handler: () => void | Promise<void>): (() => void) => {
    toastActionHandlers.set(id, handler);
    // Make sure the global listener is wired so the click can route
    // here. Fire-and-forget; the listener is idempotent.
    void ensureToastActionListener();
    return () => {
      // Only delete if the registered handler is still *our* handler
      // (avoids races where the caller swapped in a newer one).
      if (toastActionHandlers.get(id) === handler) {
        toastActionHandlers.delete(id);
      }
    };
  };

  const unregisterToastAction = (id: string): void => {
    toastActionHandlers.delete(id);
  };

  return {
    notifications: readonly(notifications),
    isVisible: readonly(isVisible),
    currentNotification: readonly(currentNotification),
    notify,
    setCurrentPage,
    setNotificationEnabled,
    setNotificationMode,
    setToastDuration,
    dismiss,
    clearAll,
    registerToastAction,
    unregisterToastAction,
  };
}
