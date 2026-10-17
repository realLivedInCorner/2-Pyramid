<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import { getCurrentWindow } from "@tauri-apps/api/window";
// 安装器没有主题设置界面，深色跟随系统；判据、核对过的权限与兜底见 theme.ts 文件头。
import { initTheme } from "./theme";
// 阶梯背景。与主程序 src/components/shell/StageBackdrop.vue 是同一块地面，
// 但这里是**重写的一份**（安装器是独立构建，不跨项目 import），理由见该文件头。
import StageBackdrop from "./components/StageBackdrop.vue";

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

/** 步骤计数补零成 `01`，与主程序 OOBE / 首页模块计数同一形式（见 StartupPage.vue:17）。 */
const pad = (n: number) => String(n).padStart(2, "0");

/**
 * 每一屏的标题与说明 —— 全部集中在这里，模板只渲染一次。
 *
 * 为什么从模板里搬出来：原来每个面板自己写一遍 `.panel-title` + `.panel-desc`，
 * 于是标题的字号、位置、留白在一屏之间各走各的（有的带说明、有的不带，
 * 有的居中、有的靠左）。切步骤时标题会「跳」一下，而这恰好是从「欢迎」
 * 到「协议」这一步最显眼的地方。集中之后标题只有一条基线，切步骤它不动。
 *
 * `counter: false` 只给卸载：卸载是单一动作，没有「第几件 / 共几件」可数，
 * 硬凑一个 01 / 01 只是噪音。
 */
const stepMeta = computed<{
  kicker: string;
  counter: boolean;
  title: string;
  desc?: string;
}>(() => {
  if (uninstallMode.value) {
    const done = step.value === 4;
    return {
      kicker: "UNINSTALL",
      counter: false,
      title: done
        ? (failed.value ? "卸载失败" : "卸载完成")
        : `卸载 ${channel.value === "beta" ? "2-Pyramid Beta" : "2-Pyramid"}`,
      desc: done ? undefined : "将移除程序文件与快捷方式。",
    };
  }

  if (updateMode.value) {
    if (step.value === 0) {
      return {
        kicker: "UPDATE / STEP",
        counter: true,
        title: "覆盖更新",
        desc: "用新版本覆盖程序文件。设置、转换记录与自定义背景都会保留。",
      };
    }
    // 进度页不给说明：「正在写入程序文件」是进度条本身在说的事，
    // 再写一行「请稍候…」只是把版面填满。
    if (step.value === 1) {
      // 失败时标题要跟着变：固定写「正在更新」时，面板里已经是一条红色错误，
      // 标题还在说「正在」，读起来像没停过
      return { kicker: "UPDATE / STEP", counter: true, title: failed.value ? "更新失败" : "正在更新" };
    }
    return {
      kicker: "UPDATE / STEP",
      counter: true,
      title: failed.value ? "更新失败" : "更新完成",
    };
  }

  switch (step.value) {
    case 0:
      return {
        kicker: "SETUP / STEP",
        counter: true,
        title: "欢迎使用 2-Pyramid",
        desc: "多版本 Minecraft 资源包转换工具。",
      };
    case 1:
      // 标题来自后端（legal_text.rs 的 EULA_TITLE），不在前端另写一份
      return { kicker: "SETUP / STEP", counter: true, title: eulaTitle.value };
    case 2:
      return {
        kicker: "SETUP / STEP",
        counter: true,
        title: "选择安装位置",
        desc: "默认安装到当前用户目录，无需管理员权限。",
      };
    case 3:
      // 同上：失败时标题改为「安装失败」，否则「正在安装」+ 红色错误 + 一个禁用的
      // 「安装中…」按钮会读成「还在装」，用户不知道已经停了。
      return { kicker: "SETUP / STEP", counter: true, title: failed.value ? "安装失败" : "正在安装" };
    default:
      return {
        kicker: "SETUP / STEP",
        counter: true,
        title: failed.value ? "安装失败" : "安装完成",
      };
  }
});

/**
 * 面板的切换键。只由「模式 + 步骤」决定，**不含 failed / 进度** ——
 * 否则安装失败或进度刷新会让整块面板重新挂载，输入框失焦、进度条回零。
 */
const panelKey = computed(() => `${uninstallMode.value ? "u" : updateMode.value ? "g" : "i"}${step.value}`);

const progressLabel = computed(() => (updateMode.value ? "更新进度" : "安装进度"));

/**
 * 底部导航是否有按钮。
 *
 * 原来没有这个判断：完成屏（step 4）与更新完成屏（update step 2）里，
 * 底部那一串 v-if 全都落空，于是渲染出一个**空药丸** —— 一块 14px 高的
 * 圆角底板浮在窗口底部，什么都没装。它不是「留白」，是一个可见的残件。
 */
const showFooter = computed(() => {
  if (updateMode.value) return step.value === 0 || step.value === 1;
  if (uninstallMode.value) return step.value !== 4;
  return step.value < 4;
});

/**
 * 「上一步」只在安装流程的 1、2 两步出现。
 *
 * 原来的条件是 `!uninstallMode && step > 0 && step < 4`，在**更新完成屏**
 * （update step 2）上也成立：按下它 step 变成 1 —— 那是「正在更新」面板，
 * 一个 100% 的静态进度条加一个禁用的按钮，退无可退，死在那里。
 * 完成屏的出口是面板里的「启动 / 关闭」，不是返回。
 */
