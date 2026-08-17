<script setup lang="ts">
import { onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { getCurrentWindow } from "@tauri-apps/api/window";

const dir = ref("");
const version = ref("2.0.0");
const busy = ref(false);
const state = ref<"idle" | "installing" | "done" | "error">("idle");
const message = ref("");
const installed = ref(false);

onMounted(async () => {
  try {
    dir.value = await invoke<string>("get_default_dir");
    version.value = await invoke<string>("get_version");
    installed.value = await invoke<boolean>("is_installed");
  } catch (e) {
    message.value = String(e);
  }
});

const browse = async () => {
  try {
    const selected = await open({ directory: true, multiple: false });
    if (selected && typeof selected === "string") {
      dir.value = selected;
    }
  } catch { /* ignore */ }
};

const doInstall = async () => {
  if (!dir.value.trim() || busy.value) return;
  busy.value = true;
  state.value = "installing";
  message.value = "";
  try {
    const result = await invoke<string>("install", { dir: dir.value.trim() });
    message.value = result;
    state.value = "done";
    installed.value = true;
  } catch (e) {
    message.value = String(e);
    state.value = "error";
  } finally {
    busy.value = false;
  }
};

const doLaunch = async () => {
  try {
    await invoke("launch_app", { dir: dir.value.trim() });
    closeWindow();
  } catch (e) {
    message.value = String(e);
  }
};

const doUninstall = async () => {
  if (busy.value) return;
  busy.value = true;
  state.value = "installing";
  try {
    const result = await invoke<string>("uninstall", { dir: dir.value.trim() });
    message.value = result;
    state.value = "done";
    installed.value = false;
  } catch (e) {
    message.value = String(e);
    state.value = "error";
  } finally {
    busy.value = false;
  }
};

const closeWindow = async () => {
  try {
    await getCurrentWindow().close();
  } catch { /* ignore */ }
};
</script>

<template>
  <div class="installer-root">
    <div class="drag-region" data-tauri-drag-region="true"></div>
    <button class="window-close" @click="closeWindow" aria-label="关闭">×</button>

    <header class="brand">
      <svg class="logo" viewBox="0 0 680 680" fill="none" xmlns="http://www.w3.org/2000/svg">
        <g stroke="currentColor" stroke-linecap="round">
          <line x1="260" y1="520" x2="200" y2="390" stroke-width="7"/>
          <line x1="260" y1="520" x2="500" y2="190" stroke-width="7"/>
          <line x1="200" y1="390" x2="500" y2="190" stroke-width="7"/>
          <line x1="260" y1="520" x2="374" y2="274" stroke-width="5"/>
        </g>
      </svg>
      <div class="brand-text">
        <h1>2-Pyramid</h1>
        <p>安装器 · v{{ version }}</p>
      </div>
    </header>

    <main class="content">
      <div class="card">
        <label class="field-label">安装位置</label>
        <div class="field-row">
          <input v-model="dir" class="dir-input" spellcheck="false" />
          <button class="btn ghost" @click="browse">浏览</button>
        </div>
        <p class="hint">默认安装到当前用户目录，无需管理员权限。</p>

        <button
          class="btn primary install-btn"
          :disabled="busy || !dir.trim()"
          @click="doInstall"
        >
          <i v-if="state === 'installing'" class="ri-loader-4-line ri-spin" aria-hidden="true"></i>
          {{ state === 'installing' ? '安装中…' : '安装 2-Pyramid' }}
        </button>

        <div v-if="state === 'done'" class="status ok">
          <i class="ri-checkbox-circle-line" aria-hidden="true"></i>
          <span>{{ message }}</span>
          <div class="status-actions">
            <button class="btn primary small" @click="doLaunch">启动 2-Pyramid</button>
            <button class="btn ghost small" @click="closeWindow">关闭</button>
          </div>
        </div>
        <div v-else-if="state === 'error'" class="status err">
          <i class="ri-error-warning-line" aria-hidden="true"></i>
          <span>{{ message }}</span>
        </div>
      </div>

      <div class="card uninstall-card" v-if="installed && state !== 'done'">
        <p class="hint">检测到已安装的 2-Pyramid。如需移除（用户数据保留）：</p>
        <button class="btn danger" :disabled="busy" @click="doUninstall">卸载</button>
      </div>
    </main>

    <footer class="foot">2-Pyramid Studio · 便携释放式安装器</footer>
  </div>
</template>

<style>
* { margin: 0; padding: 0; box-sizing: border-box; }

html, body, #app {
  height: 100%;
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", "PingFang SC", "Microsoft YaHei", sans-serif;
}

