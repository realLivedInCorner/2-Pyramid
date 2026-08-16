<script setup lang="ts"> 
 import { ref, onMounted, onUnmounted, computed } from "vue";
 import { invoke } from "@tauri-apps/api/core";
 import { getCurrentWindow } from "@tauri-apps/api/window";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
 import { isTauri as tauriIsAvailable } from "@tauri-apps/api/core";
import type { UnlistenFn } from "@tauri-apps/api/event";
 import { useI18n } from "vue-i18n";
 import HomePage from "./components/HomePage.vue";
 import ConversionPage from "./components/ConversionPage.vue";
 import OverlayPage from "./components/OverlayPage.vue";
 import SettingsPage from "./components/SettingsPage.vue";
 import StartupPage from "./components/StartupPage.vue";
 import UpdateDialog from "./components/UpdateDialog.vue";
 import NotificationToast from "./components/NotificationToast.vue";
 import { useUpdater, type UpdateCheckResult } from "./composables/useUpdater";
 import { useNotification } from "./composables/useNotification";

 const { t } = useI18n();
 const currentPage = ref<string>("home");
 const { setCurrentPage, setNotificationEnabled, setNotificationMode, setToastDuration } = useNotification();

 const showConversionGuard = ref(false);
 const sourceHandling = ref<'ask' | 'delete' | 'keep'>('ask');
 const openOutputAfterConvert = ref<boolean>(true);
 const showOOBE = ref(false);
  
 const pageComponent = computed(() => { 
   if (currentPage.value === "conversion") return ConversionPage; 
   if (currentPage.value === "settings") return SettingsPage; 
   if (currentPage.value === "overlay") return OverlayPage; 
   return HomePage; 
 }); 
  
 const pageProps = computed(() => {
   if (currentPage.value === "conversion") {
     return {
       sourceHandling: sourceHandling.value,
       openOutputAfterConvert: openOutputAfterConvert.value,
     };
   }
   if (currentPage.value === "settings") {
    return { animationStyle: animationStyle.value, devMode: devMode.value, userName: userName.value };
   }
   if (currentPage.value === "home") {
    return { userName: userName.value };
   }
   return {};
 }); 
  
 
 const animationStyle = ref<string>("fade-scale");
type AnimationSpeed = "slow" | "normal" | "fast";
const animationSpeed = ref<AnimationSpeed>("normal");
const devMode = ref<boolean>(false);
const userName = ref<string>("");
 
 const updateAnimationStyle = (value: string) => { 
   animationStyle.value = value; 
   localStorage.setItem("animationStyle", value); 
 }; 

 const updateAnimationSpeed = (value: AnimationSpeed) => {
   animationSpeed.value = value;
   localStorage.setItem("animationSpeed", value);
 }; 
  
 const applyThemeColor = (value: string) => { 
   if (!value) return; 
   document.documentElement.style.setProperty("--theme-color", value); 
   // Also set the RGB triplet for legacy rgba(var(--theme-color-rgb), ...) usages
   const hex = value.replace('#', '');
   if (hex.length === 6) {
     const r = parseInt(hex.substring(0, 2), 16);
     const g = parseInt(hex.substring(2, 4), 16);
     const b = parseInt(hex.substring(4, 6), 16);
     document.documentElement.style.setProperty("--theme-color-rgb", `${r}, ${g}, ${b}`);
   }
   localStorage.setItem("themeColor", value); 
 }; 
  
 const switchPage = (page: string) => { 
   currentPage.value = page;
   setCurrentPage(page);
 }; 

const updateDevMode = async (value: boolean) => {
  devMode.value = value;
  try {
    await invoke("set_dev_mode", { enabled: value });
  } catch (error) {
    console.error("Failed to update dev mode:", error);
  }
};

// ── Auto updater ──────────────────────────────────────
const { checkStartupUpdate, checkUpdateMarker } = useUpdater();

const showUpdateDialog = ref(false);
const updateResult = ref<UpdateCheckResult | null>(null);

const toastVisible = ref(false);
const toastPriority = ref<"safe" | "optional" | "success">("optional");
const toastVersion = ref("");
let toastTimer: ReturnType<typeof setTimeout> | null = null;

function showToast(priority: "safe" | "optional" | "success", version: string) {
  toastPriority.value = priority;
  toastVersion.value = version;
  toastVisible.value = true;
  if (toastTimer) clearTimeout(toastTimer);
  toastTimer = setTimeout(() => {
    toastVisible.value = false;
  }, 8000);
}

function dismissToast() {
  toastVisible.value = false;
  if (toastTimer) clearTimeout(toastTimer);
}

function openUpdateDialog() {
  dismissToast();
  showUpdateDialog.value = true;
}

function onSettingsCheckUpdate(result: UpdateCheckResult) {
  updateResult.value = result;
  showUpdateDialog.value = true;
}

async function doStartupCheck() {
  if (!tauriIsAvailable()) return;
  try {
    const r = await checkStartupUpdate();
    if (!r.hasUpdate || !r.result) return;
    updateResult.value = r.result;
    if (r.priority === "safe") {
      showUpdateDialog.value = true;
    } else {
      showToast("optional", r.result.latest?.version ?? "");
    }
  } catch { /* ignore startup check failures */ }
}

