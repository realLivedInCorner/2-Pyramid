<template>
  <div class="home" :data-boot="bootPhase">
    <!--
      ══ 背景 ══
      全应用共用的一块地面（StageBackdrop）。固定不动，不参与交互。
      抽出来是为了让设置页、转换页踩在同一块底上 —— 切页面时背景不动，只有内容在换。
      revealed 由加载页过渡的 bootPhase 驱动：阶段 1 起随飞行一起淡入。
    -->
    <StageBackdrop :revealed="bootPhase >= 1" />

    <!--
      左上：品牌三行 —— 标识 + 名称 / 标语 / 版本与引擎状态。
      原先「版本 · 引擎就绪」是独立绝对定位的，和品牌块没有关系；
      并进来之后左上就是一个完整的身份块，位置也不用两处各调一次。
      hm-block：本组件自己的 bootPhase 编排入场，
      必须在 App.vue 的全局 stagger 里排除掉，否则会被那条 (0,6,0) 的规则盖掉。
      .brand-mark 是加载页 logo 的落点：App.vue 量它的坐标做 FLIP，
      所以它在 bootPhase 0/1 时必须保持占位（用 visibility，不能用 display:none）。
    -->
    <header class="brand hm-block">
      <div class="brand-top">
        <img class="brand-mark" :src="logo" alt="" aria-hidden="true" />
        <span class="brand-name"><SplitText :text="'2-Pyramid'" :stagger="30" :delay="120" :active="bootPhase >= 2" /></span>
      </div>
      <span class="brand-tag">{{ t('oobe.asideTagline') }}</span>
      <span class="brand-meta">
        v{{ appVersion }}<template v-if="appIsBeta"> Beta</template>
        <i aria-hidden="true"></i>
        {{ t('home.engineReady') }}
      </span>
    </header>

    <!--
      右上角：使用者与时钟 + 一组纯装饰。
      窗口控制按钮占据 y 10–54，所以这一列从 y 64 起排 —— 不与之重叠。
      时钟走的是本地时间，每秒一次；组件卸载时清掉定时器。
    -->
    <div class="who hm-block">
      <span class="who-name">{{ userName || '—' }}</span>
      <i aria-hidden="true"></i>
      <span class="who-time">{{ clock }}</span>
    </div>

    <!-- 纯装饰：阶梯标记 + 十字准星 + 刻度尺。没有文字，也不可交互 -->
    <div class="deco hm-block" aria-hidden="true">
      <span class="deco-steps"><i></i><i></i><i></i><i></i><i></i></span>
      <span class="deco-cross"></span>
      <span class="deco-scale"><i v-for="n in 20" :key="n"></i></span>
    </div>

    <!--
      左下：只是「我在看什么」。
      刻意不放按钮、不放切换器 —— 一个角落塞四五样东西，重心就又偏了。
    -->
    <div class="subject hm-block">
      <span class="tiny">MODULE / SELECT</span>
      <div :key="selected" class="subject-body">
        <div class="counter">
          <span class="cur">{{ pad(selected + 1) }}</span>
          <i>/</i>
          <span class="total">{{ pad(modules.length) }}</span>
        </div>
        <h1 class="name">
          <SplitText :text="t(current.label)" :stagger="42" :delay="0" :active="bootPhase >= 2" />
        </h1>
        <span class="name-en">{{ current.en }}</span>
      </div>
    </div>

    <!-- 右下：动作。切换与进入分开两块，不挤在一起 -->
    <div class="actions hm-block">
      <div class="switcher">
        <button class="nav-btn" type="button" :aria-label="t('home.prevModule')" @click="step(-1)">←</button>
        <div class="ticks" role="tablist">
          <button
            v-for="(m, i) in modules"
            :key="m.page"
            type="button"
            role="tab"
            class="tick"
            :class="{ on: i === selected }"
            :aria-selected="i === selected"
            :aria-label="t(m.label)"
            @click="goTo(i)"
          ></button>
        </div>
        <button class="nav-btn" type="button" :aria-label="t('home.nextModule')" @click="step(1)">→</button>
      </div>

      <button class="enter" type="button" @click="enter">
        <span>{{ t('home.enter') }}</span>
        <i class="ri-arrow-right-line" aria-hidden="true"></i>
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue';
import { useI18n } from 'vue-i18n';
import logo from '../assets/logo-512.png';
import { useAppInfo } from '../composables/useAppInfo';
import SplitText from './SplitText.vue';
import StageBackdrop from './shell/StageBackdrop.vue';

