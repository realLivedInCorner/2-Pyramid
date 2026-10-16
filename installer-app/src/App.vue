<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import { getCurrentWindow } from "@tauri-apps/api/window";
// 安装器没有主题设置界面，深色跟随系统；判据、核对过的权限与兜底见 theme.ts 文件头。
import { initTheme } from "./theme";

interface InstallContext {
  updateMode: boolean;
  storeInstalled?: boolean;
  uninstallMode: boolean;
  installed: boolean;
  installedVersion: string | null;
  newVersion: string;
  channel: string;
  dir: string;
  appRunning: boolean;
}

// 安装：0 介绍 / 1 协议 / 2 路径 / 3 安装中 / 4 完成
// 更新：0 覆盖更新确认 / 1 更新中 / 2 完成
// 卸载：0 确认 / 4 完成
const step = ref(0);
const installTotalSteps = 5;
const updateTotalSteps = 3;

const uninstallMode = ref(false);
const deleteUserData = ref(false);
const storeInstalled = ref(false);
const updateMode = ref(false);
const installed = ref(false);
const installedVersion = ref<string | null>(null);
const dir = ref("");
const version = ref("2.0.0");
const channel = ref("stable");
const githubUrl = ref("");
const busy = ref(false);
const failed = ref(false);
const resultMessage = ref("");
const appRunning = ref(false);
const closingApp = ref(false);

// 阅读确认（非「同意许可」——本项目 MIT，不额外限制用户权利）
const eulaTitle = ref("使用须知与免责声明");
const eulaBody = ref("");
const eulaAgreed = ref(false);

// 快捷方式选项：桌面 / 开始菜单（更新模式不使用）
const shortcutDesktop = ref(true);
const shortcutStartMenu = ref(true);

// 安装进度
const progressCurrent = ref(0);
const progressTotal = ref(0);
const progressName = ref("");
const progressPercent = ref(0);
let unlistenProgress: UnlistenFn | null = null;
// 主题的取消订阅（内联脚本定的首帧色 + 这里订阅的系统切换）
let unlistenTheme: (() => void) | null = null;
let runningPoll: number | null = null;

const totalSteps = computed(() => (updateMode.value ? updateTotalSteps : installTotalSteps));
const brandTag = computed(() => {
  if (uninstallMode.value) return `卸载程序 · v${version.value}`;
  if (updateMode.value) return `更新程序 · v${version.value}`;
  return `安装程序 · v${version.value}`;
});
const versionArrowLabel = computed(() => {
  const from = installedVersion.value || "旧版";
  return `${from}  →  ${version.value}`;
});
const canStartUpdate = computed(() => !busy.value && !!dir.value.trim() && !appRunning.value);

async function refreshAppRunning() {
  try {
    appRunning.value = await invoke<boolean>("is_app_running_cmd");
  } catch {
    appRunning.value = false;
  }
}

