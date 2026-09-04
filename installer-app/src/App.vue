<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import { getCurrentWindow } from "@tauri-apps/api/window";

// 流程步骤：0 介绍 / 1 安装位置 / 2 安装中 / 3 完成
const step = ref(0);
const totalSteps = 4;

const uninstallMode = ref(false);
const dir = ref("");
const version = ref("2.0.0");
const channel = ref("stable");
const githubUrl = ref("");
const busy = ref(false);
const failed = ref(false);
const resultMessage = ref("");
const installed = ref(false);

// 快捷方式选项：桌面 / 开始菜单
const shortcutDesktop = ref(true);
const shortcutStartMenu = ref(true);

// 安装进度
const progressCurrent = ref(0);
const progressTotal = ref(0);
const progressName = ref("");
const progressPercent = ref(0);
let unlistenProgress: UnlistenFn | null = null;

onMounted(async () => {
  try {
    uninstallMode.value = await invoke<boolean>("is_uninstall_mode");
    dir.value = uninstallMode.value
      ? ((await invoke<string | null>("get_installed_dir")) ?? await invoke<string>("get_default_dir"))
      : await invoke<string>("get_default_dir");
    version.value = await invoke<string>("get_version");
    channel.value = await invoke<string>("get_channel");
    githubUrl.value = await invoke<string>("get_github_url");
    installed.value = await invoke<boolean>("is_installed");
  } catch (e) {
    console.error("[installer] init failed:", e);
  }
  if (!uninstallMode.value) {
    unlistenProgress = await listen<{ current: number; total: number; name: string }>(
      "install-progress",
      (event) => {
        progressCurrent.value = event.payload.current;
        progressTotal.value = event.payload.total;
        progressName.value = event.payload.name;
        progressPercent.value = event.payload.total > 0
          ? Math.round((event.payload.current / event.payload.total) * 100)
          : 0;
      },
    );
  }
});

onUnmounted(() => {
  if (unlistenProgress) {
    unlistenProgress();
    unlistenProgress = null;
  }
});

const browse = async () => {
  try {
    const selected = await open({ directory: true, multiple: false });
    if (selected && typeof selected === "string") dir.value = selected;
  } catch { /* ignore */ }
};

const openGithub = async () => {
  try {
    if (githubUrl.value) await openUrl(githubUrl.value);
  } catch (e) {
    console.error("[installer] open github failed:", e);
  }
};

const next = () => {
  if (step.value < totalSteps - 1) step.value++;
};

const prev = () => {
  if (step.value > 0) step.value--;
};

const doInstall = async () => {
  if (!dir.value.trim() || busy.value) return;
  busy.value = true;
  failed.value = false;
  progressCurrent.value = 0;
  progressTotal.value = 0;
  progressName.value = "";
  progressPercent.value = 0;
  step.value = 2;
  try {
    resultMessage.value = await invoke<string>("install", {
      dir: dir.value.trim(),
      shortcuts: {
        desktop: shortcutDesktop.value,
        startMenu: shortcutStartMenu.value,
      },
    });
    installed.value = true;
    step.value = 3;
  } catch (e) {
    failed.value = true;
    resultMessage.value = String(e);
  } finally {
    busy.value = false;
  }
};

const doUninstall = async () => {
  if (busy.value) return;
  busy.value = true;
  failed.value = false;
  try {
    resultMessage.value = await invoke<string>("uninstall", { dir: dir.value.trim() });
    installed.value = false;
    step.value = 3;
    // 完成动画播完后自动关窗：窗口关闭 → 进程退出 → 后台清理进程
    // （WaitForExit 等待本进程）删除卸载器自身与安装目录。
    window.setTimeout(() => { void closeWindow(); }, 3500);
  } catch (e) {
    failed.value = true;
    resultMessage.value = String(e);
  } finally {
    busy.value = false;
  }
};