const { t } = useI18n();
const props = withDefaults(
  defineProps<{
    userName?: string;
    editorMode?: boolean;
    /** 0 隐藏 / 1 背景显现、标识在飞 / 2 落地并逐项入场。默认直接就是就绪态 */
    bootPhase?: number;
  }>(),
  { bootPhase: 2 },
);
const bootPhase = computed(() => props.bootPhase);
const emit = defineEmits(['switch-page']);

const { version: appVersion, isBeta: appIsBeta } = useAppInfo();

/**
 * 本地时钟。
 * 每秒对一次系统时间 —— 用 setInterval(1000) 累加会在标签页挂起后走偏。
 * 这里每次直接读 Date，间隔只决定刷新频率，不参与计时。
 */
const clock = ref('');
let clockTimer: number | undefined;
const tickClock = () => {
  const d = new Date();
  const p = (n: number) => String(n).padStart(2, '0');
  clock.value = `${p(d.getHours())}:${p(d.getMinutes())}:${p(d.getSeconds())}`;
};
const pad = (n: number) => String(n).padStart(2, '0');

const modules = [
  { page: 'conversion', en: 'MATERIAL CONVERTER', label: 'home.navConvert' },
  { page: 'overlay',    en: 'OVERLAY WORKSHOP',   label: 'home.navOverlay' },
  { page: 'settings',   en: 'PREFERENCES',        label: 'home.navSettings' },
] as const;

const selected = ref(0);
const current = computed(() => modules[selected.value]);

/** 循环切换：到末尾再按 → 回到第一列 */
const step = (delta: number) => {
  const n = modules.length;
  selected.value = (selected.value + delta + n) % n;
};

/** 直接选中某一档 */
const goTo = (i: number) => { selected.value = i; };

const enter = () => emit('switch-page', current.value.page);

const onKey = (e: KeyboardEvent) => {
  const el = document.activeElement;
  const onControl = el instanceof HTMLElement
    && ['BUTTON', 'INPUT', 'TEXTAREA', 'SELECT'].includes(el.tagName);
  if (e.key === 'ArrowLeft') { step(-1); return; }
  if (e.key === 'ArrowRight') { step(1); return; }
  if (e.key === 'Enter' && !onControl) { e.preventDefault(); enter(); }
};

onMounted(() => {
  window.addEventListener('keydown', onKey);
  tickClock();
  clockTimer = window.setInterval(tickClock, 1000);
});
onUnmounted(() => {
  window.removeEventListener('keydown', onKey);
  // 不清会留下一个每秒跑一次、却指向已卸载组件闭包的定时器
  if (clockTimer !== undefined) window.clearInterval(clockTimer);
});
</script>

<style scoped>
/*
 * ══ 首页 ═══════════════════════════════════════════════════════════════
 *
 * 四条约束（都是之前踩过的）：
 *  ① 右上角永远留空 —— 窗口控制按钮在那儿
 *  ② 不用任何居中元素 —— 居中的潜台词就是「看这里」
 *  ③ 不制造重点 —— 默认安静，只有用户选中的那一项才亮起来
 *  ④ 信息分散在四角，不让任何一角承载过多
 *
 * 背景不含任何可辨认的物件：只有光、纸纹和品牌三角的形。没有实体，就没有抢焦点的东西。
 */

.home {
  position: relative;
  width: 100%;
  height: 100%;
  min-height: 0;
  overflow: hidden;
  color: var(--ink);
  background: var(--bg);
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", "PingFang SC", "Microsoft YaHei", sans-serif;
}

