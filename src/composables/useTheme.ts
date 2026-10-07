/**
 * 主题：浅色 / 深色 / 跟随系统。
 *
 * 状态存在模块级单例里，理由和 useFilePicker 一样 —— 切换按钮在首页，
 * 但 `data-theme` 要挂在 <html> 上，两边不共享一个 ref 就会不同步。
 *
 * 三档而不是两档：`system` 是默认值，因为它对「没表达过偏好」的用户是
 * 唯一正确的答案。用户手动切一次之后才落盘，此后就按他选的来。
 */
import { computed, readonly, ref } from 'vue';

export type ThemeMode = 'light' | 'dark' | 'system';

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
 * 文本刷新：**必须等快照结束之后**。
 *
 * 第一版把它压在墨水收尾处（duration - 300ms），以为两段能接上 ——
 * 那是错的。View Transition 期间 `::view-transition-new(root)` 显示的是一张
 * **冻结的新内容快照**，此刻对 DOM 做的任何动画都不会被渲染。元素上
 * `animationName` 查得到、`refreshing` 类也挂上了，但画面上一个像素都不变。
 *
 * 所以改成 `.finished` 之后再起步。为了让两段不脱节，墨水最后一段本来
 * 就收得很慢（cubic-bezier 尾段），快照撤掉时墨已经基本铺满，刷新紧接着
 * 落下来读起来是同一件事的收尾。
 */
function refreshText(root: HTMLElement) {
  root.classList.add('refreshing');
  // 最后一个字的延迟 = --r-i 最大值(9) × 26ms，加上动画本身 520ms
  window.setTimeout(() => root.classList.remove('refreshing'), 520 + 26 * 10);
}

/**
 * 切换主题，带「墨水泼洒」的揭示动画。
 *
 * 用 View Transitions：浏览器给旧画面拍一张快照，新主题渲染在下层，
 * 然后我们用一个遮罩把快照揭开。好处是不用给每个元素挂过渡 ——
 * 那种做法要遍历整棵树，这里只有一次合成。
 *
 * 不支持时（老 WebView）回落到原来的颜色过渡，不会变成硬切。
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
      refreshText(root);
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
  revealTheme(apply);
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
