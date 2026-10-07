/**
 * 主题：浅色 / 深色 / 跟随系统。
 *
 * 状态的**真源是配置文件**（`theme_mode`），不是 localStorage —— 它是用户
 * 偏好，应当跟着配置一起备份、重置、迁移。
 *
 * 但首帧不能等 IPC：配置文件要先 invoke 才能读到，那期间页面已经画出来了，
 * 深色用户每次启动都会先闪一下白。所以 localStorage 留一份**只读缓存**，
 * 由 index.html 里的内联脚本在首帧之前应用；App 启动后再从配置拉一次，
 * 以配置为准并回写缓存。两边不一致时配置赢。
 */
import { computed, readonly, ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';

export type ThemeMode = 'light' | 'dark' | 'system';

/** 首帧缓存。真源是配置里的 theme_mode。 */
const STORAGE_KEY = 'themeMode';

const stored = localStorage.getItem(STORAGE_KEY);
const mode = ref<ThemeMode>(
  stored === 'light' || stored === 'dark' || stored === 'system' ? stored : 'system',
);

/** 系统是否偏好深色。跟随模式下由它决定实际生效的主题。 */
const systemDark = ref(
  typeof window !== 'undefined' && window.matchMedia
    ? window.matchMedia('(prefers-color-scheme: dark)').matches
    : false,
);

if (typeof window !== 'undefined' && window.matchMedia) {
  window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', (e) => {
    systemDark.value = e.matches;
    if (mode.value === 'system') apply();
  });
}

/** 实际生效的主题 —— 按钮显示哪个图标由它决定，不是由 mode 决定。 */
const resolved = computed<'light' | 'dark'>(() =>
  mode.value === 'system' ? (systemDark.value ? 'dark' : 'light') : mode.value,
);

/**
 * 切换期间临时挂一个类，让颜色平滑过渡。
 *
 * 不常驻的原因：`transition` 加在全局每个元素上，会让所有交互（悬停、选中、
 * 列表滚动）都背上一次合成开销。只在真正切换的那 320ms 里开，代价一次付清。
 */
let switchTimer: number | undefined;

/** 老浏览器回落用：临时挂类，让颜色自己做过渡。 */
function transitionThenSettle() {
  const root = document.documentElement;
  // 减少动效时直接切，不要这段过渡
  const reduced =
    document.body.classList.contains('motion-reduced') ||
    window.matchMedia?.('(prefers-reduced-motion: reduce)').matches;
  if (reduced) return;

  root.classList.add('theme-switching');
  if (switchTimer !== undefined) window.clearTimeout(switchTimer);
  switchTimer = window.setTimeout(() => {
    root.classList.remove('theme-switching');
    switchTimer = undefined;
  }, 340);
}

/**
 * 给这次泼洒掷一次「落笔」。
 *
 * 每次都不一样的三件事，缺一就变成固定的转场特效而不是墨：
 *   1. 终点半径 ±18%，且**相对大小不固定**（谁大谁小会变）
 *   2. 时长在 1080–1300ms 之间浮动
 *   3. **切入方向上下交替** —— 上切时四点贴上边缘横排向下胀开，
 *      下切时贴下边缘向上胀开
 */
const lastDirKey = 'themeInkDir';

function rollInk() {
  const root = document.documentElement;

  // 方向上下交替。存在模块外（localStorage）是为了跨刷新也保持交替，
  // 否则每次启动都从同一个方向开始。
  const prev = localStorage.getItem(lastDirKey);
  const dir = prev === 'top' ? 'bottom' : 'top';
  localStorage.setItem(lastDirKey, dir);

  /*
   * 四点沿切入边的横排位置。两侧留出余量（-4% / 96%），
   * 让最外两个圆向屏幕外溢，边角才不会有没盖到的地方。
   * 顺序刻意打乱（6/38/72/94 而不是 8/35/62/89）—— 均匀分布看起来像刻度。
   */
  const xs = [6, 38, 72, 94];
  const ys = dir === 'top'
    ? [2, -3, 5, 0]      // 贴上边缘
    : [98, 103, 95, 100]; // 贴下边缘

  const jitter = (base: number) => Math.round(base * (0.82 + Math.random() * 0.36));
  const names = ['--ink-end-a', '--ink-end-b', '--ink-end-c', '--ink-end-d'];
  const bases = [190, 165, 205, 175];

  for (let i = 0; i < 4; i++) {
    root.style.setProperty(`--ink-pos-${'abcd'[i]}`, `${xs[i]}% ${ys[i]}%`);
    root.style.setProperty(names[i], `${jitter(bases[i])}%`);
  }
  root.style.setProperty('--ink-duration', `${1080 + Math.round(Math.random() * 220)}ms`);
  return dir;
}