/* ══════════ 加载页 → 首页的入场编排 ══════════
 *
 * bootPhase:
 *   0  全部不可见。**但 .brand-mark 必须保持占位** —— App.vue 要量它的坐标
 *      让加载页的 logo 飞过来，所以这里用 visibility 而不是 display:none / opacity。
 *   1  背景先行显现，标识在飞（此刻首页的 logo 仍然藏着，避免两个影子重叠）。
 *   2  落地。各组件的入场延迟见下方 --rd。
 *
 * 顺序是刻意的：先背景（空间先立起来），再标识（飞行落点），
 * 然后四角依次入场，最后才是每块内部的文字逐字显性。
 */
.home[data-boot="0"] .brand,
.home[data-boot="0"] .who,
.home[data-boot="0"] .deco,
.home[data-boot="0"] .subject,
.home[data-boot="0"] .actions { opacity: 0; }

.home[data-boot="1"] .brand,
.home[data-boot="1"] .who,
.home[data-boot="1"] .deco,
.home[data-boot="1"] .subject,
.home[data-boot="1"] .actions { opacity: 0; }

/* 标识在飞 —— 首页这一份先藏着，落地那一帧才亮出来 */
.home[data-boot="0"] .brand-mark,
.home[data-boot="1"] .brand-mark { visibility: hidden; }

/* 落地帧：标识直接出现在飞行终点，不播动画（飞行的本体在加载页那一层） */
.home[data-boot="2"] .brand-mark { visibility: visible; animation: none; }

/* 背景的淡入由 StageBackdrop 的 revealed 属性接管，这里不再重复控制 */

/* 四角依次入场。--rd 是各自的延迟，错开一拍 */
.home[data-boot="2"] .brand,
.home[data-boot="2"] .who,
.home[data-boot="2"] .deco,
.home[data-boot="2"] .subject,
.home[data-boot="2"] .actions {
  /* backwards 而不是 both：both 会永久保留 translate: 0 0，元素成为包含块，
     内部任何 position: fixed 都会相对它定位而不是视口。 */
  animation: boot-block 700ms var(--ease-out, cubic-bezier(0.22, 1, 0.36, 1)) backwards;
  animation-delay: var(--rd, 0ms);
}
.home .brand { --rd: 0ms; }
.home .who { --rd: 140ms; }
.home .deco { --rd: 70ms; }
.home .subject { --rd: 210ms; }
.home .actions { --rd: 300ms; }

@keyframes boot-block {
  from { opacity: 0; translate: 0 12px; }
  to   { opacity: 1; translate: 0 0; }
}

/* ══════════ 左上：品牌 ══════════ */
/*
 * 首页上唯一放大的一块。其余三个角都刻意压得很小 ——
 * 品牌标识承担识别，不需要靠其他元素一起大声说话。
 *
 * logo 的视觉墨迹在 512 画布里只占 46%×50%（四周留白 L28.7% T27.3% R25.8% B22.9%），
 * 所以 64px 的盒子实际只有约 30px 可见，用负 margin 把四周的空白吃掉。
 */
.brand {
  position: absolute;
  z-index: 3;
  left: 40px;
  top: 22px;
  /* 竖直堆叠：第一行是 logo + 名字，下面两行回到块的左缘。
     之前是整个块横向 flex，于是标语和版本都跟着名字往右缩进了 62px。 */
  display: flex;
  flex-direction: column;
  align-items: flex-start;
}

/* logo 与名字同处一行 */
.brand-top { display: flex; align-items: center; gap: 16px; }

.brand-mark { width: 64px; height: 64px; margin: -13px 0 -15px -18px; object-fit: contain; }
.brand-name { font-size: 30px; font-weight: 700; letter-spacing: -0.025em; line-height: 1.1; }
/*
 * 标语与版本：直接作为 .brand 的子元素，左缘 = 块的左缘（40px）。
 * logo 的墨迹左缘也是 40.4px（盒子 22 + 画布留白 28.7% × 64），
 * 所以这三行在视觉上落在同一条左侧线上 —— 这就是「对齐边框」。
 */