async function doMarkerCheck() {
  if (!tauriIsAvailable()) return;
  try {
    const newVer = await checkUpdateMarker();
    if (newVer) {
      showToast("success", newVer);
    }
  } catch { /* ignore */ }
}

 // Don't cache the Window instance across operations: in Tauri 2.x the
 // WebviewWindow handle can become stale and any cached reference
 // starts throwing “window not found” or “invalid handle” on the next
 // call. Fetch a fresh handle on every operation instead.
 type WindowHandle = {
   minimize: () => Promise<void>;
   maximize: () => Promise<void>;
   unmaximize: () => Promise<void>;
   isMaximized: () => Promise<boolean>;
   close: () => Promise<void>;
   hide: () => Promise<void>;
   show: () => Promise<void>;
   setFocus: () => Promise<void>;
 };

 function getWindow(): WindowHandle | null {
   if (!tauriIsAvailable()) return null;
   try {
     // Re-import lazily so a transient `getCurrentWindow` failure
     // (rare, e.g. before the runtime is ready) doesn’t poison the
     // module-level cache. `getCurrentWindow` is a cheap proxy.
     return getCurrentWindow() as unknown as WindowHandle;
   } catch (e) {
     console.warn('[window] getCurrentWindow failed:', e);
     return null;
   }
 }

 // Diagnostic state for window-control failures. When the user
 // reports “the X button stopped working”, we can read these counters
 // from DevTools to figure out which call actually failed without
 // re-running the whole flow.
 const windowDiag = {
   getCalls: 0,
   getOk: 0,
   getFail: 0,
   lastOp: '' as '' | 'min' | 'max' | 'close',
   lastError: '',
 };

 const minimizeWindow = async () => {
   windowDiag.lastOp = 'min';
   windowDiag.getCalls++;
   const w = getWindow();
   if (!w) { windowDiag.getFail++; return; }
   windowDiag.getOk++;
   try {
     await w.minimize();
   } catch (error) {
     windowDiag.lastError = String(error);
     console.error("[minimize] failed:", error, "diag=", windowDiag);
   }
 };

 const maximizeWindow = async () => {
   windowDiag.lastOp = 'max';
   windowDiag.getCalls++;
   const w = getWindow();
   if (!w) { windowDiag.getFail++; return; }
   windowDiag.getOk++;
   try {
     const isMaximized = await w.isMaximized();
     if (isMaximized) {
       await w.unmaximize();
     } else {
       await w.maximize();
     }
   } catch (error) {
     windowDiag.lastError = String(error);
     console.error("[maximize] failed:", error, "diag=", windowDiag);
   }
 };
  
const closeWindow = async () => {
    console.log('[closeWindow] checking for running conversion...');
    // Closing the window now always exits the app (the old
    // minimize-to-tray behavior is gone). The only guard we keep:
    // when a conversion is still running, warn the user that quitting
    // will interrupt it. The Rust side tracks the running state so it
    // survives page switches and component unmounts.
    let running = false;
    if (tauriIsAvailable()) {
      try {
        running = await invoke<boolean>('is_conversion_running');
      } catch (e) {
        console.warn('[closeWindow] is_conversion_running failed:', e);
      }
    }

    if (running) {
      showConversionGuard.value = true;
      return;
    }

    const wv = tauriIsAvailable() ? getCurrentWebviewWindow() : null;
    if (wv) {
      wv.close().catch((err) => {
        console.error('[closeWindow] close() request failed:', err);
      });
    }
  };

  /// User confirmed "quit anyway" on the conversion-in-progress guard.
  /// Closes the window for real; the running conversion dies with the
  /// process.
  const confirmCloseDuringConversion = () => {
    showConversionGuard.value = false;
    const wv = tauriIsAvailable() ? getCurrentWebviewWindow() : null;
    if (wv) {
      wv.close().catch((err) => {
        console.error('[confirmCloseDuringConversion] close() failed:', err);
      });
    }
  };
  
 let focusUnlisten: UnlistenFn | null = null;