const showPrev = computed(() => !updateMode.value && !uninstallMode.value && step.value > 0 && step.value < 3);

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
    // storeInstalled 在接口里是可选的（老版本后端可能不带这个字段），
    // ref 的类型是 boolean —— 缺字段时按「没装 Store 版」处理。
    storeInstalled.value = ctx.storeInstalled ?? false;
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
    <!-- 阶梯背景。与主程序四个页面同一块地面 —— 安装器是用户看到的第一个界面，
         它如果踩在自己的渐变上，读起来就是「另一个产品」。 -->
    <StageBackdrop />

    <!--
      无边框窗口（tauri.conf.json 里 decorations: false）没有系统标题栏，
      拖动全靠这条带子。它必须是 fixed + 高层级：
        · 层级低于内容 ⇒ 内容吃掉 mousedown，窗口拖不动；
        · 层级高于内容 ⇒ 顶栏的文字点不到（顶栏只有品牌，没有可点的东西），
          但关闭按钮必须再高一层，否则按不动。
      高度 64px = 顶栏的 22px 上留白 + 40px 品牌块 + 2px 余量。
    -->
    <div class="drag-region" data-tauri-drag-region="true"></div>
    <button class="window-close" @click="closeWindow" aria-label="关闭窗口" title="关闭">×</button>

    <!-- 顶栏：品牌在左上，与 OOBE / 首页同一个位置、同一套构成（logo → 名称 → 版本行） -->
    <header class="top">
      <div class="brand">
        <!--
          描边必须带 vector-effect="non-scaling-stroke"。
          这个 viewBox 是 680×680，而元素只显示 40×40，缩放比 0.0588 ——
          原来的 stroke-width="7" 实际渲染出来是 0.41px，内线 5 只有 0.29px，
          亚像素：无论什么颜色都看不见，logo 看上去像一团灰雾。
          non-scaling-stroke 让 stroke-width 以 CSS 像素表达，不再随 viewBox 缩放。
          （主程序 StartupPage.vue 踩过同一个坑，这里取它验证过的 1.6 / 1.15。）

          **它必须写在每个 <line> 上，不能写在 <g> 上** —— vector-effect 不是
          继承属性，挂 <g> 上等于没挂。
        -->
        <svg class="brand-mark" viewBox="0 0 680 680" fill="none" xmlns="http://www.w3.org/2000/svg">
          <g stroke="currentColor" stroke-linecap="round">
            <line x1="260" y1="520" x2="200" y2="390" stroke-width="1.6" vector-effect="non-scaling-stroke"/>
            <line x1="260" y1="520" x2="500" y2="190" stroke-width="1.6" vector-effect="non-scaling-stroke"/>
            <line x1="200" y1="390" x2="500" y2="190" stroke-width="1.6" vector-effect="non-scaling-stroke"/>
            <line x1="260" y1="520" x2="374" y2="274" stroke-width="1.15" vector-effect="non-scaling-stroke"/>
          </g>
        </svg>
        <div class="brand-text">
          <span class="brand-name">2-Pyramid</span>
          <span class="brand-tag">{{ brandTag }}<em v-if="channel === 'beta'" class="beta-badge">Beta</em></span>
        </div>
      </div>
    </header>

    <!-- 主体：标题块（计数 + 标题 + 说明）在上，本步的内容在下 -->
    <main class="stage">
      <div class="subject">
        <span class="kicker">{{ stepMeta.kicker }}</span>
        <!--
          步骤计数 01 / 05，取代原来的「5 个圆点 + 连线」。
          圆点表达的是「比例」，计数表达的是「第几件 / 共几件」—— 主程序的
          首页模块与 OOBE 都用后者（见 StartupPage.vue:291 的注释），
          安装器是用户第一次见到这个软件的地方，两处不该是两套语言。
          计数刻意做弱：它是导航，不是内容。
        -->
        <div v-if="stepMeta.counter" class="counter">
          <b>{{ pad(step + 1) }}</b><i aria-hidden="true">/</i><span>{{ pad(totalSteps) }}</span>
        </div>
        <h1 class="title">{{ stepMeta.title }}</h1>
        <p v-if="stepMeta.desc" class="desc">{{ stepMeta.desc }}</p>
      </div>

      <Transition name="step" mode="out-in">
        <div class="panel" :key="panelKey">
          <!-- ══ 更新模式：0 覆盖更新确认 ══ -->
          <template v-if="updateMode && step === 0">
            <!-- Store 版提示是「说明」不是「错误」：用 note 不用 err。
                 原来这里挂的是 .status.err（红色语义），可它并不表示失败。 -->
            <div v-if="storeInstalled" class="status note">
              <i class="ri-windows-line" aria-hidden="true"></i>
              <span>检测到 Microsoft Store 版本。本安装器只覆盖 GitHub 安装版，不会改动商店应用。</span>
            </div>

            <div class="version-row">
              <div class="v-block">
                <span class="v-label">当前版本</span>
                <span class="v-value">{{ installedVersion || "未知" }}</span>
              </div>
              <i class="ri-arrow-right-line v-arrow" aria-hidden="true"></i>
              <div class="v-block">
                <span class="v-label">更新到</span>
                <span class="v-value v-new">{{ version }}</span>
              </div>
            </div>

            <ul class="facts">
              <li class="fact">
                <i class="ri-folder-line" aria-hidden="true"></i>
                <div class="fact-body">
                  <span class="fact-key">安装位置</span>
                  <span class="fact-val mono">{{ dir }}</span>
                </div>
              </li>
              <li class="fact">
                <i class="ri-database-2-line" aria-hidden="true"></i>
                <div class="fact-body">
                  <span class="fact-key">保留用户数据</span>
                  <span class="fact-val">设置、转换历史与自定义背景（~/.2pyr）不会被删除</span>
                </div>
              </li>
              <li class="fact">
                <i class="ri-links-line" aria-hidden="true"></i>
                <div class="fact-body">
                  <span class="fact-key">快捷方式</span>
                  <span class="fact-val">沿用现有入口，不自动重建</span>
                </div>
              </li>
            </ul>

            <div v-if="appRunning" class="status warn">
              <i class="ri-alert-line" aria-hidden="true"></i>
              <div class="warn-body">
                <b>2-Pyramid 仍在运行</b>
                <span>文件被占用会导致更新失败，请先退出。</span>
              </div>
              <button class="btn ghost sm" :disabled="closingApp" @click="closeRunningApp">
                <i :class="closingApp ? 'ri-loader-4-line ri-spin' : 'ri-shut-down-line'" aria-hidden="true"></i>
                {{ closingApp ? '正在退出…' : '退出应用' }}
              </button>
            </div>
            <div v-else-if="failed" class="status err">
              <i class="ri-error-warning-line" aria-hidden="true"></i>
              <span class="status-text">{{ resultMessage }}</span>
            </div>
          </template>

          <!-- ══ 更新模式：1 更新中 ══ -->
          <template v-else-if="updateMode && step === 1">
            <div class="progress">
              <div
                class="progress-track"
                role="progressbar"
                :aria-label="progressLabel"
                aria-valuemin="0"
                aria-valuemax="100"
                :aria-valuenow="progressPercent"
              >
                <div class="progress-fill" :style="{ width: progressPercent + '%' }"></div>
              </div>
              <!-- 百分比必须常显：轨道对底色的对比只有 1.16:1（浅）/ 1.18:1（深），
                   达不到 WCAG 1.4.11 对非文本组件要求的 3:1，进度不能只靠轨道读。 -->
              <div class="progress-meta">
                <span class="progress-pct">{{ progressPercent }}<i>%</i></span>
                <span class="progress-file">{{ progressName }}</span>
              </div>
            </div>
            <div v-if="failed" class="status err">
              <i class="ri-error-warning-line" aria-hidden="true"></i>
              <span class="status-text">{{ resultMessage }}</span>
              <button class="btn ghost sm" @click="retryFromUpdateOverview">返回</button>
            </div>
          </template>

          <!-- ══ 更新模式：2 完成 ══ -->
          <template v-else-if="updateMode && step === 2">
            <div :class="failed ? 'status err' : 'status ok'">
              <i :class="failed ? 'ri-error-warning-line' : 'ri-checkbox-circle-line'" aria-hidden="true"></i>
              <span class="status-text">{{ resultMessage }}</span>
            </div>
            <div class="actions">
              <template v-if="failed">
                <button class="btn ghost" @click="retryFromUpdateOverview">返回</button>
                <button class="btn primary" @click="closeWindow">关闭</button>
              </template>
              <template v-else>
                <button class="btn primary" @click="doLaunch"><i class="ri-rocket-2-line" aria-hidden="true"></i> 启动 2-Pyramid</button>
                <button class="btn ghost" @click="closeWindow">关闭</button>
              </template>
            </div>
          </template>

          <!-- ══ 安装模式：0 介绍 ══ -->
          <template v-else-if="!uninstallMode && step === 0">
            <!--
              三行能力清单，不做成三张玻璃卡片。
              卡片是「并排比较」的形态，而这三条是同质的说明；并排还会被
              880 的窗口压成三列窄条，汉字逐字换行。
              数字改过：原来写「26 个 Minecraft 版本目标」，实际是
              code/src/data/minecraftVersions.ts 里的 27 个目标（26 个 Java
              pack_format 档 + Bedrock Latest）。硬编码的数量每加一个 MC 版本
              就错一次，所以这里只写区间。
            -->
            <ul class="facts">
              <li class="fact">
                <i class="ri-swap-box-line" aria-hidden="true"></i>
                <div class="fact-body">
                  <span class="fact-key">一键转换</span>
                  <span class="fact-val">1.6 → 26.3 与 Bedrock Latest</span>
                </div>
              </li>
              <li class="fact">
                <i class="ri-palette-line" aria-hidden="true"></i>
                <div class="fact-body">
                  <span class="fact-key">深度定制</span>
                  <span class="fact-val">背景图片、主题色、控件样式</span>
                </div>
              </li>
              <li class="fact">
                <i class="ri-stack-line" aria-hidden="true"></i>
                <div class="fact-body">
                  <span class="fact-key">覆盖制作</span>
                  <span class="fact-val">自定义覆盖包，叠加到任意母包之上</span>
                </div>
              </li>
            </ul>

            <div class="source-row">
              <i class="ri-github-fill" aria-hidden="true"></i>
              <div class="source-body">
                <b>MIT 开源</b>
                <span>源码、Issue 与版本发布都在 GitHub</span>
              </div>
              <button class="btn ghost sm" @click="openGithub">打开仓库</button>
            </div>
          </template>

          <!-- ══ 安装模式：1 用户协议 ══ -->
          <template v-else-if="!uninstallMode && step === 1">
            <div class="eula-box">{{ eulaBody }}</div>
            <label class="check">
              <input type="checkbox" v-model="eulaAgreed" />
              <span>我已阅读并知悉上述说明（含免责声明与 Minecraft 条款提醒）</span>
            </label>
          </template>

          <!-- ══ 安装模式：2 安装位置 ══ -->
          <template v-else-if="!uninstallMode && step === 2">
            <div class="field">
              <label class="field-label" for="install-dir">安装目录</label>
              <div class="field-row">
                <input id="install-dir" v-model="dir" class="text-input" spellcheck="false" />
                <button class="btn ghost" @click="browse"><i class="ri-folder-open-line" aria-hidden="true"></i> 浏览</button>
              </div>
              <p v-if="installed" class="hint hint-accent">
                <i class="ri-information-line" aria-hidden="true"></i>
                已检测到安装的 2-Pyramid，目录已对齐到现有位置。
              </p>
            </div>

            <div class="field">
              <span class="field-label">快捷方式</span>
              <div class="checks">
                <label class="check">
                  <input type="checkbox" v-model="shortcutDesktop" />
                  <span>桌面</span>
                </label>
                <label class="check">
                  <input type="checkbox" v-model="shortcutStartMenu" />
                  <span>开始菜单</span>
                </label>
              </div>
            </div>

            <p class="hint">
              安装内容为主程序、转换引擎与内置资源，约 10 MB；卸载时用户数据（转换记录、设置、背景）会保留。
            </p>
          </template>

          <!-- ══ 安装模式：3 安装中 ══ -->
          <template v-else-if="!uninstallMode && step === 3">
            <div class="progress">
              <div
                class="progress-track"
                role="progressbar"
                :aria-label="progressLabel"
                aria-valuemin="0"
                aria-valuemax="100"
                :aria-valuenow="progressPercent"
              >
                <div class="progress-fill" :style="{ width: progressPercent + '%' }"></div>
              </div>
              <div class="progress-meta">
                <span class="progress-pct">{{ progressPercent }}<i>%</i></span>
                <span class="progress-file">{{ progressName }}</span>
              </div>
            </div>
            <div v-if="failed" class="status err">
              <i class="ri-error-warning-line" aria-hidden="true"></i>
              <span class="status-text">{{ resultMessage }}</span>
              <!--
                安装失败必须能回去重试。
                原来这一屏失败后没有任何出口：错误块只有文案，底部按钮是禁用的
                「安装中…」，而唯一写着「返回重试」的按钮长在 step === 4 的分支里
                —— 失败时 step 根本不会变成 4（只有成功才置 4），所以那个按钮
                永不可达。用户能做的只有关掉窗口重来。
                这里退回上一步（选目录），失败原因多半就在那里。
              -->
              <button class="btn ghost sm" @click="prev">返回</button>
            </div>
          </template>

          <!-- ══ 完成（安装 / 卸载共用）══ -->
          <template v-else-if="step === 4">
            <div :class="failed ? 'status err' : 'status ok'">
              <i :class="failed ? 'ri-error-warning-line' : 'ri-checkbox-circle-line'" aria-hidden="true"></i>
              <span class="status-text">{{ resultMessage }}</span>
            </div>
            <div class="actions">
              <template v-if="failed">
                <!-- 失败时退回上一步重试。
                     原来写的是 v-if="step === 3" 而这个面板本身在 step === 4 分支里，
                     判断恒为假 —— 按钮从上线起从未渲染过。卸载没有「上一步」可退。 -->
                <button v-if="!uninstallMode" class="btn ghost" @click="prev">返回重试</button>
                <button class="btn primary" @click="closeWindow">关闭</button>
              </template>
              <template v-else-if="uninstallMode">
                <button class="btn primary" @click="closeWindow">完成</button>
              </template>
              <template v-else>
                <button class="btn primary" @click="doLaunch"><i class="ri-rocket-2-line" aria-hidden="true"></i> 启动 2-Pyramid</button>
                <button class="btn ghost" @click="closeWindow">关闭</button>
              </template>
            </div>
          </template>

          <!-- ══ 卸载模式主面板 ══ -->
          <template v-else>
            <div class="field">
              <span class="field-label">安装目录</span>
              <input v-model="dir" class="text-input mono" spellcheck="false" readonly />
            </div>
            <label class="check">
              <input type="checkbox" v-model="deleteUserData" />
              <span>同时删除用户数据（~/.2pyr、设置、转换记录、背景；不可恢复）</span>
            </label>
            <div v-if="busy" class="status working">
              <i class="ri-loader-4-line ri-spin" aria-hidden="true"></i>
              <span>正在卸载，请稍候…</span>
            </div>
            <div v-else-if="failed" class="status err">
              <i class="ri-error-warning-line" aria-hidden="true"></i>
              <span class="status-text">{{ resultMessage }}</span>
            </div>
            <p v-else class="hint">点击「卸载」开始。完成后窗口会自动关闭，卸载程序随后自行清理残留。</p>
          </template>
        </div>
      </Transition>
    </main>

    <!-- 底部药丸导航（与 OOBE 的 .nav-pill 同款：居中、玻璃底、全圆角） -->
    <footer v-if="showFooter" class="foot">
      <div class="nav-pill">
        <!-- 更新模式 -->
        <template v-if="updateMode">
          <button
            v-if="step === 0"
            class="nav-btn primary"
            :disabled="!canStartUpdate"
            @click="doInstall"
          >
            <i :class="appRunning ? 'ri-lock-line' : 'ri-refresh-line'" aria-hidden="true"></i>
            {{ appRunning ? '请先退出应用' : '立即更新' }}
          </button>
          <button v-else class="nav-btn primary" disabled>
            <i :class="failed ? 'ri-error-warning-line' : 'ri-loader-4-line ri-spin'" aria-hidden="true"></i>
            {{ failed ? '更新失败' : '更新中…' }}
          </button>
        </template>

        <!-- 卸载模式 -->
        <template v-else-if="uninstallMode">
          <button
            class="nav-btn danger"
            :disabled="busy || !installed"
            @click="doUninstall"
          >
            <i :class="busy ? 'ri-loader-4-line ri-spin' : 'ri-delete-bin-line'" aria-hidden="true"></i>
            {{ busy ? '卸载中…' : '卸载' }}
          </button>
        </template>

        <!-- 安装模式 -->
        <template v-else>
          <button v-if="showPrev" class="nav-btn ghost" @click="prev">
            <i class="ri-arrow-left-s-line" aria-hidden="true"></i> 上一步
          </button>
          <button v-if="step === 0" class="nav-btn primary" @click="next">
            下一步 <i class="ri-arrow-right-s-line" aria-hidden="true"></i>
          </button>
          <button v-else-if="step === 1" class="nav-btn primary" :disabled="!eulaAgreed" @click="next">
            下一步 <i class="ri-arrow-right-s-line" aria-hidden="true"></i>
          </button>
          <button v-else-if="step === 2" class="nav-btn primary" :disabled="busy || !dir.trim()" @click="doInstall">
            <i class="ri-install-line" aria-hidden="true"></i> 安装
          </button>
          <button v-else class="nav-btn primary" disabled>
            <i :class="failed ? 'ri-error-warning-line' : 'ri-loader-4-line ri-spin'" aria-hidden="true"></i>
            {{ failed ? '安装失败' : '安装中…' }}
          </button>
        </template>
      </div>
    </footer>
  </div>
