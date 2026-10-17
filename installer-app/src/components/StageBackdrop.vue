<script setup lang="ts">
/**
 * 舞台背景 —— 与主程序 `src/components/shell/StageBackdrop.vue` 的**同一块地面**。
 *
 * 为什么要在安装器里**重写一份**、而不是跨项目 import 主程序那个组件：
 * 安装器是独立构建（自己的 vite.config.ts / index.html / dist-installer，
 * 见 src-tauri/tauri.conf.json 的 frontendDist），它的源码将来要能单独发布、
 * 单独回滚。跨构建 import 会把两个应用绑死：改主程序的背景会连带改安装器，
 * 而安装器可能已经发出去、正在被谁的升级流程下载。
 * 代价是两份几何参数必须各自维护 —— 所以这里的常量都写明「怎么算出来的」，
 * 下次谁要调，按注释里的比例缩放，而不是凭感觉改数字。
 *
 * 构成与主程序完全一致，三层：
 *   · base   —— 大范围柔光渐变
 *   · stairs —— 阶梯台地。每升一级踏面更窄、级高更矮，由近及远收进。
 *               每级靠一条极淡的边线分隔 ——「一层一层」来自边线，不来自明暗差。
 *   · detail —— 斜向细线、暗角、纸纹。各自带自己的不透明度，只能整体淡入。
 *
 * ── 为什么要缩放下面的四个常量 ─────────────────────────────────────────
 *
 * 主程序窗口 1200×750（src-tauri/src/lib.rs:241 inner_size(1200, 750)），
 * 安装器窗口 880×640（本文件的兄弟改动，见 src-tauri/tauri.conf.json）。
 * 两边的台阶如果都用 96px 踏面宽，安装器里的台阶会比主程序**粗 36%**
 * —— 同样一个 880 宽的窗口，只放得下主程序台阶的 2/3 个周期，读起来是
 * 「几块大板子」而不是「远处的台地」。
 *
 * 所以按窗口宽度等比缩放：96 × (880/1200) ≈ 70、58 × 0.733 ≈ 42。
 * 校验（SHRINK=0.935、12 级时一个周期的累计比 ∫ = (1-0.935^12)/(1-0.935) = 8.5155）：
 *   主程序：踏面 96 × 8.5155 = 817px = 窗口宽的 68%；级高 58 × 8.5155 = 494px
 *   安装器：踏面 70 × 8.5155 = 596px = 窗口宽的 68%；级高 42 × 8.5155 = 358px
 * 68% == 68%，两边的「一个周期占多宽」因此完全相同。
 */
const TIERS = 12;      // 一个周期内的级数
const TIER_W = 70;     // 最低一级的踏面宽（主程序 96，按 880/1200 缩放）
const TIER_H = 42;     // 最低一级的级高（主程序 58，同比缩放）
const SHRINK = 0.935;  // 每升一级缩小的比例（与主程序一致，不改）

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
 *
 * COPIES 取 2：一个周期 358px 高，3 个周期 = 1074px，覆盖 640 高的窗口
 * 还多出 434px 余量 —— 窗口被拖高到 minHeight 以上任意高度都不会露出空档。
 * 主程序用的是 4，因为它那边一个周期 494px 而且窗口能拉得更高；
 * 这里再用 4 只是多出 24 个不可见的 DOM 节点。
 */
const COPIES = 2;
const tiers = Array.from({ length: TIERS * (COPIES + 1) }, (_, i) => {
  const s = shape.out[i % TIERS];
  const cycle = Math.floor(i / TIERS) - 1;
  return { left: s.left, bottom: s.bottom + cycle * PERIOD, h: s.h };
});
</script>

<template>
  <div class="stage-bg" aria-hidden="true">
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
  /* 与 index.html 的 <meta name="color-scheme">、src/styles/tokens.css 的 --bg、
     App.vue 里 body 的兜底底色**必须同色**。四处任何一处对不上，
     样式表加载或首帧的那一瞬间就会闪一下别的底色（深色系统上最明显：
     先白一帧再变深）。 */
  background: var(--bg);
}

.bg-base {
  position: absolute;
  inset: 0;
  background:
    radial-gradient(1250px 820px at 20% 8%, var(--surface) 0%, rgba(255, 255, 255, 0) 62%),
    linear-gradient(158deg, var(--surface-2) 0%, var(--surface-4) 100%);
}

/*
 * 阶梯：静态的台地。
 *
 * 第 i 级是一块矩形，左边缘从累积的踏面宽起、底边在累积的级高上，
 * 一路延伸到右缘 —— 这些矩形叠起来就是阶梯轮廓，并以 PERIOD 为周期纵向重复。
 *
 * 对比度与清晰度分开处理：
 *   · 边线（踏面顶边 / 立面左边）：清晰，但只有 14% / 9% 不透明度 → 看得见「层」
 *   · 块面填充：几乎无差 → 不形成抢眼的明暗块
 * 主程序早先用 blur 压对比度，把台阶边一起糊没了，既不像阶梯又留下一团光。
 */
.stairs {
  position: absolute;
  inset: 0;
  overflow: hidden;
}

.stairs i {
  position: absolute;
  left: var(--l);
  bottom: var(--b);
  right: 0;
  /* 高度逐级递减 —— 这是「越往上越小」的来源，不靠整面透视 */
  height: var(--h);
  /* 白釉的浓淡走 --stair-a/--stair-b，深浅两套差 9 倍，理由见 tokens.css */
  background: linear-gradient(
    180deg,
    rgba(255, 255, 255, var(--stair-a)) 0%,
    rgba(255, 255, 255, var(--stair-b)) 100%
  );
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
 * 淡入只能整体套一层容器 —— 直接给它们写 opacity 会把各自的基准值覆盖掉，
 * 纸纹瞬间变成满屏噪点。
 */
.bg-detail {
  position: absolute;
  inset: 0;
  pointer-events: none;
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
  /* 收边。两种主题的方向相反：浅色下边缘压深（纸的暗角），
     深色下边缘是要更暗才收得住，所以 --vignette-rgb 在两套里取值不同。 */
  background: radial-gradient(
    128% 104% at 46% 44%,
    rgba(var(--vignette-rgb), 0) 46%,
    rgba(var(--vignette-rgb), 0.5) 100%
  );
}

/* 纸纹：细颗粒噪点。纯 CSS 渐变做不出这个质感，用 SVG 湍流。
   深色下 multiply 会把噪点压没，改用 screen 才看得见颗粒。 */
.bg-grain {
  position: absolute;
  inset: 0;
  opacity: 0.055;
  mix-blend-mode: multiply;
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='220' height='220'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.85' numOctaves='3' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)'/%3E%3C/svg%3E");
}
[data-theme='dark'] .bg-grain { mix-blend-mode: screen; opacity: 0.03; }
</style>