.brand-tag {
  margin-top: 6px;
  font-size: 11.5px;
  font-weight: 400;
  letter-spacing: 0.04em;
  color: var(--ink-3);
}
.brand-meta {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  margin-top: 5px;
  font-size: 10.5px;
  font-weight: 400;
  letter-spacing: 0.12em;
  color: var(--ink-3);
}
/*
 * logo 的视觉墨迹在 512 画布里只占 45%×49%（四周留白 L28.9% T27.3% R26.0% B23.0%），
 * 所以 64px 的盒子实际只有约 29×31px 可见，靠负 margin 把四周空白吃掉。
 *
 * 上边距是 −13 而不是 −17：
 * 「2-Pyramid」里的 y 有下伸部，它的**墨迹**中心比盒子中心低约 3.8px。
 * 拿盒子对盒子会让 logo 看着偏高 4px（实测 logo 墨迹中心 38.3 / 名称墨迹中心 42.3），
 * 按墨迹中心对齐才对得上。
 */
.brand-meta i { width: 3px; height: 3px; border-radius: 50%; background: var(--ink-4); }

/* ══════════ 右上角：使用者与时钟 ══════════ */
/*
 * 窗口控制按钮占据 y 10–54，所以从 64 起排 —— 与按钮区不重叠。
 * 与下方的装饰组同处一列（right: 40），竖直叠放。
 */
.who {
  position: absolute;
  z-index: 3;
  right: 40px;
  top: 64px;
  display: inline-flex;
  align-items: center;
  gap: 9px;
  font-size: 11px;
  font-weight: 400;
  letter-spacing: 0.1em;
  color: var(--ink-3);
}
.who i { width: 3px; height: 3px; border-radius: 50%; background: var(--ink-4); }
/* 时钟用等宽数字，秒位跳动时整行不会左右晃 */
.who-time { font-variant-numeric: tabular-nums; color: var(--ink); }

/* ══════════ 右上角的修饰元素 ══════════ */
/* 装饰组下移，给上面的使用者/时钟让位 */
.deco {
  position: absolute;
  z-index: 2;
  right: 40px;
  top: 100px;
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 14px;
  pointer-events: none;
}

/* 十字准星 */
.deco-cross { position: relative; width: 17px; height: 17px; color: rgba(var(--ink-rgb), 0.28); }
.deco-cross::before,
.deco-cross::after { content: ""; position: absolute; background: currentColor; }
.deco-cross::before { left: 50%; top: 0; width: 1px; height: 100%; transform: translateX(-50%); }
.deco-cross::after { top: 50%; left: 0; height: 1px; width: 100%; transform: translateY(-50%); }

/* 阶梯标记：五级递增的细条，是品牌阶梯锥的缩小记号 */
.deco-steps { display: flex; align-items: flex-end; gap: 3px; height: 16px; }
.deco-steps i { width: 4px; background: rgba(var(--ink-rgb), 0.28); }
.deco-steps i:nth-child(1) { height: 4px; }
.deco-steps i:nth-child(2) { height: 7px; }
.deco-steps i:nth-child(3) { height: 10px; }
.deco-steps i:nth-child(4) { height: 13px; }
.deco-steps i:nth-child(5) { height: 16px; background: rgba(var(--ink-rgb), 0.28); }

/* 刻度尺：每 4 格一根长刻度 */
.deco-scale { display: flex; align-items: flex-end; gap: 4px; height: 15px; }
.deco-scale i { width: 1px; height: 6px; background: rgba(var(--ink-rgb), 0.28); }
.deco-scale i:nth-child(4n + 1) { height: 14px; background: rgba(var(--ink-rgb), 0.45); }

/* ══════════ 左下：我在看什么 ══════════ */
.subject {
  position: absolute;
  z-index: 3;
  left: 40px;
  bottom: 36px;
  display: flex;
  flex-direction: column;
  align-items: flex-start;
}
.tiny { font-size: 10px; font-weight: 400; letter-spacing: 0.2em; color: var(--ink-3); }

/*
 * 切换模块时整块重挂载。
 * 序号与英文标签只做淡入 —— 模块名交给 SplitText 逐字显性，
 * 所以这里不再做位移，否则两个动画叠在一起会互相打架。
 */