/**
 * 切换主题，带「墨水泼洒」的揭示动画。
 *
 * 用 View Transitions：浏览器给旧画面拍一张快照，新主题渲染在下层，
 * 然后我们用一个遮罩把快照揭开。好处是不用给每个元素挂过渡 ——
 * 那种做法要遍历整棵树，这里只有一次合成。
 *
 * 不支持时（老 WebView）回落到原来的颜色过渡，不会变成硬切。
 *
 * 文本刷新已移除。它做不到和墨水同步：View Transition 期间
 * `::view-transition-new(root)` 显示的是一张**冻结的新内容快照**，
 * 此刻对 DOM 做的动画不会被渲染（实测差异 0 像素）。只能等 `.finished`
 * 之后再做，那就成了两个分开的节拍 —— 与其硬凑，不如不做。
 */
function revealTheme(applyNext: () => void) {
  const reduced =
    document.body.classList.contains('motion-reduced') ||
    window.matchMedia?.('(prefers-reduced-motion: reduce)').matches;

  const start = (
    document as Document & { startViewTransition?: (cb: () => void) => { finished: Promise<void> } }
  ).startViewTransition;

  if (reduced || typeof start !== 'function') {
    transitionThenSettle();
    applyNext();
    return;
  }

  // 落笔必须在 startViewTransition 之前定下来：伪元素的动画名字在快照
  // 建立时就固定了，之后再改自定义属性不会重新解析关键帧。
  const dir = rollInk();

  // 快照期间把光标也冻住，避免动画中途 hover 改了底色
  const root = document.documentElement;
  root.classList.add('theme-inking');
  // 切入方向挂到 html 上，页面可以据此调整自己的编排
  root.dataset.inkDir = dir;

  start
    .call(document, () => {
      applyNext();
    })
    .finished.finally(() => {
      root.classList.remove('theme-inking');
    });
}

/** 把当前主题写到 <html data-theme>。没有这个属性就是浅色。 */
export function apply() {
  const root = document.documentElement;
  if (resolved.value === 'dark') root.setAttribute('data-theme', 'dark');
  else root.removeAttribute('data-theme');

  // 让原生控件（滚动条、输入框）也跟着走，否则深色下会露出一条白滚动条
  root.style.colorScheme = resolved.value;
}

export function setThemeMode(next: ThemeMode) {
  if (mode.value === next) return;
  mode.value = next;
  localStorage.setItem(STORAGE_KEY, next);
  // 真源是配置。写失败不回滚内存状态 —— 界面已经切过去了，把它翻回来
  // 反而更让人困惑；下次启动会以配置为准，那是它该有的行为。
  void persist(next);
  revealTheme(apply);
}

/**
 * 从配置里拉一次主题，以它为准。
 *
 * 在 App 启动时调用。拿不到（老版本配置没有这个字段、或 IPC 失败）就保持
 * 现有状态不动 —— 此时它来自 localStorage 缓存，已经是可用的值。
 */
export async function syncThemeFromConfig() {
  try {
    const cfg = await invoke<{ theme_mode?: string; themeMode?: string }>('get_config');
    const stored = cfg?.theme_mode ?? cfg?.themeMode;
    if (stored === 'light' || stored === 'dark' || stored === 'system') {
      mode.value = stored;
      localStorage.setItem(STORAGE_KEY, stored);
    } else {
      // 配置里还没有这个字段（旧配置）—— 把当前选择补写进去，
      // 否则用户这次的选择只活在 localStorage 里，重装就丢。
      void persist(mode.value);
    }
  } catch {
    // 读不到配置不影响渲染，localStorage 那份已经生效了
  }
  apply();
}

function persist(value: ThemeMode) {
  return invoke('update_config', { patch: { themeMode: value } }).catch(() => {
    // 静默：配置写失败不该弹错。主题已经应用，只是下次启动回退到缓存值。
  });
}

/** 按钮用：在浅色与深色之间切换。系统档按当前实际颜色切到它的反面。 */
export function toggleTheme() {
  setThemeMode(resolved.value === 'dark' ? 'light' : 'dark');
}

export function useTheme() {
  return {
    mode: readonly(mode),
    resolved,
    isDark: computed(() => resolved.value === 'dark'),
    setThemeMode,
    toggleTheme,
    apply,
  };
}
