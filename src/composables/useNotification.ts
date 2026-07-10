import { ref, readonly } from 'vue';
import { sendNotification } from '@tauri-apps/plugin-notification';
import { invoke } from '@tauri-apps/api/core';

export type NotificationType = 'info' | 'success' | 'error' | 'warning';
export type NotificationSource = 'conversion' | 'overlay' | 'update' | 'system';
export type NotificationMode = 'system' | 'app' | 'both';

export interface NotificationOptions {
  title: string;
  body: string;
  type?: NotificationType;
  source?: NotificationSource;
  silent?: boolean;
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
      silent = false
    } = options;

    await logNotification(type, title, body);

    const showSystem = notificationMode.value === 'system' || notificationMode.value === 'both';
    const showApp = notificationMode.value === 'app' || notificationMode.value === 'both';

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

    if (showApp) {
      const item: NotificationItem = {
        id: nextId++,
        title,
        body,
        type,
        source,
        timestamp: Date.now()
      };

      notifications.value.push(item);

      queue.push(item);
      if (!isVisible.value) {
        processQueue();
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

  const dismiss = () => {
    dismissCurrent();
  };

  const clearAll = () => {
    queue = [];
    dismissCurrent();
    notifications.value = [];
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
    clearAll
  };
}