</template>

<style>
/*
 * ══ 颜色全部走令牌 ════════════════════════════════════════════════════
 *
 * 令牌在 src/styles/tokens.css（安装器自带的一份，值与主程序 shared.css 对齐）。
 * 本文件（以及 components/StageBackdrop.vue）里**没有任何写死的颜色值** ——
 * 注释之外 grep `#[0-9a-f]{6}` 的结果是 0。
 *
 * 说明一下上一版的状态，免得后人以为这里还留着旧账：git HEAD 的那份 App.vue
 * 里有 24 处 hex，全部在注释里（是更早一次迁移留下的历史说明），样式规则本身
 * 已经是令牌了。这一版真正动的是排版与结构，颜色只是顺手补齐了两条：
 *   · 中性叠加一律 rgba(var(--ink-rgb), α) 或 rgba(var(--warm-rgb), α)，
 *     不写死黑/白 —— 深色下 --ink-rgb 会翻成浅色，写死的黑在近黑底上画不出东西；
 *   · 语义色 --danger / --warn / --ok 的淡底用 color-mix(…, transparent) 派生，
 *     但**语义色只给图标与描边**，正文一律 --ink（理由与实测见 .status 一节）。
 *
 * ══ 三条排版规则（来自 docs/compose/spec/dialog-spec.md）═════════════
 *
 * 1. 层级靠**字号 + 留白 + 字距**建立，不靠加粗：UI 文本字重上限 600，禁止 700+。
 *    git HEAD 那份里有 **10 处 700 / 800**（.panel-title 26px/800、品牌名 17px/800、
 *    .field-label 13px/700、.progress-meta 700、.pill-btn 700 …），这一版是 0 处。
 *    800 在 26px 的中文标题上不是「更清楚」，是把笔画糊成一团黑 —— 中文没有
 *    x-height 与小写字母的对比，靠字重提层级只会让整行变脏。
 * 2. **中文标题不用负字距**。负字距是拉丁大标题的收紧技巧（大写字母之间本有
 *    空隙）；中文是满 em 方块字，-0.5px 会把相邻笔画挤到一起。上一版
 *    .panel-title 是 26px/800/-0.5px，三件事都在这条线上；这一版是 32px/600/0。
 *    Latin 的品牌名「2-Pyramid」保留 -0.4px —— 那是拉丁字，跟随主程序顶部品牌块。
 * 3. 卡片、列表行、页内块**一律无投影**；只有真正脱离文档流的元素（底部药丸、
 *    主操作按钮的轻微抬升）保留投影。上一版的 .feature / .github-card /
 *    .update-version-card 三处玻璃卡片各带一层 0 8px 24px 的浮起阴影再叠
 *    backdrop-filter，整屏像四块飘着的磨砂玻璃；这一版把这些块改成了
 *    「行 + 发丝线」的清单，投影跟着消失。
 */