.installer-root {
  height: 100%;
  display: flex;
  flex-direction: column;
  background: linear-gradient(165deg, #f6f8fd 0%, #eef2fb 50%, #e8edf9 100%);
  color: #1a1a2e;
  overflow: hidden;
  position: relative;
}

.drag-region {
  position: fixed;
  top: 0; left: 0; right: 0;
  height: 32px;
  -webkit-app-region: drag;
}

.window-close {
  position: fixed;
  top: 8px; right: 12px;
  width: 30px; height: 30px;
  border: none;
  border-radius: 8px;
  background: transparent;
  color: #64748b;
  font-size: 18px;
  cursor: pointer;
  -webkit-app-region: no-drag;
}
.window-close:hover { background: rgba(0, 0, 0, 0.07); color: #0f172a; }

.brand {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 40px 36px 0;
}

.logo {
  width: 52px;
  height: 52px;
  color: #007bff;
}

.brand-text h1 { font-size: 22px; font-weight: 800; letter-spacing: -0.4px; }
.brand-text p { font-size: 12.5px; color: #94a3b8; margin-top: 2px; }

.content {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 14px;
  padding: 24px 36px;
  overflow-y: auto;
}

.card {
  background: rgba(255, 255, 255, 0.8);
  border: 1px solid rgba(255, 255, 255, 0.9);
  border-radius: 16px;
  padding: 20px;
  box-shadow: 0 8px 26px rgba(15, 23, 42, 0.06);
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.field-label { font-size: 13px; font-weight: 700; color: #1a1a2e; }

.field-row { display: flex; gap: 10px; }

.dir-input {
  flex: 1;
  padding: 11px 14px;
  font-size: 13.5px;
  font-family: inherit;
  border: 1.5px solid rgba(0, 0, 0, 0.1);
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.9);
  color: #1a1a2e;
  outline: none;
}
.dir-input:focus { border-color: #007bff; }

.btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 11px 20px;
  font-size: 14px;
  font-weight: 700;
  font-family: inherit;
  border: none;
  border-radius: 10px;
  cursor: pointer;
  transition: all 0.2s ease;
}
.btn:disabled { opacity: 0.5; cursor: not-allowed; }

.btn.primary { background: #007bff; color: #fff; box-shadow: 0 4px 14px rgba(0, 123, 255, 0.35); }
.btn.primary:hover:not(:disabled) { transform: translateY(-1px); }

.btn.ghost { background: rgba(0, 0, 0, 0.05); color: #475569; }
.btn.ghost:hover:not(:disabled) { background: rgba(0, 0, 0, 0.08); }

.btn.danger { background: rgba(239, 68, 68, 0.1); color: #dc2626; }
.btn.danger:hover:not(:disabled) { background: rgba(239, 68, 68, 0.18); }

.btn.small { padding: 8px 14px; font-size: 13px; }

.install-btn { width: 100%; padding: 13px; font-size: 15px; }

.install-btn .ri-spin { font-size: 16px; }

.hint { font-size: 12px; color: #94a3b8; line-height: 1.5; }

.status {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 14px;
  border-radius: 12px;
  font-size: 13.5px;
  font-weight: 600;
}
.status.ok { background: #ecfdf5; color: #15803d; }
.status.ok i { font-size: 26px; }
.status.err { background: #fef2f2; color: #b91c1c; }

.status-actions { display: flex; gap: 10px; margin-top: 4px; }

.uninstall-card { align-items: flex-start; }

.foot {
  padding: 14px 36px 20px;
  font-size: 11.5px;
  color: #b0b7c3;
  text-align: center;
}
</style>