onMounted(async () => {
  // (no per-mount window caching — see getWindow() above; we resolve
  //  a fresh handle on every operation.)

  // Subscribe to Tauri’s window focus event. When focus returns to the
  // window (e.g. after it was minimized), re-issue a setFocus so the
  // WebView re-evaluates its input target before the next user click
  // — without this the first click after a restore can be consumed by
  // the OS focusing the window instead of reaching the button.
  if (tauriIsAvailable()) {
    try {
      const w = getCurrentWindow();
      focusUnlisten = await w.onFocusChanged(async ({ payload: focused }) => {
        if (focused) {
          try { await w.setFocus(); } catch { /* ignore */ }
        }
      });
    } catch (e) {
      console.warn('[window] failed to subscribe to onFocusChanged:', e);
    }
  }
      const savedAnimationStyle = localStorage.getItem("animationStyle");
      if (savedAnimationStyle) {
       animationStyle.value = savedAnimationStyle;
     }

     const savedThemeColor = localStorage.getItem("themeColor"); 
     if (savedThemeColor) { 
       applyThemeColor(savedThemeColor); 
     }

     const savedAnimationSpeed = localStorage.getItem("animationSpeed");
     if (savedAnimationSpeed === "slow" || savedAnimationSpeed === "normal" || savedAnimationSpeed === "fast") {
       animationSpeed.value = savedAnimationSpeed;
     }
     
     // One-time cleanup: the close-action setting (ask / close /
     // minimize-to-tray) was removed from the app — purge any stale
     // localStorage value left over from older builds.
     localStorage.removeItem("closeAction");

     const savedSourceHandling = localStorage.getItem('sourceHandling');
     if (savedSourceHandling === 'delete' || savedSourceHandling === 'keep' || savedSourceHandling === 'ask') {
       sourceHandling.value = savedSourceHandling;
     }
     const savedOpenOutput = localStorage.getItem('openOutputAfterConvert');
     if (savedOpenOutput === 'true' || savedOpenOutput === 'false') {
       openOutputAfterConvert.value = savedOpenOutput === 'true';
     }
     
     try {
       const [cfg, dev] = await Promise.all([
         invoke<any>("get_config"),
         invoke<boolean>("get_dev_mode").catch(() => false),
       ]);
       if (cfg?.palette?.theme_color) {
         applyThemeColor(cfg.palette.theme_color);
       }
       if (typeof cfg?.notification_enabled === 'boolean') {
         setNotificationEnabled(cfg.notification_enabled);
       }
       if (cfg?.notification_mode === 'system' || cfg?.notification_mode === 'app' || cfg?.notification_mode === 'both') {
         setNotificationMode(cfg.notification_mode);
       }
       if (typeof cfg?.toast_duration_ms === 'number') {
         setToastDuration(cfg.toast_duration_ms);
       }
       if (cfg?.user_name) {
         userName.value = cfg.user_name;
       }
       if (!cfg?.initialized) {
         showOOBE.value = true;
       }
       devMode.value = dev;
     } catch {
       showOOBE.value = true;
     }
  
   document.addEventListener("contextmenu", (e) => {
     e.preventDefault();
   });

   // Ctrl+Shift+Q — guaranteed hard exit. Bypasses the window close
   // flow entirely (including the conversion guard) via app.exit(0).
   document.addEventListener("keydown", (e) => {
     if (e.ctrlKey && e.shiftKey && (e.key === 'Q' || e.key === 'q')) {
       e.preventDefault();
       if (tauriIsAvailable()) {
         invoke('force_quit').catch((err) => {
           console.error('[force_quit] invoke failed:', err);
         });
       } else {
         // Pure browser fallback (dev mode without Tauri) — shouldn't
         // normally be reachable but keeps the shortcut harmless.
         console.warn('[force_quit] Tauri not available, ignoring');
       }
     }
   });

  // Dismiss splash screen (minimum 1.2s display) — skip if OOBE is showing.
  const splash = document.getElementById('splash-screen');
  if (splash && !showOOBE.value) {
    const elapsed = Date.now() - Number((splash as HTMLElement).dataset.start || 0);
    const delay = Math.max(0, 1200 - elapsed);
    setTimeout(() => {
      splash.classList.add('splash-out');
      splash.addEventListener('transitionend', () => splash.remove(), { once: true });
    }, delay);
  }

  // Startup update check & post-update marker (deferred)
  setTimeout(() => doMarkerCheck(), 1800);
  setTimeout(() => doStartupCheck(), 2000);

  // Native (non-Vue) click listeners on the three window-control
  // buttons. If a click reaches the WebView but Vue's handler never
  // fires, the Vue app is stuck — this distinguishes "click never
  // arrives at the WebView" from "Vue event system broken".
  ['minimize', 'maximize', 'close'].forEach((id) => {
    const el = document.getElementById(id);
    if (!el) return;
    el.addEventListener('click', () => {
      console.log(`[native-click] #${id} received`);
    });
  });

  // Catch-all: any unhandled JS error would explain a “dead” UI.
  window.addEventListener('error', (ev) => {
    console.error('[global-error]', ev.message, ev.error);
  });
 });

 onUnmounted(() => {
  if (focusUnlisten) {
    try { focusUnlisten(); } catch { /* ignore */ }
    focusUnlisten = null;
  }
 });

 /// User clicked “Delete user profile” in Settings. The Rust side has
 /// already deleted settings.json; we just show the OOBE overlay so
 /// the reset is visible immediately (no app exit / reload needed).
 function onFactoryResetToOobe() {
   showOOBE.value = true;
 }

 async function onOOBEComplete() {
   showOOBE.value = false;
   // Reload config to pick up values saved during startup
   try {
     const cfg = await invoke<any>("get_config");
     if (cfg?.user_name) userName.value = cfg.user_name;
     if (cfg?.palette?.theme_color) applyThemeColor(cfg.palette.theme_color);
     if (cfg?.source_handling === 'ask' || cfg?.source_handling === 'delete' || cfg?.source_handling === 'keep') {
       sourceHandling.value = cfg.source_handling;
     }
     if (typeof cfg?.open_output_after_convert === 'boolean') {
       openOutputAfterConvert.value = cfg.open_output_after_convert;
     }
     if (typeof cfg?.toast_duration_ms === 'number') {
       setToastDuration(cfg.toast_duration_ms);
     }
   } catch { /* ignore */ }
   // Dismiss splash after OOBE finishes
   const splash = document.getElementById('splash-screen');
   if (splash) {
     splash.classList.add('splash-out');
     splash.addEventListener('transitionend', () => splash.remove(), { once: true });
   }
 }
 </script> 

 <template>
   <StartupPage v-if="showOOBE" @complete="onOOBEComplete" />
   <div class="app-container">
     <div class="drag-region" data-tauri-drag-region="true"></div> 
 
     <div class="floating-window-controls"> 
       <button class="window-button" id="minimize" @click="minimizeWindow" data-tauri-drag-region="false"> 
         <svg width="14" height="14" viewBox="0 0 14 14" fill="none"> 
           <line x1="3" y1="7" x2="11" y2="7" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/> 
         </svg> 
       </button> 
       <button class="window-button" id="maximize" @click="maximizeWindow" data-tauri-drag-region="false"> 
         <svg width="14" height="14" viewBox="0 0 14 14" fill="none"> 
           <path d="M4 3V4H3V11H4V12H11V11H12V4H11V3H4Z" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/> 
         </svg> 
       </button> 
       <button class="window-button close-button" id="close" @click="closeWindow" data-tauri-drag-region="false"> 
         <svg width="14" height="14" viewBox="0 0 14 14" fill="none"> 
           <path d="M3 3L11 11" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/> 
           <path d="M11 3L3 11" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/> 
         </svg> 
       </button> 
     </div> 
 
     <main class="main-content"> 
       <div class="page-content"> 
         <Transition :name="animationStyle" mode="out-in"> 
           <div class="page-shell" :key="currentPage" :data-anim-speed="animationSpeed"> 
             <component
               :is="pageComponent"
               v-bind="pageProps"
               @switch-page="switchPage"
               @update:animation-style="updateAnimationStyle"
               @update:animation-speed="updateAnimationSpeed"
               @update:dev-mode="updateDevMode"
               @update:user-name="(v: string) => userName = v"
               @update:source-handling="(v: 'ask' | 'delete' | 'keep') => sourceHandling = v"
               @update:open-output-after-convert="(v: boolean) => openOutputAfterConvert = v"
               @show-update-dialog="onSettingsCheckUpdate"
               @reset-to-oobe="onFactoryResetToOobe"
             />
           </div> 
         </Transition> 
       </div> 
     </main>

    <!-- Update toast -->
    <Transition name="toast-slide">
      <div v-if="toastVisible" class="toast-container">
        <div class="toast-card" :class="'toast-' + toastPriority" @click="toastPriority !== 'success' && openUpdateDialog()">
          <div class="toast-icon">
            <i v-if="toastPriority === 'safe'" class="ri-shield-check-line"></i>
            <i v-else-if="toastPriority === 'success'" class="ri-check-double-line"></i>
            <i v-else class="ri-information-line"></i>
          </div>
          <div class="toast-text">
            <div class="toast-title">
              <template v-if="toastPriority === 'safe'">{{ t('update.toast.safeUpdate') }}</template>
              <template v-else-if="toastPriority === 'success'">{{ t('update.toast.updateComplete') }}</template>
              <template v-else>{{ t('update.toast.newVersion') }}</template>
            </div>
            <div class="toast-desc">
              <template v-if="toastPriority === 'success'">{{ t('update.toast.updatedTo', { version: toastVersion }) }}</template>
              <template v-else>{{ t('update.toast.versionAvailable', { version: toastVersion }) }}</template>
            </div>
          </div>
          <button class="toast-close" @click.stop="dismissToast">
            <i class="ri-close-line"></i>
          </button>
        </div>
      </div>
    </Transition>

     <!-- Update dialog -->
     <UpdateDialog
       v-if="showUpdateDialog && updateResult"
       :updateResult="updateResult"
       @close="showUpdateDialog = false"
     />

     <!-- Notification toast -->
     <NotificationToast />

     <!-- Conversion-in-progress guard -->
     <Transition name="dialog-pop">
       <div v-if="showConversionGuard" class="conv-guard-overlay" @click="showConversionGuard = false">
         <div class="conv-guard dialog-content" @click.stop>
           <div class="conv-guard-header">
             <h3>{{ t('dialog.convRunningTitle') }}</h3>
           </div>
           <div class="conv-guard-body">
             <p>{{ t('dialog.convRunningBody') }}</p>
           </div>
           <div class="conv-guard-footer">
             <button class="conv-guard-btn secondary" @click="showConversionGuard = false">
               <i class="ri-loader-4-line"></i>
               {{ t('dialog.convRunningCancel') }}
             </button>
             <button class="conv-guard-btn primary" @click="confirmCloseDuringConversion">
               <i class="ri-close-line"></i>
               {{ t('dialog.convRunningConfirm') }}
             </button>
           </div>
         </div>
       </div>
     </Transition>
   </div>
 </template>
 
 <style> 
 
 * { 
   margin: 0; 
   padding: 0; 
   box-sizing: border-box; 
 } 
 
 html, body { 
   height: 100%; 
   font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", "PingFang SC", "Microsoft YaHei", Roboto, "Helvetica Neue", Arial, sans-serif;
   background-color: var(--bg-color); 
   color: var(--text-color); 
   line-height: 1.6; 
   transition: background-color 0.3s, color 0.3s; 
 } 
 
 #app { 
   height: 100%; 
 } 
 
 :root { 
   --theme-color: #007bff; 
   --bg-color: #ffffff; 
   --sidebar-bg: #f8f9fa; 
   --text-color: #212529; 
   --text-secondary: #6c757d; 
   --border-color: #e9ecef; 
   --hover-color: rgba(0, 0, 0, 0.05); 
   --primary-color: var(--theme-color); 
   --accent-color: var(--theme-color); 
   --success-color: var(--theme-color); 
   --warning-color: var(--theme-color); 
   --danger-color: var(--theme-color); 
   --shadow-light: 0 2px 4px rgba(0, 0, 0, 0.05); 
   --shadow-medium: 0 4px 8px rgba(0, 0, 0, 0.1); 
   --shadow-heavy: 0 8px 16px rgba(0, 0, 0, 0.15); 
   --radius-small: 6px; 
   --radius-medium: 12px; 
   --radius-large: 20px; 
   --radius-full: 50%; 
 } 
 
 
 .app-container { 
   display: flex; 
   flex-direction: column; 
  height: 100dvh; 
  width: 100%; 
   overflow: hidden; 
   background-color: var(--bg-color); 
 } 
 
 .drag-region { 
   position: fixed; 
   top: 0; 
   left: 0; 
   right: 0; 
   height: 50px; 
   z-index: 1001; 
   cursor: move; 
   -webkit-app-region: drag; 
   background: transparent; 
 } 
 
 .floating-window-controls { 
   position: fixed; 
   top: 10px; 
   right: 10px; 
   display: flex; 
   align-items: center; 
   justify-content: center; 
   gap: 8px; 
   z-index: 1002; 
   -webkit-app-region: no-drag; 
   backdrop-filter: blur(10px); 
   background-color: rgba(255, 255, 255, 0.1); 
   border-radius: var(--radius-medium); 
   padding: 4px; 
 } 
 
 
 .window-button { 
   width: 36px; 
   height: 36px; 
   border: none; 
   background: transparent; 
   color: var(--text-color); 
   display: flex; 
   align-items: center; 
   justify-content: center; 
   cursor: pointer; 
   border-radius: 6px; 
   transition: all 0.2s ease; 
   font-size: 14px; 
   -webkit-app-region: no-drag; 
   vertical-align: middle; 
   line-height: 36px; 
 } 
 
 .window-button:hover { 
   background-color: rgba(0, 0, 0, 0.1); 
   color: var(--primary-color); 
 } 
 
 .close-button:hover { 
   background-color: #ff4d4f !important; 
   color: white !important; 
 } 
 
 .main-content { 
   flex: 1; 
   display: flex; 
   overflow: auto; 
   padding: 0; 
   margin-top: 0; 
   box-sizing: border-box; 
  min-height: 0;
 } 
 
 .page-content {
   width: 100%;
   height: 100%;
   display: flex;
   flex-direction: column;
   overflow: hidden;
   position: relative;
 }
 
 .page-shell { 
   width: 100%; 
   height: 100%; 
   position: relative; 
 } 
 

 @media (max-width: 992px) { 
   .logo-text { 
     display: none; 
   } 
 
   .nav-text { 
     font-size: 13px; 
   } 
 
   .nav-item { 
     padding: 12px 16px; 
   } 
 } 
 
 @media (max-width: 768px) { 
   .nav-text { 
     display: none; 
   } 
 
   .nav-item { 
     padding: 12px; 
   } 
 
   .main-content { 
     padding: 16px; 
   } 
 
   .page-content { 
     padding: 16px; 
   } 
 } 
 
 @media (max-width: 480px) { 
   .nav-menu { 
     gap: 1px; 
   } 
 
   .nav-icon { 
     font-size: 16px; 
   } 
 
   .theme-toggle { 
     padding: 8px; 
   } 
 
   .toggle-text { 
     display: none; 
   } 
 } 
 
 ::-webkit-scrollbar { 
   width: 8px; 
   height: 8px; 
 } 
 
 ::-webkit-scrollbar-track { 
   background: var(--bg-color); 
 } 
 
 ::-webkit-scrollbar-thumb { 
   background: var(--border-color); 
   border-radius: var(--radius-small); 
   transition: background 0.3s; 
 } 
 
 ::-webkit-scrollbar-thumb:hover { 
   background: var(--text-secondary); 
 } 
 
 button { 
   font-family: inherit; 
   font-size: inherit; 
   color: inherit; 
   background: none; 
   border: none; 
   cursor: pointer; 
   transition: all 0.3s ease; 
 } 
 
 input, select, textarea { 
   font-family: inherit; 
   font-size: inherit; 
   color: inherit; 
   border: 1px solid var(--border-color); 
   border-radius: var(--radius-small); 
   padding: 10px 14px; 
   background-color: var(--bg-color); 
   transition: all 0.3s ease; 
 } 
 
 input:focus, select:focus, textarea:focus { 
   outline: none; 
   border-color: var(--primary-color); 
   box-shadow: 0 0 0 2px rgba(0, 123, 255, 0.25); 
 } 
 

 .card { 
   background-color: var(--bg-color); 
   border-radius: var(--radius-medium); 
   border: 1px solid var(--border-color); 
   box-shadow: var(--shadow-light); 
   padding: 24px; 
   margin-bottom: 20px; 
   transition: all 0.3s ease; 
 } 
 
 .card:hover { 
   box-shadow: var(--shadow-medium); 
   transform: translateY(-2px); 
 } 
 
 @keyframes fadeIn { 
   from { 
     opacity: 0; 
     transform: translateY(10px); 
   } 
   to { 
     opacity: 1; 
     transform: translateY(0); 
   } 
 } 
 
 @keyframes slideIn { 
   from { 
     transform: translateX(-100%); 
   } 
   to { 
     transform: translateX(0); 
   } 
 } 
 
 @keyframes pulse { 
   0% { 
     transform: scale(1); 
   } 
   50% { 
     transform: scale(1.05); 
   } 
   100% { 
     transform: scale(1); 
   } 
 } 
 
 @keyframes floatUp { 
   0% { 
     opacity: 0; 
     transform: translateY(20px) scale(0.95); 
   } 
   50% { 
     transform: translateY(-5px) scale(1.02); 
   } 
   100% { 
     opacity: 1; 
     transform: translateY(0) scale(1); 
   } 
 } 
 
 @keyframes ripple { 
   0% { 
     transform: scale(0); 
     opacity: 1; 
   } 
   100% { 
     transform: scale(4); 
     opacity: 0; 
   } 
 } 
 
 .fadeIn { 
   animation: fadeIn 0.5s ease-out; 
 } 
 
 .slideIn { 
   animation: slideIn 0.3s ease-out; 
 } 
 
 .pulse { 
   animation: pulse 1.5s infinite; 
 } 
 
 .fade-enter-active, 
 .fade-leave-active { 
   transition: opacity 0.5s ease-in-out; 
   position: relative; 
   top: 0; 
   left: 0; 
   width: 100%; 
   height: 100%; 
 } 
 
 .fade-enter-from { 
   opacity: 0; 
 } 
 
 .fade-leave-to { 
   opacity: 0; 
 } 
 
 .fade-scale-enter-active,
 .fade-scale-leave-active {
   transition: opacity 0.32s cubic-bezier(0.2, 0.8, 0.2, 1);
   position: absolute;
   top: 0;
   left: 0;
   width: 100%;
   height: 100%;
 }

 .fade-scale-enter-active {
   z-index: 2;
 }

 .fade-scale-leave-active {
   z-index: 1;
 }

 .fade-scale-enter-from {
   opacity: 0;
 }

 .fade-scale-leave-to {
   opacity: 0;
 }

 .fade-scale-enter-to {
   opacity: 1;
 }

 .fade-scale-leave-from {
   opacity: 1;
 }

 /*
  * Page-entry stagger — replaces the old whole-page fade/scale wipe so
  * each top-level block of the freshly-mounted page animates in one after
  * another (top-to-bottom) instead of the whole page appearing as one
  * block. The outer fade-scale transition above still handles the cross-page
  * swap, this only governs the children of the freshly-mounted page.
  *
  * IMPORTANT — selector shape: `.page-shell > *` would target the entire
  * page root component (e.g. `.fanhua-home` for HomePage, `.conversion-page`
  * for ConversionPage, etc.), which is a SINGLE element per page-shell. That
  * would (a) leave nth-child(2..10) matching nothing, and (b) hide the whole
  * page since that single root would receive `opacity: 0` and only animate
  * in after the longest delay. Use the two-level `.page-shell > * > *`
  * selector so we stagger the page root's DIRECT children (header, main,
  * dock-wrap, …) instead of the root itself.
  *
  * Because Vue's <Transition :key="currentPage"> mode="out-in" mounts a
  * fresh .page-shell on every page swap, the CSS `animation` below plays
  * from frame 1 every time — no JS hook needed.
  *
  * Speed is controlled by the `data-anim-speed` attribute on .page-shell,
  * which is bound from App.vue's animationSpeed state (slow/normal/fast).
  * The selector `[data-anim-speed="fast"] > * > *` overrides the default
  * delays for that speed; default (no attribute or "normal") uses the
  * base values below.
  */
 .page-shell > * > *:not(.dialog-overlay):not(.sidebar-overlay):not(.sidebar-content):not(.conv-guard-overlay) {
   opacity: 0;
   animation: stagger-rise 0.5s cubic-bezier(0.2, 0.8, 0.2, 1) forwards;
 }
 .page-shell > * > *:nth-child(1):not(.dialog-overlay):not(.sidebar-overlay):not(.sidebar-content):not(.conv-guard-overlay)  { animation-delay: 0.18s; }
 .page-shell > * > *:nth-child(2):not(.dialog-overlay):not(.sidebar-overlay):not(.sidebar-content):not(.conv-guard-overlay)  { animation-delay: 0.26s; }
 .page-shell > * > *:nth-child(3):not(.dialog-overlay):not(.sidebar-overlay):not(.sidebar-content):not(.conv-guard-overlay)  { animation-delay: 0.34s; }
 .page-shell > * > *:nth-child(4):not(.dialog-overlay):not(.sidebar-overlay):not(.sidebar-content):not(.conv-guard-overlay)  { animation-delay: 0.42s; }
 .page-shell > * > *:nth-child(5):not(.dialog-overlay):not(.sidebar-overlay):not(.sidebar-content):not(.conv-guard-overlay)  { animation-delay: 0.50s; }
 .page-shell > * > *:nth-child(6):not(.dialog-overlay):not(.sidebar-overlay):not(.sidebar-content):not(.conv-guard-overlay)  { animation-delay: 0.58s; }
 .page-shell > * > *:nth-child(7):not(.dialog-overlay):not(.sidebar-overlay):not(.sidebar-content):not(.conv-guard-overlay)  { animation-delay: 0.66s; }
 .page-shell > * > *:nth-child(8):not(.dialog-overlay):not(.sidebar-overlay):not(.sidebar-content):not(.conv-guard-overlay)  { animation-delay: 0.74s; }
 .page-shell > * > *:nth-child(9):not(.dialog-overlay):not(.sidebar-overlay):not(.sidebar-content):not(.conv-guard-overlay)  { animation-delay: 0.82s; }
 .page-shell > * > *:nth-child(10):not(.dialog-overlay):not(.sidebar-overlay):not(.sidebar-content):not(.conv-guard-overlay) { animation-delay: 0.90s; }

 /* Slow mode: 1.5× duration, +60ms between siblings, longer start delay */
 .page-shell[data-anim-speed="slow"] > * > *:not(.dialog-overlay):not(.sidebar-overlay):not(.sidebar-content):not(.conv-guard-overlay) {
   animation-duration: 0.75s;
 }
 .page-shell[data-anim-speed="slow"] > * > *:nth-child(1):not(.dialog-overlay):not(.sidebar-overlay):not(.sidebar-content):not(.conv-guard-overlay)  { animation-delay: 0.25s; }
 .page-shell[data-anim-speed="slow"] > * > *:nth-child(2):not(.dialog-overlay):not(.sidebar-overlay):not(.sidebar-content):not(.conv-guard-overlay)  { animation-delay: 0.40s; }
 .page-shell[data-anim-speed="slow"] > * > *:nth-child(3):not(.dialog-overlay):not(.sidebar-overlay):not(.sidebar-content):not(.conv-guard-overlay)  { animation-delay: 0.55s; }
 .page-shell[data-anim-speed="slow"] > * > *:nth-child(4):not(.dialog-overlay):not(.sidebar-overlay):not(.sidebar-content):not(.conv-guard-overlay)  { animation-delay: 0.70s; }
 .page-shell[data-anim-speed="slow"] > * > *:nth-child(5):not(.dialog-overlay):not(.sidebar-overlay):not(.sidebar-content):not(.conv-guard-overlay)  { animation-delay: 0.85s; }
 .page-shell[data-anim-speed="slow"] > * > *:nth-child(6):not(.dialog-overlay):not(.sidebar-overlay):not(.sidebar-content):not(.conv-guard-overlay)  { animation-delay: 1.00s; }
 .page-shell[data-anim-speed="slow"] > * > *:nth-child(7):not(.dialog-overlay):not(.sidebar-overlay):not(.sidebar-content):not(.conv-guard-overlay)  { animation-delay: 1.15s; }
 .page-shell[data-anim-speed="slow"] > * > *:nth-child(8):not(.dialog-overlay):not(.sidebar-overlay):not(.sidebar-content):not(.conv-guard-overlay)  { animation-delay: 1.30s; }
 .page-shell[data-anim-speed="slow"] > * > *:nth-child(9):not(.dialog-overlay):not(.sidebar-overlay):not(.sidebar-content):not(.conv-guard-overlay)  { animation-delay: 1.45s; }
 .page-shell[data-anim-speed="slow"] > * > *:nth-child(10):not(.dialog-overlay):not(.sidebar-overlay):not(.sidebar-content):not(.conv-guard-overlay) { animation-delay: 1.60s; }

 /* Fast mode: 0.6× duration, -60ms between siblings, near-zero start delay */
 .page-shell[data-anim-speed="fast"] > * > *:not(.dialog-overlay):not(.sidebar-overlay):not(.sidebar-content):not(.conv-guard-overlay) {
   animation-duration: 0.3s;
 }
 .page-shell[data-anim-speed="fast"] > * > *:nth-child(1):not(.dialog-overlay):not(.sidebar-overlay):not(.sidebar-content):not(.conv-guard-overlay)  { animation-delay: 0.05s; }
 .page-shell[data-anim-speed="fast"] > * > *:nth-child(2):not(.dialog-overlay):not(.sidebar-overlay):not(.sidebar-content):not(.conv-guard-overlay)  { animation-delay: 0.10s; }
 .page-shell[data-anim-speed="fast"] > * > *:nth-child(3):not(.dialog-overlay):not(.sidebar-overlay):not(.sidebar-content):not(.conv-guard-overlay)  { animation-delay: 0.15s; }
 .page-shell[data-anim-speed="fast"] > * > *:nth-child(4):not(.dialog-overlay):not(.sidebar-overlay):not(.sidebar-content):not(.conv-guard-overlay)  { animation-delay: 0.20s; }
 .page-shell[data-anim-speed="fast"] > * > *:nth-child(5):not(.dialog-overlay):not(.sidebar-overlay):not(.sidebar-content):not(.conv-guard-overlay)  { animation-delay: 0.25s; }
 .page-shell[data-anim-speed="fast"] > * > *:nth-child(6):not(.dialog-overlay):not(.sidebar-overlay):not(.sidebar-content):not(.conv-guard-overlay)  { animation-delay: 0.30s; }
 .page-shell[data-anim-speed="fast"] > * > *:nth-child(7):not(.dialog-overlay):not(.sidebar-overlay):not(.sidebar-content):not(.conv-guard-overlay)  { animation-delay: 0.35s; }
 .page-shell[data-anim-speed="fast"] > * > *:nth-child(8):not(.dialog-overlay):not(.sidebar-overlay):not(.sidebar-content):not(.conv-guard-overlay)  { animation-delay: 0.40s; }
 .page-shell[data-anim-speed="fast"] > * > *:nth-child(9):not(.dialog-overlay):not(.sidebar-overlay):not(.sidebar-content):not(.conv-guard-overlay)  { animation-delay: 0.45s; }
 .page-shell[data-anim-speed="fast"] > * > *:nth-child(10):not(.dialog-overlay):not(.sidebar-overlay):not(.sidebar-content):not(.conv-guard-overlay) { animation-delay: 0.50s; }

 @keyframes stagger-rise {
   from {
     opacity: 0;
     transform: translateY(10px);
   }
   to {
     opacity: 1;
     transform: translateY(0);
   }
 } 
 
 .slide-enter-active, 
 .slide-leave-active { 
   transition: all 0.6s cubic-bezier(0.4, 0, 0.2, 1); 
   position: absolute; 
   top: 0; 
   left: 0; 
   width: 100%; 
   height: 100%; 
 } 
 
 .slide-enter-active { 
   z-index: 2; 
 } 
 
 .slide-leave-active { 
   z-index: 1; 
 } 
 
 .slide-enter-from { 
   opacity: 0; 
   transform: translateX(100px); 
 } 
 
 .slide-leave-to { 
   opacity: 0; 
   transform: translateX(-100px); 
 } 
 
 .slide-enter-to { 
   opacity: 1; 
   transform: translateX(0); 
 } 
 
 .slide-leave-from { 
   opacity: 1; 
   transform: translateX(0); 
 } 
 
 .page-turn-enter-active, 
 .page-turn-leave-active { 
   transition: all 0.8s cubic-bezier(0.2, 0.8, 0.2, 1); 
   position: absolute; 
   top: 0; 
   left: 0; 
   width: 100%; 
   height: 100%; 
   backface-visibility: hidden; 
 } 
 
 .page-turn-enter-active { 
   z-index: 2; 
 } 
 
 .page-turn-leave-active { 
   z-index: 1; 
 } 
 
 .page-turn-enter-from { 
   opacity: 0; 
   transform: rotateY(-90deg) translateZ(100px); 
 } 
 
 .page-turn-leave-to { 
   opacity: 0; 
   transform: rotateY(90deg) translateZ(-100px); 
 } 
 
 .page-turn-enter-to { 
   opacity: 1; 
   transform: rotateY(0deg) translateZ(0); 
 } 
 
 .page-turn-leave-from { 
   opacity: 1; 
   transform: rotateY(0deg) translateZ(0); 
 } 
 
 .float-ripple-btn { 
   position: relative; 
   overflow: hidden; 
   transition: all 0.3s ease; 
 } 
 
 .float-ripple-btn:hover { 
   transform: translateY(-3px); 
   box-shadow: 0 8px 16px rgba(0, 0, 0, 0.15); 
 } 
 
 .float-ripple-btn:active { 
   transform: translateY(-1px); 
 } 
 
 .float-ripple-btn::after { 
   content: ''; 
   position: absolute; 
   top: 50%; 
   left: 50%; 
   width: 5px; 
   height: 5px; 
   background: rgba(255, 255, 255, 0.5); 
   opacity: 0; 
   border-radius: 100%; 
   transform: scale(1, 1) translate(-50%, -50%); 
   transform-origin: 50% 50%; 
 } 
 
 .float-ripple-btn:focus:not(:active)::after { 
   animation: ripple 1s ease-out; 
 } 
 
 .float-ripple-btn:focus { 
   outline: none; 
 } 
 
 .loading { 
   display: inline-block; 
   width: 20px; 
   height: 20px; 
   border: 2px solid var(--border-color); 
   border-top: 2px solid var(--primary-color); 
   border-radius: var(--radius-full); 
   animation: spin 1s linear infinite; 
 } 
 
 @keyframes spin { 
   0% { transform: rotate(0deg); } 
   100% { transform: rotate(360deg); } 
 } 
 
 .tooltip { 
   position: relative; 
   cursor: help; 
 } 
 
 .tooltip::after { 
   content: attr(data-tooltip); 
   position: absolute; 
   bottom: 125%; 
   left: 50%; 
   transform: translateX(-50%); 
   background-color: rgba(0, 0, 0, 0.8); 
   color: white; 
   padding: 6px 10px; 
   border-radius: var(--radius-small); 
   font-size: 12px; 
   white-space: nowrap; 
   opacity: 0; 
   visibility: hidden; 
   transition: all 0.3s ease; 
   z-index: 1000; 
 } 
 
 .tooltip:hover::after { 
   opacity: 1; 
   visibility: visible; 
 } 
 
 @media (max-width: 768px) { 
   html { 
     font-size: 14px; 
   } 
 } 
 
 @media (max-width: 480px) { 
   html { 
     font-size: 13px; 
   } 
 } 
 /* ── Update toast ────────────────────────────── */