* { margin: 0; padding: 0; box-sizing: border-box; }

html, body, #app { height: 100%; }

body {
  /* 与主程序 OOBE 同一套字体栈（StartupPage.vue:666）。先排中文字体：
     Windows 上 YaHei UI 的字形比 Segoe UI 回退出来的中文字形更整齐。 */
  font-family: "Microsoft YaHei UI", "Microsoft YaHei", "PingFang SC",
    "Hiragino Sans GB", "Noto Sans CJK SC", "Source Han Sans SC", -apple-system,
    BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
  /* 兜底底色，与 index.html 的 color-scheme、StageBackdrop 的 .stage-bg 同色。
     深色系统上任何一处漏掉都会在首帧闪一块白。 */
  background: var(--bg);
}

.installer-root {
  position: relative;
  height: 100%;
  display: flex;
  flex-direction: column;
  background: var(--bg);
  color: var(--ink);
  overflow: hidden;
}

/*
 * 主体三块必须自己建立层叠上下文。
 *
 * StageBackdrop 是 position: absolute，而顶栏 / 主体 / 底栏是**非定位元素** ——
 * 定位元素绘制在非定位内容之上，与 DOM 顺序无关。不写这两行，背景会把整个
 * 界面盖住（实测：元素的 rect、opacity、color 全部正确，就是一个像素都看不见，
 * 看起来像「页面没渲染」）。主程序 StartupPage.vue:858 踩过同一个坑。
 */
