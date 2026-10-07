<script setup lang="ts">
/**
 * 舞台背景 —— 全应用共用的一块「地面」。
 *
 * 从首页抽出来的。之前只有首页有这套背景，设置页、转换页各自踩在蓝色渐变上，
 * 于是切页面时底色整块换掉，读起来像两个不同的应用。
 * 抽成组件之后所有主页面共用同一块地面，页面切换时背景不动，只有内容在换。
 *
 * 构成分三层：
 *   · base   —— 大范围柔光渐变
 *   · stairs —— 阶梯台地。每升一级踏面更窄、级高更矮，由近及远收进。
 *               每级靠一条极淡的边线分隔 ——「一层一层」来自边线，不来自明暗差，
 *               所以看得清层级却不抢焦点。
 *   · detail —— 细线、暗角、纸纹。各自带自己的不透明度，只能整体淡入。
 */
withDefaults(
  defineProps<{
    /** 加载页过渡时用：false 时整层不可见，true 时淡入 */
    revealed?: boolean;
  }>(),
  { revealed: true },
);

const TIERS = 12;          // 一个周期内的级数
const TIER_W = 96;         // 最低一级的踏面宽
const TIER_H = 58;         // 最低一级的级高
const SHRINK = 0.935;      // 每升一级缩小的比例

/**
 * 一个周期内每一级的几何：left（左缘）、bottom（底边）、h（自身高度）。
 * 位置是累积的 —— 上一级的左缘加上它的踏面宽，就是下一级的左缘。
 */
const shape = (() => {
  const out: { left: number; bottom: number; h: number }[] = [];
  let left = 0, bottom = 0, w = TIER_W, h = TIER_H;
  for (let j = 0; j < TIERS; j++) {
    out.push({ left: Math.round(left), bottom: Math.round(bottom), h: Math.round(h) });
    left += w;
    bottom += h;
    w *= SHRINK;
    h *= SHRINK;
  }
  return { out, height: Math.round(bottom) };
})();
const PERIOD = shape.height;

/**
 * 铺几个周期，并向下多铺一轮。
 * 向下多一轮是为了适配更高的窗口，省得以后有人改视口高度时底部露出空档。
 */
const COPIES = 4;
const tiers = Array.from({ length: TIERS * (COPIES + 1) }, (_, i) => {
  const s = shape.out[i % TIERS];
  const cycle = Math.floor(i / TIERS) - 1;
  return { left: s.left, bottom: s.bottom + cycle * PERIOD, h: s.h };
});
</script>

<template>
  <div class="stage-bg" :class="{ on: revealed }" aria-hidden="true">
    <div class="bg-base"></div>

    <div class="stairs">
      <i
        v-for="(t, k) in tiers"
        :key="k"
        :style="{ '--l': `${t.left}px`, '--b': `${t.bottom}px`, '--h': `${t.h}px` }"
      ></i>
    </div>

    <div class="bg-detail">
      <div class="bg-grid"></div>
      <div class="bg-vignette"></div>
      <div class="bg-grain"></div>
    </div>
  </div>
</template>

<style scoped>
.stage-bg {
  position: absolute;
  inset: 0;
  pointer-events: none;
  overflow: hidden;
  /* 与 index.html 加载页、App.vue 的 html/body 同色 —— 三处必须一致，
     否则过渡或页面切换的瞬间会闪一下别的底色 */
  background: var(--bg);
}

.bg-base {
  position: absolute;
  inset: 0;
  background:
    radial-gradient(1250px 820px at 20% 8%, var(--surface) 0%, rgba(255, 255, 255, 0) 62%),
    linear-gradient(158deg, #f8f6f2 0%, #efebe4 100%);
}

/*
 * 阶梯：静态的台地。
 *
 * 第 i 级是一块矩形，左边缘从累积的踏面宽起、底边在累积的级高上，
 * 一路延伸到右缘 —— 这些矩形叠起来就是阶梯轮廓，并以 PERIOD 为周期纵向重复。
 *
 * 对比度与清晰度分开处理：
 *   · 边线（踏面顶边 / 立面左边）：清晰，但只有 9–13% 不透明度 → 看得见「层」
 *   · 块面填充：几乎无差 → 不形成抢眼的明暗块
 * 早先用 blur 压对比度，把台阶边一起糊没了，既不像阶梯又留下一团光。
 */
.stairs {
  position: absolute;
  inset: 0;
  overflow: hidden;
  transition: opacity 680ms var(--ease-out, cubic-bezier(0.22, 1, 0.36, 1));
}

.stairs i {
  position: absolute;
  left: var(--l);
  bottom: var(--b);
  right: 0;
  /* 高度逐级递减 —— 这是「越往上越小」的来源，不靠整面透视 */
  height: var(--h);
  background: linear-gradient(180deg, rgba(255, 255, 255, 0.5) 0%, rgba(255, 255, 255, 0.26) 100%);
  border-top: 1px solid rgba(var(--warm-rgb), 0.14);
  border-left: 1px solid rgba(var(--warm-rgb), 0.09);
}

/* 越往上越淡 —— 在尺寸递减之外再叠一层明度衰减，纵深更明确 */
.stairs i:nth-child(n + 7) { opacity: 0.82; }
.stairs i:nth-child(n + 13) { opacity: 0.66; }
.stairs i:nth-child(n + 19) { opacity: 0.5; }
.stairs i:nth-child(n + 25) { opacity: 0.36; }
.stairs i:nth-child(n + 31) { opacity: 0.24; }

/*
 * 这三个是「细部」层，各自带自己的不透明度（纸纹只有 0.055）。
 * 淡入只能整体套一层容器 —— 直接给它们写 opacity: 1
 * 会把各自的基准值覆盖掉，纸纹瞬间变成满屏噪点。
 */
.bg-detail {
  position: absolute;
  inset: 0;
  pointer-events: none;
  transition: opacity 900ms var(--ease-out, cubic-bezier(0.22, 1, 0.36, 1));
}

/* 极淡的斜向细线：给纸面一点纹理方向，远看几乎不可见 */
.bg-grid {
  position: absolute;
  inset: 0;
  background-image: repeating-linear-gradient(
    118deg,
    rgba(var(--warm-rgb), 0.06) 0 1px,
    transparent 1px 13px
  );
  mask-image: linear-gradient(200deg, #000 0%, transparent 62%);
  -webkit-mask-image: linear-gradient(200deg, #000 0%, transparent 62%);
}

.bg-vignette {
  position: absolute;
  inset: 0;
  background: radial-gradient(128% 104% at 46% 44%, rgba(244, 242, 238, 0) 46%, rgba(214, 208, 198, 0.5) 100%);
}

/* 纸纹：细颗粒噪点。纯 CSS 渐变做不出这个质感，用 SVG 湍流 */
.bg-grain {
  position: absolute;
  inset: 0;
  opacity: 0.055;
  mix-blend-mode: multiply;
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='220' height='220'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.85' numOctaves='3' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)'/%3E%3C/svg%3E");
}

/* 未显现：整层退到不可见，由父级在合适时机切成 .on */
.stage-bg:not(.on) .stairs,
.stage-bg:not(.on) .bg-detail { opacity: 0; }

@media (prefers-reduced-motion: reduce) {
  .stairs,
  .bg-detail { transition: none; }
}
</style>