const doLaunch = async () => {
  try {
    await invoke("launch_app", { dir: dir.value.trim() });
    closeWindow();
  } catch (e) {
    failed.value = true;
    resultMessage.value = String(e);
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

    <!-- 顶部品牌 -->
    <header class="top-brand">
      <svg class="logo" viewBox="0 0 680 680" fill="none" xmlns="http://www.w3.org/2000/svg">
        <g stroke="currentColor" stroke-linecap="round">
          <line x1="260" y1="520" x2="200" y2="390" stroke-width="7"/>
          <line x1="260" y1="520" x2="500" y2="190" stroke-width="7"/>
          <line x1="200" y1="390" x2="500" y2="190" stroke-width="7"/>
          <line x1="260" y1="520" x2="374" y2="274" stroke-width="5"/>
        </g>
      </svg>
      <div class="top-brand-text">
        <span class="top-brand-name">2-Pyramid</span>
        <span class="top-brand-tag">
          {{ uninstallMode ? '卸载程序' : '安装程序' }} · v{{ version }}
          <em v-if="channel === 'beta'" class="beta-badge">Beta</em>
        </span>
      </div>
    </header>

    <!-- 步骤指示器（卸载模式隐藏） -->
    <div class="steps" v-if="!uninstallMode">
      <template v-for="i in totalSteps" :key="i">
        <span v-if="i > 1" class="step-line" :class="{ done: i - 2 < step }"></span>
        <span class="step-dot" :class="{ active: i - 1 === step, done: i - 1 < step }">
          <i v-if="i - 1 < step" class="ri-check-line"></i>
          <template v-else>{{ i }}</template>
        </span>
      </template>
    </div>

    <!-- 内容区 -->
    <main class="content">
      <!-- 安装模式：步骤 0 介绍 -->
      <div v-if="!uninstallMode && step === 0" class="panel">
        <div class="panel-title">欢迎使用 2-Pyramid</div>
        <p class="panel-desc">
          2-Pyramid 是一款多版本 Minecraft 资源包转换工具，
          支持将传统资源包一键转换为任意 Minecraft 版本。
        </p>
        <div class="feature-row">
          <div class="feature">
            <i class="ri-swap-box-line" aria-hidden="true"></i>
            <b>一键转换</b>
            <span>26 个 Minecraft 版本目标</span>
          </div>
          <div class="feature">
            <i class="ri-palette-line" aria-hidden="true"></i>
            <b>深度定制</b>
            <span>背景、主题色、控件皮肤</span>
          </div>
          <div class="feature">
            <i class="ri-stack-line" aria-hidden="true"></i>
            <b>覆盖包系统</b>
            <span>叠加任意母包之上</span>
          </div>
        </div>
        <div class="github-card">
          <i class="ri-github-fill" aria-hidden="true"></i>
          <div class="github-text">
            <b>开源项目</b>
            <span>2-Pyramid 完全开源，欢迎 Star / Issue / PR</span>
          </div>
          <button class="btn ghost" @click="openGithub">访问 GitHub</button>
        </div>
      </div>

      <!-- 安装模式：步骤 1 安装位置 -->
      <div v-else-if="!uninstallMode && step === 1" class="panel">
        <div class="panel-title">选择安装位置</div>
        <p class="panel-desc">默认安装到当前用户目录，无需管理员权限。</p>
        <label class="field-label">安装目录</label>
        <div class="field-row">
          <input v-model="dir" class="dir-input" spellcheck="false" />
          <button class="btn ghost" @click="browse"><i class="ri-folder-open-line"></i> 浏览</button>
        </div>
        <p v-if="installed" class="hint align-hint">
          <i class="ri-information-line" aria-hidden="true"></i>
          已检测到安装的 2-Pyramid，安装目录已自动对齐到现有位置。
        </p>

        <label class="field-label">快捷方式</label>
        <div class="shortcut-options">
          <label class="check-row">
            <input type="checkbox" v-model="shortcutDesktop" />
            <i class="ri-computer-line" aria-hidden="true"></i>
            <span>创建桌面快捷方式</span>
          </label>
          <label class="check-row">
            <input type="checkbox" v-model="shortcutStartMenu" />
            <i class="ri-menu-line" aria-hidden="true"></i>
            <span>加入开始菜单</span>
          </label>
        </div>
        <p class="hint">
          安装内容：主程序、转换引擎与内置资源（约几十 MB）。
          卸载时用户数据（转换记录、设置）将保留。
        </p>
      </div>

      <!-- 安装模式：步骤 2 安装中 -->
      <div v-else-if="!uninstallMode && step === 2" class="panel">
        <div class="panel-title">正在安装</div>
        <div class="progress-wrap">
          <div class="progress-track">
            <div class="progress-fill" :style="{ width: progressPercent + '%' }"></div>
          </div>
          <div class="progress-meta">
            <span>{{ progressPercent }}%</span>
            <span class="progress-file">{{ progressName }}</span>
          </div>
        </div>
        <div v-if="failed" class="status err">
          <i class="ri-error-warning-line" aria-hidden="true"></i>
          <span>{{ resultMessage }}</span>
        </div>
      </div>

      <!-- 完成（安装/卸载共用） -->
      <div v-else-if="step === 3" class="panel">
        <div class="panel-title">{{ failed ? '出错了' : (uninstallMode ? '卸载完成' : '安装完成') }}</div>
        <div :class="failed ? 'status err' : 'status ok'">
          <i :class="failed ? 'ri-error-warning-line' : 'ri-checkbox-circle-line'" aria-hidden="true"></i>
          <span>{{ resultMessage }}</span>
        </div>
        <div v-if="!uninstallMode && !failed" class="status-actions">
          <button class="btn primary" @click="doLaunch"><i class="ri-rocket-2-line"></i> 启动 2-Pyramid</button>
          <button class="btn ghost" @click="closeWindow">关闭</button>
        </div>
        <div v-else-if="failed" class="status-actions">
          <button v-if="step === 2" class="btn ghost" @click="prev">返回重试</button>
          <button class="btn ghost" @click="closeWindow">关闭</button>
        </div>
        <div v-else class="status-actions">
          <button class="btn primary" @click="closeWindow"><i class="ri-check-line"></i> 完成</button>
        </div>
      </div>

      <!-- 卸载模式主面板 -->
      <div v-else class="panel">
        <div class="panel-title">卸载 {{ channel === 'beta' ? '2-Pyramid Beta' : '2-Pyramid' }}</div>
        <p class="panel-desc">
          将从以下位置移除全部程序文件与快捷方式。
          用户数据（转换记录、设置、背景等）将被保留。
        </p>
        <label class="field-label">安装目录</label>
        <div class="field-row">
          <input v-model="dir" class="dir-input" spellcheck="false" readonly />
        </div>
        <div v-if="busy" class="status working">
          <i class="ri-loader-4-line ri-spin" aria-hidden="true"></i>
          <span>正在卸载，请稍候…</span>
        </div>
        <div v-else-if="failed" class="status err">
          <i class="ri-error-warning-line" aria-hidden="true"></i>
          <span>{{ resultMessage }}</span>
        </div>
        <p v-else class="hint">点击下方「卸载」开始。完成后窗口会自动关闭，卸载程序随后自行清理残留。</p>
      </div>
    </main>

    <!-- 底部药丸导航 -->
    <footer class="foot">
      <div class="nav-pill">
        <button
          v-if="!uninstallMode && step > 0 && step < 3"
          class="pill-btn ghost"
          :disabled="busy"
          @click="prev"
        ><i class="ri-arrow-left-s-line"></i> 上一步</button>

        <button
          v-if="!uninstallMode && step === 0"
          class="pill-btn primary"
          @click="next"
        >下一步 <i class="ri-arrow-right-s-line"></i></button>

        <button
          v-else-if="!uninstallMode && step === 1"
          class="pill-btn primary"
          :disabled="busy || !dir.trim()"
          @click="doInstall"
        ><i class="ri-install-line"></i> 安装</button>

        <button
          v-else-if="!uninstallMode && step === 2"
          class="pill-btn primary"
          disabled
        ><i class="ri-loader-4-line ri-spin"></i> 安装中…</button>

        <button
          v-else-if="uninstallMode && step !== 3"
          class="pill-btn danger"
          :disabled="busy || !installed"
          @click="doUninstall"
        ><i :class="busy ? 'ri-loader-4-line ri-spin' : 'ri-delete-bin-line'"></i> {{ busy ? '卸载中…' : '卸载' }}</button>
      </div>
    </footer>
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
  background: linear-gradient(165deg, #f6f8fd 0%, #eef2fb 55%, #e8edf9 100%);
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
  top: 10px; right: 14px;
  width: 32px; height: 32px;
  border: none;
  border-radius: 9px;
  background: transparent;
  color: #64748b;
  font-size: 18px;
  cursor: pointer;
  -webkit-app-region: no-drag;
  transition: all 0.15s;
}
.window-close:hover { background: rgba(239, 68, 68, 0.12); color: #dc2626; }

/* 顶部品牌 */
.top-brand {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 28px 40px 0;
}

.logo { width: 42px; height: 42px; color: #007bff; }

.top-brand-text { display: flex; flex-direction: column; gap: 1px; }
.top-brand-name { font-size: 17px; font-weight: 800; letter-spacing: -0.4px; }
.top-brand-tag { font-size: 12px; color: #94a3b8; }

.beta-badge {
  font-style: normal;
  font-size: 10.5px;
  font-weight: 800;
  padding: 1px 7px;
  margin-left: 6px;
  border-radius: 999px;
  background: rgba(249, 115, 22, 0.14);
  color: #ea580c;
  vertical-align: 1px;
}

/* 步骤指示器 */
.steps {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 22px 24px 0;
}

.step-dot {
  width: 28px; height: 28px;
  border-radius: 50%;
  display: flex; align-items: center; justify-content: center;
  font-size: 12.5px; font-weight: 700;
  background: rgba(0, 0, 0, 0.07);
  color: #94a3b8;
  transition: all 0.3s cubic-bezier(0.22, 1, 0.36, 1);
}
.step-dot i { font-size: 14px; }
.step-dot.active {
  background: #007bff;
  color: #fff;
  box-shadow: 0 2px 10px rgba(0, 123, 255, 0.4);
}
.step-dot.done { background: rgba(0, 123, 255, 0.18); color: #007bff; }

.step-line {
  width: 34px; height: 2px;
  border-radius: 1px;
  background: rgba(0, 0, 0, 0.09);
  transition: background 0.3s ease;
}
.step-line.done { background: rgba(0, 123, 255, 0.45); }

/* 内容 */
.content {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 20px 48px;
  min-height: 0;
  overflow-y: auto;
}

.panel {
  width: 100%;
  max-width: 640px;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 14px;
  text-align: center;
}

.panel-title { font-size: 26px; font-weight: 800; letter-spacing: -0.5px; color: #111827; }
.panel-desc { font-size: 14px; color: #6b7280; line-height: 1.7; max-width: 480px; }

.feature-row { display: flex; gap: 14px; width: 100%; margin-top: 8px; }

.feature {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  padding: 22px 16px;
  background: rgba(255, 255, 255, 0.8);
  border: 1px solid rgba(255, 255, 255, 0.9);
  border-radius: 16px;
  box-shadow: 0 8px 24px rgba(15, 23, 42, 0.05);
  backdrop-filter: blur(18px);
  -webkit-backdrop-filter: blur(18px);
}
.feature i { font-size: 26px; color: #007bff; }
.feature b { font-size: 14px; }
.feature span { font-size: 12px; color: #94a3b8; }

.github-card {
  width: 100%;
  display: flex;
  align-items: center;
  gap: 14px;
  margin-top: 8px;
  padding: 16px 18px;
  background: rgba(255, 255, 255, 0.8);
  border: 1px solid rgba(255, 255, 255, 0.9);
  border-radius: 16px;
  box-shadow: 0 8px 24px rgba(15, 23, 42, 0.05);
  backdrop-filter: blur(18px);
  -webkit-backdrop-filter: blur(18px);
  text-align: left;
}
.github-card > i { font-size: 30px; color: #1a1a2e; }
.github-text { flex: 1; display: flex; flex-direction: column; gap: 2px; }
.github-text b { font-size: 14px; }
.github-text span { font-size: 12.5px; color: #94a3b8; }

.field-label { width: 100%; text-align: left; font-size: 13px; font-weight: 700; }

.field-row { display: flex; gap: 10px; width: 100%; }

.shortcut-options {
  width: 100%;
  display: flex;
  flex-direction: column;
  gap: 9px;
  padding: 14px 16px;
  border: 1.5px dashed rgba(0, 0, 0, 0.12);
  border-radius: 12px;
  background: rgba(255, 255, 255, 0.55);
}

.check-row {
  display: flex;
  align-items: center;
  gap: 9px;
  font-size: 13.5px;
  color: #334155;
  cursor: pointer;
  user-select: none;
}
.check-row input {
  width: 16px;
  height: 16px;
  accent-color: #007bff;
  cursor: pointer;
  margin: 0;
}
.check-row i { font-size: 15px; color: #007bff; }

.align-hint {
  display: flex;
  align-items: center;
  gap: 6px;
  color: #007bff;
  font-weight: 700;
}

.dir-input {
  flex: 1;
  padding: 12px 15px;
  font-size: 13.5px;
  font-family: inherit;
  border: 1.5px solid rgba(0, 0, 0, 0.1);
  border-radius: 11px;
  background: rgba(255, 255, 255, 0.9);
  color: #1a1a2e;
  outline: none;
}
.dir-input:focus { border-color: #007bff; }

.hint { width: 100%; text-align: left; font-size: 12.5px; color: #94a3b8; line-height: 1.6; }

/* 进度 */
.progress-wrap { width: 100%; display: flex; flex-direction: column; gap: 10px; margin-top: 10px; }

.progress-track {
  width: 100%; height: 12px;
  border-radius: 999px;
  background: rgba(0, 0, 0, 0.06);
  overflow: hidden;
}

.progress-fill {
  height: 100%;
  border-radius: 999px;
  background: linear-gradient(90deg, #007bff, #60a5fa);
  transition: width 0.25s ease;
}

.progress-meta {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 13px;
  font-weight: 700;
  color: #007bff;
}

.progress-file {
  flex: 1;
  margin-left: 16px;
  font-weight: 500;
  font-size: 12px;
  color: #94a3b8;
  text-align: right;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* 状态 */
.status {
  width: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 18px;
  border-radius: 14px;
  font-size: 14px;
  font-weight: 600;
}
.status.ok { background: #ecfdf5; color: #15803d; }
.status.ok i { font-size: 30px; animation: status-pop 0.6s cubic-bezier(0.34, 1.56, 0.64, 1); }
.status.err { background: #fef2f2; color: #b91c1c; }
.status.err i { font-size: 30px; }
.status.working { background: #eff6ff; color: #2563eb; }
.status.working i { font-size: 30px; }
.status span { white-space: pre-line; line-height: 1.6; }

@keyframes status-pop {
  0% { transform: scale(0); opacity: 0; }
  60% { transform: scale(1.25); opacity: 1; }
  100% { transform: scale(1); opacity: 1; }
}

.status-actions { display: flex; gap: 12px; margin-top: 6px; }

/* 按钮 */
.btn {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 10px 18px;
  font-size: 13.5px;
  font-weight: 700;
  font-family: inherit;
  border: none;
  border-radius: 10px;
  cursor: pointer;
  transition: all 0.2s cubic-bezier(0.34, 1.56, 0.64, 1);
}
.btn:disabled { opacity: 0.5; cursor: not-allowed; }
.btn.primary { background: #007bff; color: #fff; box-shadow: 0 4px 14px rgba(0, 123, 255, 0.35); }
.btn.primary:hover:not(:disabled) { transform: scale(1.04); }
.btn.ghost { background: rgba(0, 0, 0, 0.05); color: #475569; }
.btn.ghost:hover:not(:disabled) { background: rgba(0, 0, 0, 0.09); }

/* 底部药丸 */
.foot {
  display: flex;
  justify-content: center;
  padding: 8px 24px 26px;
}

.nav-pill {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 7px;
  background: rgba(255, 255, 255, 0.85);
  border: 1px solid rgba(255, 255, 255, 0.95);
  border-radius: 999px;
  box-shadow: 0 10px 30px rgba(15, 23, 42, 0.1);
  backdrop-filter: blur(18px);
  -webkit-backdrop-filter: blur(18px);
  transition: transform 0.35s cubic-bezier(0.34, 1.56, 0.64, 1), box-shadow 0.35s ease;
}
.nav-pill:hover {
  transform: translateY(-2px);
  box-shadow: 0 16px 40px rgba(15, 23, 42, 0.14);
}

.pill-btn {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 11px 26px;
  font-size: 14px;
  font-weight: 700;
  font-family: inherit;
  border: none;
  border-radius: 999px;
  cursor: pointer;
  transition: all 0.3s cubic-bezier(0.34, 1.56, 0.64, 1);
}
.pill-btn:disabled { opacity: 0.5; cursor: not-allowed; }
.pill-btn.primary { background: #007bff; color: #fff; box-shadow: 0 4px 14px rgba(0, 123, 255, 0.35); }
.pill-btn.primary:hover:not(:disabled) { transform: scale(1.05); }
.pill-btn.ghost { background: rgba(0, 0, 0, 0.05); color: #6b7280; }
.pill-btn.ghost:hover:not(:disabled) { background: rgba(0, 0, 0, 0.09); transform: scale(1.05); }
.pill-btn.danger { background: rgba(239, 68, 68, 0.12); color: #dc2626; }
.pill-btn.danger:hover:not(:disabled) { background: rgba(239, 68, 68, 0.2); transform: scale(1.05); }

.ri-spin { display: inline-block; }
</style>