.top, .stage, .foot { position: relative; z-index: 1; }

/* ── 无边框窗口：拖动带 + 关闭 ───────────────────────────────────── */
.drag-region {
  position: fixed;
  top: 0; left: 0; right: 0;
  height: 64px;
  z-index: 40; /* 高于内容（1），低于关闭按钮（50） */
  -webkit-app-region: drag;
}

.window-close {
  position: fixed;
  /* 顶栏上留白 22 + 品牌 40 = 62，其竖直中心 42；30px 按钮的 top = 42-15 = 27 */
  top: 27px; right: 20px;
  z-index: 50;
  width: 30px; height: 30px;
  display: flex; align-items: center; justify-content: center;
  border: none;
  border-radius: 8px;
  background: transparent;
  /* ink-3：与 .kicker 同一底色档，实测 5.09:1（浅）/ 4.28:1（深）—— 看得见，但退在后面 */
  color: var(--ink-3);
  font-size: 17px;
  line-height: 1;
  cursor: pointer;
  -webkit-app-region: no-drag;
  transition: background 0.15s ease, color 0.15s ease;
}
/* 悬停变危险色：danger 压在 12% 红底上实测 5.32:1（浅）/ 3.84:1（深）。
   它是悬停反馈 + 非文本图标（1.4.11 的门槛是 3:1），不是正文 —— 所以这里可以低于 4.5。 */
.window-close:hover { background: color-mix(in srgb, var(--danger) 12%, transparent); color: var(--danger); }

/* ── 顶栏：品牌 ─────────────────────────────────────────────────── */
.top {
  flex-shrink: 0;
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  padding: 22px 26px 0;
}

/* 与 OOBE 的 .top-brand 同构：logo 40 → 名称 16 → 版本行 11.5 */
.brand {
  display: flex;
  align-items: center;
  gap: 12px;
  min-width: 0;
  color: var(--accent);
}

.brand-mark { width: 40px; height: 40px; flex-shrink: 0; }

.brand-text { display: flex; flex-direction: column; gap: 1px; min-width: 0; }

.brand-name {
  font-size: 16px;
  font-weight: 600;
  /* 拉丁标识名可以收紧（与 StartupPage 的 .top-brand-name 同值） */
  letter-spacing: -0.4px;
  line-height: 1.2;
  color: var(--ink);
}

.brand-tag {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 11.5px;
  /* ink-2：这一行载着版本号，实测 6.67:1（浅）/ 7.44:1（深）。
     第一版用 ink-4，实测只有 3.38:1 / 2.80:1 —— 深色下**连 3:1 都不到**，
     而 ink-4 的定位本来就是「弱文本」，不该放版本号这种要读的信息。 */
  color: var(--ink-2);
  white-space: nowrap;
}

.beta-badge {
  font-style: normal;
  font-size: 10px;
  font-weight: 600;
  letter-spacing: 0.04em;
  padding: 1px 6px;
  border-radius: 999px;
  /* Beta 是「提醒」不是「错误」：走 --warn 语义色，
     淡底 --warn-tint、描边 --warn-line 在深色下会一起变成低透明琥珀。 */
  background: var(--warn-tint);
  border: 1px solid var(--warn-line);
  color: var(--warn);
}

/* ── 主体 ───────────────────────────────────────────────────────── */
.stage {
  flex: 1 1 auto;
  display: flex;
  flex-direction: column;
  align-items: center;
  /*
   * **必须是 safe center**，不能是 center。
   * 内容比容器高时（协议页在最小窗口下就是），普通 center 会把标题块推到
   * 视口之上并裁掉，而且**滚不回去** —— 滚动位置够不到上边界。
   * safe 关键字让它在放不下时退化为 flex-start，内容从头开始、可以正常滚动。
   * （同一个坑在 StartupPage.vue:923 踩过。）
   */
  justify-content: safe center;
  gap: 20px;
  padding: 18px 40px 6px;
  min-height: 0;
  overflow-y: auto;
}

/* 标题块：kicker / 计数 / 标题 / 说明，一行一条基线 */
.subject {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  min-width: 0;
}

.kicker {
  font-size: 10px;
  font-weight: 400;
  letter-spacing: 0.22em;
  text-transform: uppercase;
  /* ink-3 而不是 ink-4：实测 5.09:1（浅）/ 4.28:1（深）；ink-4 只有 3.38 / 2.80，
     深色下 2.80 连「弱文本」的 3:1 都不到。层级仍然成立 —— 它是 10px + 0.22em 字距，
     往后放它的是字号和字距，不是颜色。 */
  color: var(--ink-3);
}

.counter {
  display: flex;
  align-items: baseline;
  gap: 5px;
  margin-top: 7px;
  font-variant-numeric: tabular-nums;
  color: var(--ink-3);
}
/* 当前步 22px：一眼能读出，但比标题（32px）小一档 —— 它是导航，不是内容。
   原来是 28px 的实心圆点 + 40% 强调色投影，在 5 步这个数量上反而比标题更抢眼。 */
.counter b {
  font-size: 22px;
  font-weight: 600;
  line-height: 1;
  letter-spacing: 0;
  color: var(--ink);
}
.counter i {
  font-style: normal;
  font-size: 12px;
}
.counter span { font-size: 12px; }