.subject-body {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
}
.subject-body > .counter,
.subject-body > .name-en {
  animation: subject-fade 420ms var(--ease-out, cubic-bezier(0.22, 1, 0.36, 1)) both;
}
@keyframes subject-fade {
  from { opacity: 0; }
  to   { opacity: 1; }
}

.counter {
  display: flex;
  align-items: baseline;
  gap: 6px;
  margin-top: 6px;
  font-weight: 300;
  color: var(--ink);
  font-variant-numeric: tabular-nums;
}
.counter .cur { font-size: 42px; line-height: 1; }
.counter i { font-style: normal; font-size: 21px; color: var(--ink-4); }
.counter .total { font-size: 21px; color: var(--ink-3); }

/* 模块名默认就是普通深灰，不是「已高亮」的状态 —— 重点交给用户的选择去决定 */
.name {
  margin: 16px 0 0;
  font-size: 44px;
  font-weight: 700;
  letter-spacing: -0.03em;
  line-height: 1.05;
  color: var(--ink);
}
.name-en {
  margin-top: 8px;
  font-size: 10.5px;
  font-weight: 400;
  letter-spacing: 0.16em;
  color: var(--ink-3);
}

/* ══════════ 右下：动作 ══════════ */
.actions {
  position: absolute;
  z-index: 3;
  right: 40px;
  bottom: 36px;
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 16px;
}

.switcher { display: inline-flex; align-items: center; gap: 12px; }

/* 圆角克制：4px，不是胶囊也不是大圆角 */
.nav-btn {
  width: 36px;
  height: 36px;
  border: 1px solid rgba(var(--ink-rgb), 0.18);
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.5);
  color: var(--ink);
  font-family: inherit;
  font-size: 16px;
  font-weight: 400;
  line-height: 1;
  cursor: pointer;
  transition:
    background 0.18s ease,
    color 0.18s ease,
    border-color 0.18s ease;
}
.nav-btn:hover { background: var(--ink); border-color: var(--ink); color: var(--surface); }

.ticks { display: flex; align-items: center; gap: 7px; }
.tick {
  width: 7px;
  height: 7px;
  border: none;
  padding: 0;
  border-radius: 3px;
  background: rgba(var(--ink-rgb), 0.18);
  cursor: pointer;
  transition: width 0.26s cubic-bezier(0.34, 1.4, 0.64, 1), background 0.18s ease;
}
.tick:hover { background: rgba(var(--ink-rgb), 0.45); }
.tick.on { width: 22px; background: var(--ink); }

/*
 * 进入按钮：单独一块，描边态起步。
 * 默认安静（透明底 + 描边），悬停才填充 —— 重点由用户的行为产生。
 */
.enter {
  display: inline-flex;
  align-items: center;
  gap: 30px;
  padding: 12px 18px;
  border: 1px solid rgba(var(--ink-rgb), 0.28);
  border-radius: 8px;
  background: transparent;
  color: var(--ink);
  font-family: inherit;
  font-size: 12px;
  font-weight: 600;
  letter-spacing: 0.14em;
  cursor: pointer;
  transition:
    background 0.18s ease,
    color 0.18s ease,
    border-color 0.18s ease;
}
.enter:hover { background: var(--ink); border-color: var(--ink); color: var(--surface); }
.enter i { font-size: 15px; transition: transform 0.2s ease; }
.enter:hover i { transform: translateX(4px); }

/* ── 尊重「减少动效」 ── */
@media (prefers-reduced-motion: reduce) {
  .nav-btn, .tick, .enter, .enter i { transition: none; }
}

/* ── 窄窗口 ── */
@media (max-width: 900px) {
  .brand, .subject { left: 20px; }
  /* .who 靠右，窄窗口下改右边距 */
  .who, .deco { right: 20px; }
  .actions { right: 20px; }
  .name { font-size: 32px; }
  .counter .cur { font-size: 30px; }
  .enter { gap: 18px; padding: 10px 14px; }
}
</style>