.toast-container {
  position: fixed; bottom: 24px; right: 24px; z-index: 3000;
}
.toast-card {
  display: flex; align-items: center; gap: 12px;
  padding: 14px 18px; border-radius: 16px;
  background: #fff; box-shadow: 0 12px 36px rgba(0, 0, 0, 0.15);
  border: 1px solid rgba(0, 0, 0, 0.06);
  cursor: pointer; max-width: 380px;
  transition: all 0.2s;
}
.toast-card:hover {
  transform: translateY(-2px);
  box-shadow: 0 16px 40px rgba(0, 0, 0, 0.2);
}
.toast-icon {
  width: 36px; height: 36px; border-radius: 50%;
  display: flex; align-items: center; justify-content: center;
  font-size: 18px; flex-shrink: 0;
}
.toast-safe .toast-icon { background: #fef2f2; color: #dc2626; }
.toast-optional .toast-icon { background: color-mix(in srgb, var(--theme-color) 10%, transparent); color: var(--theme-color); }
.toast-success .toast-icon { background: #ecfdf5; color: #10b981; }
.toast-success { cursor: default; }
.toast-success:hover { transform: none; box-shadow: 0 12px 36px rgba(0, 0, 0, 0.15); }
.toast-text { flex: 1; min-width: 0; }
.toast-title { font-size: 14px; font-weight: 700; color: #1d1d1f; }
.toast-desc { font-size: 12px; color: #6b7280; margin-top: 2px; }
.toast-close {
  width: 28px; height: 28px; border-radius: 50%; border: none;
  background: rgba(0, 0, 0, 0.05); color: #9ca3af; cursor: pointer;
  display: flex; align-items: center; justify-content: center;
  font-size: 14px; flex-shrink: 0; transition: 0.2s;
}
.toast-close:hover { background: rgba(0, 0, 0, 0.1); color: #1d1d1f; }

.toast-slide-enter-active { transition: all 0.35s cubic-bezier(0.2, 0.8, 0.2, 1); }
.toast-slide-leave-active { transition: all 0.25s ease-in; }
.toast-slide-enter-from { opacity: 0; transform: translateX(40px); }
.toast-slide-leave-to { opacity: 0; transform: translateX(40px); }

/* ── Conversion-in-progress guard dialog ────────────── */
.conv-guard-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 9999;
  backdrop-filter: blur(4px);
}

.conv-guard {
  background: #fff;
  border-radius: 16px;
  padding: 24px;
  width: 400px;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.2);
}

.conv-guard-header h3 {
  font-size: 18px;
  font-weight: 700;
  color: #1d1d1f;
  margin: 0;
}

.conv-guard-body {
  margin: 16px 0;
}

.conv-guard-body p {
  font-size: 14px;
  color: #6c757d;
  margin: 0;
  line-height: 1.6;
}

.conv-guard-footer {
  display: flex;
  gap: 12px;
}

.conv-guard-btn {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 12px 16px;
  border: none;
  border-radius: 12px;
  font-size: 14px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s;
}

.conv-guard-btn.primary {
  background: #1d1d1f;
  color: #fff;
}

.conv-guard-btn.primary:hover {
  background: #000;
}

.conv-guard-btn.secondary {
  background: #f1f5f9;
  color: #475569;
}

.conv-guard-btn.secondary:hover {
  background: #e2e8f0;
}

.dialog-pop-enter-active,
.dialog-pop-leave-active {
  transition: opacity 0.2s ease, transform 0.2s ease;
}

.dialog-pop-enter-from,
.dialog-pop-leave-to {
  opacity: 0;
  transform: scale(0.95);
}

/* Faster sibling of dialog-pop, used by small quick dialogs
   (OverlayPage create/import, SettingsPage compact dialogs). */
.dialog-pop-quick-enter-active {
  transition: opacity 0.15s ease, transform 0.15s ease;
}
.dialog-pop-quick-leave-active {
  transition: opacity 0.12s ease, transform 0.12s ease;
}
.dialog-pop-quick-enter-from,
.dialog-pop-quick-leave-to {
  opacity: 0;
  transform: scale(0.96);
}

/* dialog-pop-fast: snappy enter, slow leave (ConversionPage version
   picker needs 700ms leave so the inner version-card spring can play
   out before the overlay disappears). */
.dialog-pop-fast-enter-active {
  transition: opacity 0.15s ease, transform 0.15s ease;
}
.dialog-pop-fast-leave-active {
  transition: opacity 0.7s ease, transform 0.7s ease;
}
.dialog-pop-fast-enter-from,
.dialog-pop-fast-leave-to {
  opacity: 0;
  transform: scale(0.95);
}

/* OverlayPage header status toast: slides down from the top, fades out. */
.header-status-toast-enter-active {
  transition: all 0.3s cubic-bezier(0.2, 0.8, 0.2, 1);
}
.header-status-toast-leave-active {
  transition: all 0.25s ease-in;
}
.header-status-toast-enter-from {
  opacity: 0;
  transform: translateY(-16px);
}
.header-status-toast-leave-to {
  opacity: 0;
  transform: translateY(-8px);
}

/* ConversionPage version-picker sidebar overlay: plain opacity fade. */
.sidebar-overlay-fade-enter-active,
.sidebar-overlay-fade-leave-active {
  transition: opacity 0.25s ease;
}
.sidebar-overlay-fade-enter-from,
.sidebar-overlay-fade-leave-to {
  opacity: 0;
}
</style>