.title {
  margin-top: 10px;
  font-size: 32px;
  font-weight: 600;
  /* 字距 0：中文满 em，负字距会把相邻笔画挤到一起。见文件头第 2 条。 */
  letter-spacing: 0;
  line-height: 1.16;
  color: var(--ink); /* 实测 16.95:1（浅）/ 13.13:1（深） */
  text-align: center;
}

.desc {
  margin-top: 9px;
  max-width: 460px;
  font-size: 13px;
  line-height: 1.65;
  letter-spacing: 0;
  /* ink-2：实测 6.28:1（浅）/ 7.15:1（深） */
  color: var(--ink-2);
  text-align: center;
}

/* ── 本步内容块 ─────────────────────────────────────────────────── */
/*
 * 620px：中文正文 13px 时舒适行长是 35–48 字（约 455–624px），
 * 而窗口 880 减去左右各 40 的内边距还有 800 可用 —— 留 90px 两侧余白，
 * 表单读起来不散。再宽（原来 640 的 .panel 放在 1200 窗口里）就只剩空。
 */
.panel {
  width: 100%;
  max-width: 620px;
  display: flex;
  flex-direction: column;
  gap: 11px;
  min-width: 0;
}

/* ── 说明清单（欢迎页的能力 / 更新页的事实）──────────────────────── */
/* 行 + 发丝线的「清单」，不是卡片：并排卡片会被 880 压成三列汉字竖排，
   而且卡片在这套设计里一律无投影，去掉投影之后它只剩一块几乎看不见的底板。 */
.facts { list-style: none; display: flex; flex-direction: column; }

.fact {
  display: flex;
  align-items: flex-start;
  gap: 11px;
  padding: 10px 2px;
  border-top: 1px solid rgba(var(--warm-rgb), 0.16);
}
.fact:first-child { border-top: none; }

.fact > i {
  width: 18px;
  flex-shrink: 0;
  margin-top: 2px;
  font-size: 16px;
  color: var(--accent); /* 按令牌算：浅色 16.09:1 / 深色 5.82:1（图标，非文本） */
  text-align: center;
}

.fact-body { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
.fact-key { font-size: 13px; color: var(--ink); }
.fact-val {
  font-size: 12.5px;
  line-height: 1.5;
  /* 这是**句子**不是标签，所以取 ink-2：实测 6.12–6.29:1（浅）/ 7.22–7.43:1（深）。
     第一版用 ink-3，深色下实测只有 4.32–4.45:1 —— 差一点点到 AA 正文线，
     而这一行正是「保留用户数据不被删除」这类必须读到的说明，不能停在那儿。 */
  color: var(--ink-2);
}

/* 路径用等宽：Windows 路径里的 \ 和 : 用比例字体读起来会串行 */
.mono {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, "Cascadia Mono", monospace;
  font-size: 12px;
  word-break: break-all;
}

/* 版本对比：旧 → 新。只有数字，不要胶囊底 —— 数字本身就是内容 */
.version-row {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 16px;
  padding: 2px 0 6px;
}
.v-block { display: flex; flex-direction: column; align-items: center; gap: 5px; min-width: 0; }
.v-label { font-size: 11px; color: var(--ink-3); letter-spacing: 0.04em; }
.v-value {
  font-size: 21px;
  font-weight: 600;
  line-height: 1.1;
  font-variant-numeric: tabular-nums;
  color: var(--ink-3);
}
.v-new { color: var(--ink); }
.v-arrow { font-size: 17px; color: var(--ink-3); flex-shrink: 0; }

/* GitHub 行：与 facts 同一套「行 + 发丝线」语言，不单独做一张卡 */
.source-row {
  display: flex;
  align-items: center;
  gap: 11px;
  padding-top: 12px;
  border-top: 1px solid rgba(var(--warm-rgb), 0.16);
}
.source-row > i { font-size: 17px; color: var(--ink-3); flex-shrink: 0; }
.source-body { flex: 1; display: flex; flex-direction: column; gap: 2px; min-width: 0; }
.source-body b { font-size: 13px; font-weight: 600; color: var(--ink); }
.source-body span { font-size: 12.5px; color: var(--ink-2); }

/* ── 协议读稿区 ─────────────────────────────────────────────────── */
/*
 * 「凹陷的读稿区」：比页面底色暗一档（暖灰 7% 叠加，浅色下截图实测叠完是 #ebe9e6）。
 *
 * ⚠ 底色上的正文**必须**是 --ink-2 —— 实测 5.65:1（浅）/ 6.76:1（深）。
 *   第一版写的是 --ink-3：同一块底上只有 4.31:1 / 4.00:1，两套都过不了 AA 正文。
 *   同一个色号换个底就掉线，是「把说明文字放进凹底」最容易踩的一脚。
 *   这是把「说明文字」放进凹底时最容易踩的一脚：同色号换个底就掉线。
 */
.eula-box {
  /*
   * 300px：legal_text.rs 现在这份正文在 12.5px/1.75 下排出来是 267px（实测），
   * 装得下就**不滚动** —— 协议这类「必须能读到」的内容，藏在一段内滚动里
   * 等于把阅读变成一次操作。再长的正文由 max-height 兜住，内部滚动，
   * 勾选框不会被顶出视野（880×640 下实测整屏 411px 内容 / 496px 可用高度）。
   */
  max-height: 300px;
  min-height: 150px;
  overflow-y: auto;
  padding: 13px 15px;
  border-radius: 11px;
  background: rgba(var(--warm-rgb), 0.07);
  border: 1px solid rgba(var(--warm-rgb), 0.16);
  font-size: 12.5px;
  line-height: 1.75;
  color: var(--ink-2);
  white-space: pre-wrap;
  text-align: left;
  /*
   * 滚动条自己画。
   * 只靠 color-scheme: dark 是不够的：实测 WebView2/Chromium 的默认滚动条
   * 在深色下仍是一条**浅灰**的轨道 + 两枚箭头按钮，压在近黑的协议区上
   * 比正文还亮（截图里它是最扎眼的东西）。这里用令牌着色，
   * transparent 轨道让它在两套主题下都退到背景里。
   */
  scrollbar-width: thin;
  scrollbar-color: rgba(var(--warm-rgb), 0.45) transparent;
}

/* ── 勾选 ───────────────────────────────────────────────────────── */
.check {
  display: flex;
  align-items: flex-start;
  gap: 9px;
  font-size: 12.5px;
  line-height: 1.55;
  color: var(--ink-2); /* 实测 6.07:1（浅）/ 6.78:1（深） */
  cursor: pointer;
  user-select: none;
}
.check input {
  width: 15px; height: 15px;
  margin-top: 1px;
  flex-shrink: 0;
  accent-color: var(--accent);
  cursor: pointer;
}
.checks { display: flex; gap: 22px; flex-wrap: wrap; }

/* ── 字段 ───────────────────────────────────────────────────────── */
.field { display: flex; flex-direction: column; gap: 7px; min-width: 0; }

/* 字段名用 ink-3 + 12px：标签是导航，不是内容。
   实测 4.80–4.90:1（浅）/ 4.06–4.43:1（深）—— 过辅助文字的 3:1 线，
   但不到正文的 4.5：它是 12px 的标签，不是句子，这里按标签处理。
   原来是 13px/700 —— 靠字重把标签做得和正文一样重，标签就不再是标签。 */
.field-label { font-size: 12px; letter-spacing: 0.02em; color: var(--ink-3); }

.field-row { display: flex; gap: 8px; min-width: 0; }

.text-input {
  flex: 1;
  min-width: 0;
  height: 38px;
  padding: 0 12px;
  font-family: inherit;
  font-size: 12.5px;
  border: 1px solid rgba(var(--warm-rgb), 0.24);
  border-radius: 9px;
  /* panel-4：浅色下是 90% 白，深色下退成实心面 #232019 ——
     两套都是「比页面高一档的输入面」。正文实测 17.84:1（浅）/ 14.33:1（深）。 */
  background: var(--panel-4);
  color: var(--ink);
  outline: none;
}
/* 聚焦环用 color-mix 派生而不是另取一个蓝：强调色换掉时它跟着换。
   16% 与主程序输入框的聚焦环同档。 */
.text-input:focus {
  border-color: var(--accent);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 16%, transparent);
}
/* 只读（卸载页显示目录）：不假装可编辑 */
.text-input[readonly] {
  background: rgba(var(--warm-rgb), 0.07);
  border-color: rgba(var(--warm-rgb), 0.16);
  color: var(--ink-2);
}

