import { ref, readonly } from 'vue';
import { sendNotification } from '@tauri-apps/plugin-notification';
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
const notificationMode = ref<NotificationMode>('both');
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

export function useNotification() {
  const notify = async (options: NotificationOptions) => {
    if (!notificationEnabled.value) return;

    const {
      title,
      body,
      type = 'info',
      source = 'system',
      silent = false,
      actions = []
    } = options;

    await logNotification(type, title, body);

    const showSystem = notificationMode.value === 'system' || notificationMode.value === 'both';
    const showApp = notificationMode.value === 'app' || notificationMode.value === 'both';

    // Desktop top-level toast (independent always-on-top window,
    // upper-right of primary monitor, stacks downward). This is the
    // primary path — preferred over the OS notification center because
    // (a) it carries our branding / iconography, (b) it stays on
    // screen long enough to be read and supports click-to-dismiss, and
    // (c) Win 11 folds Action Center toasts into a tray menu the user
    // has to actively open.
    if (showApp && !silent) {
      // Wire up the toast-action listener before firing any toast
      // that carries actions, but NEVER let a hung listener block the
      // notify — otherwise the success toast would never appear
      // (observed: conversion-complete feedback silently missing).
      if (actions.length > 0) {
        await Promise.race([
          ensureToastActionListener(),
          new Promise((resolve) => setTimeout(resolve, 500)),
        ]);
      }
      try {
        await invoke('show_toast', {
          payload: {
            title,
            body,
            kind: type,
            // Long enough to actually read the text. Toasts carrying
            // action buttons stay even longer so the user has time to
            // notice and click them.
            durationMs: actions.length > 0 ? 10000 : 8000,
            actions,
          },
        });
      } catch (e) {
        console.warn('Top-level toast failed, falling back to in-app:', e);
        // Fallback: queue the in-app toast so the user still gets
        // visual feedback even if the desktop-level window failed.
        const item: NotificationItem = {
          id: nextId++,
          title,
          body,
          type,
          source,
          timestamp: Date.now(),
        };
        notifications.value.push(item);
        queue.push(item);
        if (!isVisible.value) {
          processQueue();
        }
      }
    }

    // OS notification center (Windows Action Center / macOS banner).
    // Only used when the user picked `system` or `both` mode and the
    // toast isn't explicitly silenced.
    if (showSystem && !silent) {
      try {
        await sendNotification({
          title: `2-Pyramid - ${title}`,
          body
        });
      } catch (e) {
        console.warn('Windows notification failed:', e);
      }
    }

    // In-app toast (the legacy queue). Kept as an additional channel
    // when the user explicitly opts in to `app` mode via the legacy
    // SettingsPage option. The desktop toast above already covers the
    // most common case, so this only fires when the legacy
    // notification mode says `app` *and* the desktop toast is somehow
    // unavailable (e.g. unsupported platform).
    if (showApp && silent === false) {
      // (The desktop-toast path above already covers the common case;
      //  this branch intentionally stays empty so we don't double-fire
      //  in-app toasts on every notify() call.)
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
    dismiss,
    clearAll,
    registerToastAction,
    unregisterToastAction,
  };
}