onMounted(async () => {
  // 主题先定下来再拉安装上下文：它决定首屏配色，晚一步用户就会看到一次色闪。
  // 首帧那次已由 index.html 的内联脚本做完（同步、不走 IPC），
  // 这里补的是 Tauri 窗口主题那一路判据与「系统切换时跟着变」。
  // 外面再包一层 try：配色是装饰，绝不能让「装不上」由它引起 ——
  // initTheme 内部已经吞掉了 IPC 异常，这里是给以后改动留的保险。
  try {
    unlistenTheme = await initTheme();
  } catch (e) {
    console.warn("[installer] theme init failed, fallback to light:", e);
  }

  try {
    const ctx = await invoke<InstallContext>("get_install_context");
    uninstallMode.value = ctx.uninstallMode;
    updateMode.value = ctx.updateMode;
    storeInstalled.value = ctx.storeInstalled;
    installed.value = ctx.installed;
    installedVersion.value = ctx.installedVersion;
    dir.value = ctx.dir;
    version.value = ctx.newVersion;
    channel.value = ctx.channel;
    appRunning.value = ctx.appRunning;
    try {
      githubUrl.value = await invoke<string>("get_github_url");
    } catch { /* optional */ }
    if (!uninstallMode.value && !updateMode.value) {
      try {
        const eula = await invoke<{ title: string; body: string }>("get_eula");
        eulaTitle.value = eula.title;
        eulaBody.value = eula.body;
      } catch {
        eulaBody.value = "请阅读仓库 legal/EULA.md 与 legal/DISCLAIMER.md。";
      }
    }
  } catch (e) {
    console.error("[installer] init failed:", e);
    // 回退：尽量用旧命令拼出上下文
    try {
      uninstallMode.value = await invoke<boolean>("is_uninstall_mode");
      installed.value = await invoke<boolean>("is_installed");
      updateMode.value = !uninstallMode.value && installed.value;
      dir.value = uninstallMode.value
        ? ((await invoke<string | null>("get_installed_dir")) ?? await invoke<string>("get_default_dir"))
        : await invoke<string>("get_default_dir");
      version.value = await invoke<string>("get_version");
      channel.value = await invoke<string>("get_channel");
    } catch { /* ignore */ }
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

  if (updateMode.value) {
    runningPoll = window.setInterval(() => {
      if (!busy.value) void refreshAppRunning();
    }, 1500);
  }
});

onUnmounted(() => {
  if (unlistenTheme) {
    unlistenTheme();
    unlistenTheme = null;
  }
  if (unlistenProgress) {
    unlistenProgress();
    unlistenProgress = null;
  }
  if (runningPoll !== null) {
    window.clearInterval(runningPoll);
    runningPoll = null;
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
  if (step.value === 1 && !eulaAgreed.value) return;
  if (step.value < totalSteps.value - 1) step.value++;
};

const prev = () => {
  if (step.value > 0) step.value--;
};

const closeRunningApp = async () => {
  if (closingApp.value) return;
  closingApp.value = true;
  try {
    await invoke("close_running_app");
    await refreshAppRunning();
  } catch (e) {
    failed.value = true;
    resultMessage.value = String(e);
  } finally {
    closingApp.value = false;
  }
};

const doInstall = async () => {
  if (!dir.value.trim() || busy.value) return;
  if (updateMode.value && appRunning.value) {
    failed.value = true;
    resultMessage.value = "2-Pyramid 仍在运行，请先退出再更新。";
    return;
  }
  busy.value = true;
  failed.value = false;
  progressCurrent.value = 0;
  progressTotal.value = 0;
  progressName.value = "";
  progressPercent.value = 0;
  // 更新模式：step 1 = 更新中；安装模式：step 3 = 安装中
  step.value = updateMode.value ? 1 : 3;
  try {
    resultMessage.value = await invoke<string>("install", {
      dir: dir.value.trim(),
      shortcuts: {
        desktop: shortcutDesktop.value,
        startMenu: shortcutStartMenu.value,
      },
    });
    installed.value = true;
    progressPercent.value = 100;
    step.value = updateMode.value ? 2 : 4;
  } catch (e) {
    failed.value = true;
    resultMessage.value = String(e);
  } finally {
    busy.value = false;
    if (updateMode.value) void refreshAppRunning();
  }
};

const doUninstall = async () => {
  if (busy.value) return;
  busy.value = true;
  failed.value = false;
  try {
    resultMessage.value = await invoke<string>("uninstall", {
      dir: dir.value.trim(),
      deleteUserData: deleteUserData.value,
    });
    installed.value = false;
    step.value = 4;
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

const retryFromUpdateOverview = () => {
  failed.value = false;
  resultMessage.value = "";
  step.value = 0;
  void refreshAppRunning();
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
          {{ brandTag }}
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
      <!-- ── 更新模式：步骤 0 覆盖更新确认 ── -->
      <div v-if="updateMode && step === 0" class="panel">
        <div v-if="storeInstalled" class="status err" style="margin-bottom: 10px;">
          <i class="ri-windows-line"></i>
          <span>检测到 Microsoft Store 版本。本安装器仅覆盖 GitHub 安装版，不会改动商店应用。</span>
        </div>
        <div class="panel-title">覆盖更新</div>
        <p class="panel-desc">
          检测到本机已安装 2-Pyramid。将覆盖程序文件完成更新，
          设置、转换记录等用户数据会保留。
        </p>

        <div class="update-version-card">
          <div class="uv-block">
            <span class="uv-label">当前版本</span>
            <span class="uv-value old">{{ installedVersion || "未知" }}</span>
          </div>
          <i class="ri-arrow-right-line uv-arrow" aria-hidden="true"></i>
          <div class="uv-block">
            <span class="uv-label">更新到</span>
            <span class="uv-value new">{{ version }}</span>
          </div>
        </div>

        <div class="update-facts">
          <div class="fact">
            <i class="ri-folder-line" aria-hidden="true"></i>
            <div>
              <b>安装位置</b>
              <span class="fact-mono">{{ dir }}</span>
            </div>
          </div>
          <div class="fact">
            <i class="ri-database-2-line" aria-hidden="true"></i>
            <div>
              <b>保留用户数据</b>
              <span>设置、转换历史、自定义背景（~/.2pyr）不会被删除</span>
            </div>
          </div>
          <div class="fact">
            <i class="ri-links-line" aria-hidden="true"></i>
            <div>
              <b>快捷方式</b>
              <span>沿用现有入口，不会自动重建</span>
            </div>
          </div>
        </div>

        <div v-if="appRunning" class="status warn">
          <i class="ri-alert-line" aria-hidden="true"></i>
          <div class="warn-text">
            <b>2-Pyramid 仍在运行</b>
            <span>文件被占用会导致更新失败，请先退出应用。</span>
          </div>
          <button class="btn ghost sm" :disabled="closingApp" @click="closeRunningApp">
            <i :class="closingApp ? 'ri-loader-4-line ri-spin' : 'ri-shut-down-line'"></i>
            {{ closingApp ? '正在退出…' : '退出应用' }}
          </button>
        </div>
        <div v-else-if="failed" class="status err">
          <i class="ri-error-warning-line" aria-hidden="true"></i>
          <span>{{ resultMessage }}</span>
        </div>
      </div>

      <!-- ── 更新模式：步骤 1 更新中 ── -->
      <div v-else-if="updateMode && step === 1" class="panel">
        <div class="panel-title">正在更新</div>
        <p class="panel-desc">正在覆盖写入程序文件，请稍候…</p>
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
          <button class="btn ghost sm" @click="retryFromUpdateOverview">返回</button>
        </div>
      </div>

      <!-- ── 更新模式：步骤 2 完成 ── -->
      <div v-else-if="updateMode && step === 2" class="panel">
        <div class="panel-title">{{ failed ? '更新失败' : '更新完成' }}</div>
        <div :class="failed ? 'status err' : 'status ok'">
          <i :class="failed ? 'ri-error-warning-line' : 'ri-checkbox-circle-line'" aria-hidden="true"></i>
          <span>{{ resultMessage }}</span>
        </div>
        <div v-if="!failed" class="status-actions">
          <button class="btn primary" @click="doLaunch"><i class="ri-rocket-2-line"></i> 启动 2-Pyramid</button>
          <button class="btn ghost" @click="closeWindow">关闭</button>
        </div>
        <div v-else class="status-actions">
          <button class="btn ghost" @click="retryFromUpdateOverview">返回</button>
          <button class="btn ghost" @click="closeWindow">关闭</button>
        </div>
      </div>

      <!-- 安装模式：步骤 0 介绍 -->
      <div v-else-if="!uninstallMode && step === 0" class="panel">
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

      <!-- 步骤 1 用户协议 -->
      <div v-else-if="!uninstallMode && step === 1" class="panel">
        <div class="panel-title">{{ eulaTitle }}</div>
        <div class="eula-box">{{ eulaBody }}</div>
        <label class="check-row eula-agree">
          <input type="checkbox" v-model="eulaAgreed" />
          <span>我已阅读并知悉上述说明（含免责声明与 Minecraft 条款提醒）</span>
        </label>
      </div>

      <!-- 安装模式：步骤 2 安装位置 -->
      <div v-else-if="!uninstallMode && step === 2" class="panel">
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

      <!-- 安装模式：步骤 3 安装中 -->
      <div v-else-if="!uninstallMode && step === 3" class="panel">
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
      <div v-else-if="step === 4" class="panel">
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
          <!--
            失败时可以退回上一步重试。
            原来写的是 v-if="step === 3"，但这个面板本身在 v-else-if="step === 4"
            分支内 —— 在 step 4 里判断 step 3 恒为假，按钮从上线起从未渲染过。
            卸载模式不回退：卸载没有「上一步」可重试。
          -->
          <button v-if="!uninstallMode" class="btn ghost" @click="prev">返回重试</button>
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
          可选择是否同时删除用户数据。
        </p>
        <label class="field-label">安装目录</label>
        <div class="field-row">
          <input v-model="dir" class="dir-input" spellcheck="false" readonly />
        </div>
        <label class="check-row" style="margin-top: 12px; display: flex; gap: 8px; align-items: center; cursor: pointer;">
          <input type="checkbox" v-model="deleteUserData" />
          <span>同时删除用户数据（~/.2pyr、设置、转换记录、背景；不可恢复）</span>
        </label>
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
        <!-- 更新模式 -->
        <button
          v-if="updateMode && step === 0"
          class="pill-btn primary"
          :disabled="!canStartUpdate"
          @click="doInstall"
        >
          <i v-if="appRunning" class="ri-lock-line"></i>
          <i v-else class="ri-refresh-line"></i>
          {{ appRunning ? '请先退出应用' : '立即更新' }}
        </button>
        <button
          v-else-if="updateMode && step === 1"
          class="pill-btn primary"
          disabled
        ><i class="ri-loader-4-line ri-spin"></i> 更新中…</button>

        <!-- 安装模式 -->
        <button
          v-else-if="!uninstallMode && step > 0 && step < 4"
          class="pill-btn ghost"
          :disabled="busy"
          @click="prev"
        ><i class="ri-arrow-left-s-line"></i> 上一步</button>

        <button
          v-if="!updateMode && !uninstallMode && step === 0"
          class="pill-btn primary"
          @click="next"
        >下一步 <i class="ri-arrow-right-s-line"></i></button>

        <button
          v-else-if="!updateMode && !uninstallMode && step === 1"
          class="pill-btn primary"
          :disabled="!eulaAgreed"
          @click="next"
        >下一步 <i class="ri-arrow-right-s-line"></i></button>

        <button
          v-else-if="!updateMode && !uninstallMode && step === 2"
          class="pill-btn primary"
          :disabled="busy || !dir.trim()"
          @click="doInstall"
        ><i class="ri-install-line"></i> 安装</button>

        <button
          v-else-if="!updateMode && !uninstallMode && step === 3"
          class="pill-btn primary"
          disabled
        ><i class="ri-loader-4-line ri-spin"></i> 安装中…</button>

        <button
          v-else-if="uninstallMode && step !== 4"
          class="pill-btn danger"
          :disabled="busy || !installed"
          @click="doUninstall"
        ><i :class="busy ? 'ri-loader-4-line ri-spin' : 'ri-delete-bin-line'"></i> {{ busy ? '卸载中…' : '卸载' }}</button>
      </div>
    </footer>
  </div>
</template>

<style>
/*
 * ══ 颜色全部走令牌 ════════════════════════════════════════════════════
 *
 * 令牌在 src/styles/tokens.css（安装器自带的一份，值与主程序的 shared.css 对齐），
 * 本文件里没有任何写死的颜色值。
 *
 * 迁移前这里有 90 处硬编码，用的是另一套调色板：14 处 Bootstrap 蓝 #007bff、
 * 8 处冷板岩灰 #94a3b8、3 处 #6b7280 …。主程序早就不用它们了 —— 暖白纸面上
 * 压一个高饱和冷蓝，纸感直接没了；#94a3b8 当正文在浅色底上实测只有 2.29:1，
 * 连 3:1 都不到。替换规则：
 *   · 强调色一律 var(--accent)（浅色 = 墨色 #14171a，深色自动提亮成 #929496）；
 *   · 中性叠加用 rgba(var(--ink-rgb), α)（阴影/压暗）或 rgba(var(--warm-rgb), α)
 *     （发丝线/悬停染色），不写死黑 —— 深色下 --ink-rgb 会翻成浅色，
 *     写死的黑在近黑底上什么都画不出来；
 *   · 玻璃面按原 alpha 就近落到 --panel-1..4（深色下它们弱一个数量级，
 *     写死的 rgba(255,255,255,·) 在深色下会变成一块块发光的白板）；
 *   · 语义色 --danger / --warn / --ok，淡底用 color-mix(…, transparent) 派生，
 *     这样深色下同一行代码自动跟着语义色变亮。
 *
 * ⚠ alpha 不是照抄的：--warm-rgb 是 120,110,95，同一个 alpha 比纯黑淡得多。
 *   换算表（浅色、底 #f4f2ee、叠完每通道差 ≤3）见 tokens.css 文件头。
 */
* { margin: 0; padding: 0; box-sizing: border-box; }

html, body, #app {
  height: 100%;
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", "PingFang SC", "Microsoft YaHei", sans-serif;
}

.installer-root {
  height: 100%;
  display: flex;
  flex-direction: column;
  background: linear-gradient(165deg, var(--surface-2) 0%, var(--bg) 55%, var(--surface-3) 100%);
  color: var(--ink);
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
  color: var(--ink-3);
  font-size: 18px;
  cursor: pointer;
  -webkit-app-region: no-drag;
  transition: all 0.15s;
}
.window-close:hover { background: color-mix(in srgb, var(--danger) 12%, transparent); color: var(--danger); }

/* 顶部品牌 */
.top-brand {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 28px 40px 0;
}

.logo { width: 42px; height: 42px; color: var(--accent); }

.top-brand-text { display: flex; flex-direction: column; gap: 1px; }
.top-brand-name { font-size: 17px; font-weight: 800; letter-spacing: -0.4px; }
.top-brand-tag { font-size: 12px; color: var(--ink-3); }

.beta-badge {
  font-style: normal;
  font-size: 10.5px;
  font-weight: 800;
  padding: 1px 7px;
  margin-left: 6px;
  border-radius: 999px;
  /* Beta 是「提醒」不是「错误」：走 --warn 语义色。
     原来写死的 #ea580c / rgba(249,115,22,·) 是 Tailwind 橙，主程序用的是 --warn 这一对，
     而且 --warn-tint / --warn 在深色下会自动换成低透明的琥珀底（见 tokens.css）。 */
  background: var(--warn-tint);
  color: var(--warn);
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
  background: rgba(var(--warm-rgb), 0.12);
  color: var(--ink-4);
  transition: all 0.3s cubic-bezier(0.22, 1, 0.36, 1);
}
.step-dot i { font-size: 14px; }
.step-dot.active {
  background: var(--accent);
  /* 叠在强调色上的文字用 --surface：浅色下强调色是墨色、--surface 是白（17.99:1），
     深色下强调色提亮成 #929496、--surface 变成深面 #232019（5.34:1）—— 一份声明两套主题都成立。
     写死 #fff 在深色下只剩 3.04:1，过不了 AA 正文 4.5:1。 */
  color: var(--surface);
  box-shadow: 0 2px 10px color-mix(in srgb, var(--accent) 40%, transparent);
}
.step-dot.done { background: color-mix(in srgb, var(--accent) 18%, transparent); color: var(--accent); }

.step-line {
  width: 34px; height: 2px;
  border-radius: 1px;
  background: rgba(var(--warm-rgb), 0.16);
  transition: background 0.3s ease;
}
.step-line.done { background: color-mix(in srgb, var(--accent) 45%, transparent); }

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

/* 标题原来是 #111827（冷黑）。这套设计里只有一种墨色：--ink。
   #111827 在主程序里早被换掉了 —— 暖白纸面上它偏蓝，和暖灰放一起会发脏。 */
.panel-title { font-size: 26px; font-weight: 800; letter-spacing: -0.5px; color: var(--ink); }
.panel-desc { font-size: 14px; color: var(--ink-2); line-height: 1.7; max-width: 480px; }

.feature-row { display: flex; gap: 14px; width: 100%; margin-top: 8px; }

/*
 * 玻璃面。原来写死 rgba(255,255,255,0.8) + 白边 —— 深色下这就是一块发光的白板，
 * 白边更是直接消失（白压白）。现在：面走 --panel-3（深色下只有 6% 白），
 * 边改走 --warm-rgb 发丝线（主程序的卡片/幽灵按钮都是这条线），
 * 投影用 --ink-rgb（深色下翻成浅色，成为一层极淡的抬升）。
 */
.feature {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  padding: 22px 16px;
  background: var(--panel-3);
  border: 1px solid rgba(var(--warm-rgb), 0.2);
  border-radius: 16px;
  box-shadow: 0 8px 24px rgba(var(--ink-rgb), 0.05);
  backdrop-filter: blur(18px);
  -webkit-backdrop-filter: blur(18px);
}
.feature i { font-size: 26px; color: var(--accent); }
.feature b { font-size: 14px; }
.feature span { font-size: 12px; color: var(--ink-3); }

.github-card {
  width: 100%;
  display: flex;
  align-items: center;
  gap: 14px;
  margin-top: 8px;
  padding: 16px 18px;
  background: var(--panel-3);
  border: 1px solid rgba(var(--warm-rgb), 0.2);
  border-radius: 16px;
  box-shadow: 0 8px 24px rgba(var(--ink-rgb), 0.05);
  backdrop-filter: blur(18px);
  -webkit-backdrop-filter: blur(18px);
  text-align: left;
}
.github-card > i { font-size: 30px; color: var(--ink); }
.github-text { flex: 1; display: flex; flex-direction: column; gap: 2px; }
.github-text b { font-size: 14px; }
.github-text span { font-size: 12.5px; color: var(--ink-3); }

/* ── 覆盖更新页 ── */
.update-version-card {
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 18px;
  padding: 20px 24px;
  background: var(--panel-3);
  border: 1px solid rgba(var(--warm-rgb), 0.2);
  border-radius: 18px;
  box-shadow: 0 8px 24px rgba(var(--ink-rgb), 0.05);
  backdrop-filter: blur(18px);
  -webkit-backdrop-filter: blur(18px);
}

.uv-block {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  min-width: 0;
}

.uv-label {
  font-size: 12px;
  font-weight: 600;
  color: var(--ink-4);
}

.uv-value {
  font-size: 22px;
  font-weight: 800;
  letter-spacing: -0.3px;
  font-variant-numeric: tabular-nums;
}

.uv-value.old { color: var(--ink-3); }
.uv-value.new { color: var(--accent); }

.uv-arrow {
  font-size: 22px;
  color: var(--ink-4);
  flex-shrink: 0;
}

.update-facts {
  width: 100%;
  display: flex;
  flex-direction: column;
  gap: 10px;
  text-align: left;
}

.update-facts .fact {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  padding: 12px 14px;
  background: var(--panel-2);
  border: 1px solid rgba(var(--warm-rgb), 0.14);
  border-radius: 14px;
}

.update-facts .fact > i {
  font-size: 18px;
  color: var(--accent);
  margin-top: 2px;
  flex-shrink: 0;
}

.update-facts .fact > div {
  display: flex;
  flex-direction: column;
  gap: 3px;
  min-width: 0;
}

.update-facts .fact b {
  font-size: 13.5px;
  color: var(--ink);
}

.update-facts .fact span {
  font-size: 12.5px;
  color: var(--ink-2);
  line-height: 1.5;
}

.fact-mono {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 12px !important;
  word-break: break-all;
}

.field-label { width: 100%; text-align: left; font-size: 13px; font-weight: 700; }

.field-row { display: flex; gap: 10px; width: 100%; }

.shortcut-options {
  width: 100%;
  display: flex;
  flex-direction: column;
  gap: 9px;
  padding: 14px 16px;
  border: 1.5px dashed rgba(var(--warm-rgb), 0.28);
  border-radius: 12px;
  background: var(--panel-2);
}

.check-row {
  display: flex;
  align-items: center;
  gap: 9px;
  font-size: 13.5px;
  color: var(--ink-2);
  cursor: pointer;
  user-select: none;
}
.check-row input {
  width: 16px;
  height: 16px;
  accent-color: var(--accent);
  cursor: pointer;
  margin: 0;
}
.check-row i { font-size: 15px; color: var(--accent); }

.align-hint {
  display: flex;
  align-items: center;
  gap: 6px;
  color: var(--accent);
  font-weight: 700;
}

.dir-input {
  flex: 1;
  padding: 12px 15px;
  font-size: 13.5px;
  font-family: inherit;
  border: 1.5px solid rgba(var(--warm-rgb), 0.2);
  border-radius: 11px;
  background: var(--panel-4);
  color: var(--ink);
  outline: none;
}
/* 聚焦环用 color-mix 派生而不是另取一个蓝：强调色换掉时它跟着换。
   16% 与主程序输入框的聚焦环同档（0 0 0 3px color-mix(accent 16%, transparent)）。 */
.dir-input:focus {
  border-color: var(--accent);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 16%, transparent);
}

.hint { width: 100%; text-align: left; font-size: 12.5px; color: var(--ink-3); line-height: 1.6; }

.eula-box {
  width: 100%;
  flex: 1;
  min-height: 180px;
  max-height: 280px;
  overflow-y: auto;
  padding: 14px 16px;
  border-radius: 12px;
  /* EULA 是「凹陷的读稿区」：比卡片暗一档。原来是 rgba(0,0,0,0.04)，
     换成暖灰后 alpha 要抬到 0.07 才是同一个明度（叠完 #ebe9e4 vs #eae8e4）。 */
  background: rgba(var(--warm-rgb), 0.07);
  border: 1px solid rgba(var(--warm-rgb), 0.16);
  font-size: 12.5px;
  line-height: 1.7;
  color: var(--ink-2);
  white-space: pre-wrap;
  text-align: left;
}
.eula-agree {
  width: 100%;
  margin-top: 8px;
  font-size: 13px;
  font-weight: 600;
}

/* 进度 */
.progress-wrap { width: 100%; display: flex; flex-direction: column; gap: 10px; margin-top: 10px; }

.progress-track {
  width: 100%; height: 12px;
  border-radius: 999px;
  background: rgba(var(--warm-rgb), 0.12);
  overflow: hidden;
}

.progress-fill {
  height: 100%;
  border-radius: 999px;
  /* 原来是一条 蓝 → 浅蓝 的渐变（#007bff → #60a5fa）。设计系统里的进度是单色的
     （主程序 UpdateDialog 的 .ud-progress-fill 就是 background: var(--ink)），
     这里取 --accent：浅色下是墨色，深色下自动提亮成浅灰，两端都看得见。 */
  background: var(--accent);
  transition: width 0.25s ease;
}

.progress-meta {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 13px;
  font-weight: 700;
  color: var(--accent);
}

.progress-file {
  flex: 1;
  margin-left: 16px;
  font-weight: 500;
  font-size: 12px;
  color: var(--ink-4);
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
/*
 * 状态面板。淡底一律用 color-mix(语义色, transparent) 派生，不写死「淡绿/淡红/淡蓝」——
 * 深色下那三块写死的浅底会变成三块发光的色板，而语义色自己会变亮，派生底跟着走。
 *
 * ⚠ .ok 的文字**不能**用 --ok：浅色下 #10b981 压在这层 10% 绿底上实测只有 2.08:1
 *   （旧配色 #15803d on #ecfdf5 是 4.76:1），等于把「安装完成」这句话洗掉。
 *   主程序里 --ok 也只用在图标上，正文始终是墨色 —— 这里照同一套：
 *   语义色给图标，文字用 --ink（14.72:1）。
 */
.status.ok { background: color-mix(in srgb, var(--ok) 10%, transparent); color: var(--ink); }
.status.ok i { font-size: 30px; color: var(--ok); animation: status-pop 0.6s cubic-bezier(0.34, 1.56, 0.64, 1); }
/* 6% 与主程序 .cancel-btn:hover / .toast-safe 的 danger 底同档：
   --danger 压在上面浅色 5.33:1、深色 4.49:1，都在 AA 正文线上下。 */
.status.err { background: color-mix(in srgb, var(--danger) 6%, transparent); color: var(--danger); }
.status.err i { font-size: 30px; }
.status.working { background: color-mix(in srgb, var(--accent) 8%, transparent); color: var(--accent); }
.status.working i { font-size: 30px; }
.status.warn {
  flex-direction: row;
  align-items: center;
  /* --warn-tint / --warn 是一对：深色下 tint 会从 #fef3c7 变成 14% 琥珀，
     浅色那对写死的话深色下就是一块亮黄板。 */
  background: var(--warn-tint);
  color: var(--warn);
  text-align: left;
  gap: 12px;
}
.status.warn > i { font-size: 24px; flex-shrink: 0; }
.warn-text {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 3px;
  min-width: 0;
}
.warn-text b { font-size: 13.5px; }
.warn-text span { font-size: 12.5px; opacity: 0.9; white-space: normal; }
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
/* 主操作按钮：--accent 面 + --surface 字，与主程序一致
   （shared.css / StartupPage / ItemSizeDialog 的主按钮都是这一对）。
   写死 #fff 在深色下是白字压提亮后的强调色 #929496，实测 3.04:1，
   过不了 AA 正文 4.5:1；--surface（深色下是 #232019）压上去是 5.34:1。 */
.btn.primary { background: var(--accent); color: var(--surface); box-shadow: 0 4px 14px color-mix(in srgb, var(--accent) 35%, transparent); }
.btn.primary:hover:not(:disabled) { transform: scale(1.04); }
.btn.ghost { background: rgba(var(--warm-rgb), 0.09); color: var(--ink-2); }
.btn.ghost:hover:not(:disabled) { background: rgba(var(--warm-rgb), 0.16); }
.btn.sm {
  height: 32px;
  padding: 0 12px;
  font-size: 12.5px;
  border-radius: 9px;
  flex-shrink: 0;
}

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
  background: var(--panel-4);
  border: 1px solid rgba(var(--warm-rgb), 0.2);
  border-radius: 999px;
  box-shadow: 0 10px 30px rgba(var(--ink-rgb), 0.12);
  backdrop-filter: blur(18px);
  -webkit-backdrop-filter: blur(18px);
  transition: transform 0.35s cubic-bezier(0.34, 1.56, 0.64, 1), box-shadow 0.35s ease;
}
.nav-pill:hover {
  transform: translateY(-2px);
  box-shadow: 0 16px 40px rgba(var(--ink-rgb), 0.16);
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
.pill-btn.primary { background: var(--accent); color: var(--surface); box-shadow: 0 4px 14px color-mix(in srgb, var(--accent) 35%, transparent); }
.pill-btn.primary:hover:not(:disabled) { transform: scale(1.05); }
.pill-btn.ghost { background: rgba(var(--warm-rgb), 0.09); color: var(--ink-2); }
.pill-btn.ghost:hover:not(:disabled) { background: rgba(var(--warm-rgb), 0.16); transform: scale(1.05); }
/* 卸载是危险操作，但按钮本身是淡底 + 危险色文字（主程序 .cancel-btn 的做法），
   不做成实心红 —— 向导里唯一实心的应该只有主操作。 */
.pill-btn.danger { background: color-mix(in srgb, var(--danger) 12%, transparent); color: var(--danger); }
.pill-btn.danger:hover:not(:disabled) { background: color-mix(in srgb, var(--danger) 20%, transparent); transform: scale(1.05); }

.ri-spin { display: inline-block; }
</style>