.hint {
  display: flex;
  align-items: flex-start;
  gap: 6px;
  font-size: 12px;
  line-height: 1.6;
  /* 同样是句子，取 ink-2：实测 6.07–6.12:1（浅）/ 6.78:1（深）。
     ink-3 在这块底上深色只有 4.06:1（4.5 线以下），ink-4 只有 2.80:1 ——
     两档都量过，才落在这里。 */
  color: var(--ink-2);
}
.hint i { font-size: 13px; margin-top: 1px; flex-shrink: 0; }
.hint-accent { color: var(--ink-2); }
.hint-accent i { color: var(--accent); }

/* ── 进度 ───────────────────────────────────────────────────────── */
.progress { display: flex; flex-direction: column; gap: 9px; min-width: 0; }

.progress-track {
  height: 8px;
  border-radius: 999px;
  /* 轨道 14% 暖灰：对底色的对比实测只有 1.18:1（浅）/ 1.23:1（深），
     达不到 1.4.11 对非文本组件的 3:1 —— 所以百分比数字必须常显，进度不能只靠轨道读。
     有意如此：把轨道调到 3:1 得铺一整条深灰，「安装中」这一屏会被压成一条黑杠。
     填充与轨道的对比是够的（浅色 13.8:1 / 深色 4.16:1），「装到哪」始终看得见。 */
  background: rgba(var(--warm-rgb), 0.14);
  overflow: hidden;
}

.progress-fill {
  height: 100%;
  border-radius: 999px;
  /* 填充取 --accent：浅色 = 墨色（压轨道 14.9:1），深色自动提亮成浅灰 */
  background: var(--accent);
  transition: width 0.25s var(--ease-out);
}

.progress-meta {
  display: flex;
  align-items: baseline;
  gap: 12px;
  min-width: 0;
}
.progress-pct {
  font-size: 13px;
  font-variant-numeric: tabular-nums;
  color: var(--ink);
  flex-shrink: 0;
}
.progress-pct i { font-style: normal; font-size: 10px; color: var(--ink-3); }
.progress-file {
  flex: 1;
  min-width: 0;
  text-align: right;
  font-size: 12px;
  color: var(--ink-3); /* 实测 4.85:1（浅）/ 4.10:1（深）：与百分比形成主次 */
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* ── 状态条 ─────────────────────────────────────────────────────── */
/*
 * 淡底一律用 color-mix(语义色, transparent) 派生，不写死「淡绿/淡红/淡黄」——
 * 深色下那三块写死的浅底会变成三块发光的色板，而语义色自己会变亮，派生底跟着走。
 *
 * ⚠ 正文**不能**用语义色当文字色：
 *   · .ok 的文字用 --ok 时，浅色下 #10b981 压 10% 绿底实测只有 2.08:1；
 *   · .err 的文字用 --danger 时，深色下 #e5534b 压 6% 红底实测 4.49:1（≈4.5，
 *     但不达标）。浅色那侧是 5.33:1，两侧不一致 —— 说明这条路本身不稳。
 * 所以：语义色只给图标，正文一律 --ink —— 实测 14.71:1（错误块，浅）/ 14.82:1（完成块，浅）
 * / 13.2:1（深色按令牌算）。语义色自己只承担图标与描边。
 * 主程序里 --ok 也只用在图标上，正文始终是墨色。
 */
.status {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  padding: 10px 14px;
  border-radius: 11px;
  font-size: 12.5px;
  line-height: 1.6;
  text-align: left;
  min-width: 0;
}
.status > i { font-size: 16px; margin-top: 1px; flex-shrink: 0; }
.status-text { min-width: 0; white-space: pre-line; word-break: break-word; }

.status.ok { background: color-mix(in srgb, var(--ok) 10%, transparent); color: var(--ink); }
.status.ok > i { color: var(--ok); }

.status.err { background: color-mix(in srgb, var(--danger) 6%, transparent); color: var(--ink); }
.status.err > i { color: var(--danger); }

.status.working { background: color-mix(in srgb, var(--accent) 8%, transparent); color: var(--ink); }
.status.working > i { color: var(--accent); }

/* 中性说明（Store 版提示）：不是错误，不该穿红色 */
.status.note {
  background: rgba(var(--warm-rgb), 0.09);
  border: 1px solid rgba(var(--warm-rgb), 0.16);
  color: var(--ink-2);
}
.status.note > i { color: var(--ink-3); }

.status.warn {
  align-items: center;
  background: var(--warn-tint);
  border: 1px solid var(--warn-line);
  /* warn 压 warn-tint：浅色实测 4.51:1（正好在 AA 正文线上）/ 深色 5.59:1（按令牌合成算） */
  color: var(--warn);
}
.status.warn > i { font-size: 18px; }
.warn-body { flex: 1; display: flex; flex-direction: column; gap: 2px; min-width: 0; }
.warn-body b { font-size: 12.5px; font-weight: 600; }
.warn-body span { font-size: 12px; white-space: normal; }

.actions { display: flex; justify-content: center; gap: 9px; padding-top: 2px; }

/* ── 按钮 ───────────────────────────────────────────────────────── */
/*
 * 面板内按钮（浏览 / 退出应用 / 启动 / 关闭）：36px 高、圆角 9，
 * 与旁边的输入框（38px / 圆角 9）同一组形制 —— 它是「字段级」的动作。
 * 底部导航另有一套 .nav-btn（全圆角药丸），因为那是浮起来的导航条。
 */
.btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  height: 36px;
  padding: 0 16px;
  font-family: inherit;
  font-size: 13px;
  font-weight: 600;
  border: 1px solid transparent;
  border-radius: 9px;
  cursor: pointer;
  white-space: nowrap;
  transition: background 0.15s ease, border-color 0.15s ease, color 0.15s ease, transform 0.15s var(--ease-out), box-shadow 0.15s ease;
}
.btn i { font-size: 15px; line-height: 1; }
.btn:disabled { opacity: 0.45; cursor: not-allowed; }

