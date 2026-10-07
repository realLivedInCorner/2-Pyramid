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
 * 四个墨点的终点半径每次都不同 —— 固定值会让每次切换长得一模一样，
 * 那就成了一个转场特效，不是墨。抖动 ±18%：太小看不出差别，太大
 * 偶尔会有一个圆明显没盖到边（虽然终点都过 150%，但视觉上会偏）。
 *
 * 半径之间的**相对大小**也随机化（谁大谁小不固定），否则四次切换下来
 * 还是能看出是同一组形状。
 */
function rollInk() {
  const root = document.documentElement;
  const jitter = (base: number) => Math.round(base * (0.82 + Math.random() * 0.36));
  const names = ['--ink-end-a', '--ink-end-b', '--ink-end-c', '--ink-end-d'];
  // 基准值刻意拉开：190 / 165 / 205 / 175，各自负责一块区域
  const bases = [190, 165, 205, 175];
  for (let i = 0; i < names.length; i++) {
    root.style.setProperty(names[i], `${jitter(bases[i])}%`);
  }
  // 时长也轻微浮动 —— 每次都一样长会显得很机械
  root.style.setProperty('--ink-duration', `${1080 + Math.round(Math.random() * 220)}ms`);
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
  rollInk();

  // 快照期间把光标也冻住，避免动画中途 hover 改了底色
  const root = document.documentElement;
  root.classList.add('theme-inking');
  start.call(document, () => {
    applyNext();
  }).finished.finally(() => {
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
