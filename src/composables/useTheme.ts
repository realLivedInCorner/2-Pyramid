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
  transitionThenSettle();
  mode.value = next;
  localStorage.setItem(STORAGE_KEY, next);
  apply();
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