/* 主按钮：--accent 面 + --surface 字。
   浅色 17.99:1，深色 5.34:1（#929496 面 / #232019 字）—— 两套都过 AA 正文。
   写死 #fff 在深色下只有 3.04:1，过不了。 */
.btn.primary {
  background: var(--accent);
  color: var(--surface);
  /* 主操作是唯一允许「轻微抬升」的面内元素（dialog-spec 2.4） */
  box-shadow: 0 3px 10px color-mix(in srgb, var(--accent) 22%, transparent);
}
.btn.primary:hover:not(:disabled) { transform: translateY(-1px); }

/* 次级按钮：透明底 + 发丝描边，不抢主操作 */
.btn.ghost {
  background: transparent;
  border-color: rgba(var(--warm-rgb), 0.28);
  color: var(--ink-2); /* 实测 6.07:1（浅）/ 6.78:1（深） */
}
.btn.ghost:hover:not(:disabled) {
  background: rgba(var(--warm-rgb), 0.09);
  color: var(--ink);
}

.btn.sm { height: 30px; padding: 0 12px; font-size: 12px; border-radius: 8px; flex-shrink: 0; }

/* ── 底部药丸导航（OOBE .nav-pill 同款）──────────────────────────── */
.foot {
  flex-shrink: 0;
  display: flex;
  justify-content: center;
  padding: 8px 26px 22px;
}

.nav-pill {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px;
  background: var(--panel-4);
  border: 1px solid rgba(var(--warm-rgb), 0.2);
  border-radius: 999px;
  /* 它脱离文档流的视觉位置（浮在内容与底边之间），是这套设计里少数该有投影的元素 */
  box-shadow: 0 10px 30px rgba(var(--ink-rgb), 0.1);
  backdrop-filter: blur(16px);
  -webkit-backdrop-filter: blur(16px);
  transition: transform 0.3s var(--ease-spring), box-shadow 0.3s ease;
}
.nav-pill:hover {
  transform: translateY(-1px);
  box-shadow: 0 14px 34px rgba(var(--ink-rgb), 0.13);
}

.nav-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  height: 38px;
  padding: 0 24px;
  font-family: inherit;
  font-size: 13.5px;
  font-weight: 600;
  border: none;
  border-radius: 999px;
  cursor: pointer;
  transition: background 0.16s ease, color 0.16s ease, opacity 0.16s ease;
}
.nav-btn i { font-size: 15px; line-height: 1; }
.nav-btn:disabled { opacity: 0.45; cursor: not-allowed; }

.nav-btn.primary { background: var(--accent); color: var(--surface); }
.nav-btn.primary:hover:not(:disabled) { background: color-mix(in srgb, var(--accent) 88%, var(--ink)); }

.nav-btn.ghost { background: transparent; color: var(--ink-3); }
.nav-btn.ghost:hover:not(:disabled) { background: rgba(var(--warm-rgb), 0.09); color: var(--ink-2); }

/*
 * 卸载是危险操作，但按钮不做成实心红 —— 向导里唯一实心的应该是主操作，
 * 而这一屏没有别的主操作，实心红会跟「安装」按钮抢同一个位置的含义。
 *
 * ⚠ 危险色**只给图标和描边，文字用 --ink**。
 *   实测：--danger 当文字压在 12% 红底上，浅色 5.85:1（读得清），
 *   深色只有 3.84:1 —— 深色下 --danger #e5534b 太暗，既压不住浅字也亮不过红底。
 *   改用 --ink 之后是 14.7:1 / 13.2:1，两套主题一致；危险语义交给
 *   红色图标 + 红色描边 + 红色底，与 .status.err 的处理保持同一套规则。
 */
.nav-btn.danger {
  background: color-mix(in srgb, var(--danger) 12%, transparent);
  border: 1px solid color-mix(in srgb, var(--danger) 34%, transparent);
  color: var(--ink);
}
.nav-btn.danger i { color: var(--danger); }
.nav-btn.danger:hover:not(:disabled) { background: color-mix(in srgb, var(--danger) 20%, transparent); }

/* ── 动效 ───────────────────────────────────────────────────────── */
/*
 * 步骤切换：淡入 + 6px 上浮。
 * 不用位移作为主信息（OOBE 的墨水揭示那套需要 view-transition，安装器不开这条路），
 * 也不给卡片做 scale —— scale 会让元素成为包含块，父级里任何一个
 * position: fixed 的浮层都会改相对定位（dialog-spec 3.2 踩过：对话框遮罩只盖住半屏）。
 */
.step-enter-active,
.step-leave-active {
  transition: opacity 0.18s var(--ease-out), transform 0.18s var(--ease-out);
}
.step-enter-from { opacity: 0; transform: translateY(6px); }
.step-leave-to { opacity: 0; transform: translateY(-4px); }

/*
 * 旋转图标。
 * ⚠ remixicon 4.6 **没有** ri-spin 这个类（实测 grep 它的字体 CSS：只有
 *   图标字形，没有任何 spin 关键帧），主程序是在 shared.css 里自己补的
 *   （.spin + @keyframes ri-spin）。安装器原来只写了 `.ri-spin { display: inline-block }`
 *   —— 于是「安装中…」「正在退出…」旁边的加载图标是一枚**静止**的图标，
 *   看起来像卡住。这里把动画补齐。
 */
.ri-spin { display: inline-block; animation: ri-spin 1s linear infinite; }
@keyframes ri-spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

@media (prefers-reduced-motion: reduce) {
  .step-enter-active,
  .step-leave-active { transition: none; }
  .nav-pill { transition: none; }
  .ri-spin { animation-duration: 3s; }
}
</style>
