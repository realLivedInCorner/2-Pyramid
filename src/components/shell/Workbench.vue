<script setup lang="ts">
/**
 * 流程工作台外壳：左侧常驻步骤轨 + 右侧当前步骤工作区。
 *
 * 为什么是「一次一步」而不是「一屏铺开」：
 * 原来转换页把导入模式、目标版本、执行进度、拖放区四张卡铺在一个 2×2 网格里，
 * 结果两张卡长期是空的（导入模式只有一个切换、进度空闲时只有三个灰点），
 * 而真正的主操作「开始转换」被放在最后一张卡的底部，离它依赖的版本设置隔了半个屏幕。
 *
 * 现在：每一步只呈现自己的控件；已完成的步骤收缩成一行摘要，可点回去改；
 * 未到达的步骤显示为锁定态，不占视觉重量。
 *
 * ── 切换动效 ──
 * ① 选中框是一个**独立的滑块**，靠 translateY 在选项之间弹过去，
 *    而不是每一项各自切背景色 —— 后者是「啪」地换，前者才有「移动」的实感。
 * ② 舞台内容随 stageKey 重挂载并播一次入场，槽内的分组再按序号错开。
 */
import { ref, watch, nextTick, onMounted, onBeforeUnmount } from 'vue';

export interface WorkbenchStep {
  id: string;
  /** 步骤名，如「源」「目标版本」 */
  label: string;
  /** 已完成步骤的摘要，如「12 项 · 48.2 MB」。未完成时留空 */
  summary?: string;
  state: 'done' | 'current' | 'todo' | 'locked';
  /**
   * 可选图标。给了就替代序号徽标 —— 用于「分类导航」而不是「流程步骤」的场合：
   * 设置页的分类没有先后顺序，标 1/2/3 会误导成有先后。
   */
  icon?: string;
}

const props = withDefaults(
  defineProps<{
    steps: WorkbenchStep[];
    /** 无障碍标签，如「转换流程」 */
    label: string;
    /**
     * 舞台内容的变化键。变了就把内容重挂载并播一次入场动画。
     * 不传就跟随选中项，所以调用方通常不用管。
     */
    stageKey?: string;
  }>(),
  { stageKey: '' },
);

const emit = defineEmits<{ go: [id: string] }>();

const railEl = ref<HTMLElement | null>(null);
/** 滑块的位置与高度（相对轨道）。null = 当前没有选中项 */
const slider = ref<{ y: number; h: number } | null>(null);

/** 把滑块量到当前选中项上。用 offsetTop/offsetHeight 而不是 getBoundingClientRect，
 *  这样拿到的是相对轨道的坐标，滚动时不会算错。 */
function measure() {
  const rail = railEl.value;
  const active = rail?.querySelector<HTMLElement>('.rail-step.current');
  slider.value = active ? { y: active.offsetTop, h: active.offsetHeight } : null;
}

/** 选中项一变，等 DOM 更新完再量 —— 否则量到的是上一帧的布局 */
watch(
  () => props.steps.map((s) => `${s.id}:${s.state}:${s.label}`).join('|'),
  () => { void nextTick(measure); },
);

let ro: ResizeObserver | null = null;
onMounted(() => {
  void nextTick(measure);
  window.addEventListener('resize', measure);
  // 字体加载完、面板出现滚动条等都会改变项高，用 ResizeObserver 兜住
  if (typeof ResizeObserver !== 'undefined' && railEl.value) {
    ro = new ResizeObserver(() => measure());
    ro.observe(railEl.value);
  }
});
onBeforeUnmount(() => {
  window.removeEventListener('resize', measure);
  ro?.disconnect();
  ro = null;
});

/** 舞台内容键：调用方没给就跟随选中项 */
const stageId = () => props.stageKey || props.steps.find((s) => s.state === 'current')?.id || '';
</script>

<template>
  <div class="workbench">
    <nav class="step-rail" ref="railEl" :aria-label="label">
      <!--
        滑块：一个独立元素在选项之间弹过去。
        它在按钮**下层**（z-index: 0 对按钮的 1），所以不会挡住点击。
      -->
      <span
        v-if="slider"
        class="rail-slider"
        :style="{ transform: `translateY(${slider.y}px)`, height: `${slider.h}px` }"
        aria-hidden="true"
      ></span>

      <button
        v-for="(step, i) in steps"
        :key="step.id"
        type="button"
        class="rail-step"
        :class="step.state"
        :disabled="step.state === 'locked'"
        :aria-current="step.state === 'current' ? 'step' : undefined"
        @click="emit('go', step.id)"
      >
        <span class="rail-index" aria-hidden="true">
          <i v-if="step.icon" :class="step.icon"></i>
          <!--
            有摘要 = 这一步已经走过。即使因为业务规则不再可点（locked），
            也该显示勾而不是序号 —— 否则「已完成但被锁」看起来像「还没到」。
          -->
          <i v-else-if="step.state === 'done' || step.summary" class="ri-check-line"></i>
          <template v-else>{{ i + 1 }}</template>
        </span>
        <span class="rail-body">
          <span class="rail-label">{{ step.label }}</span>
          <span v-if="step.summary" class="rail-summary">{{ step.summary }}</span>
        </span>
      </button>
    </nav>

    <!--
      舞台：:key 一变就整块重挂载，于是入场动画会重播。
      错开的分组动画由调用方在槽内容上写（它才知道自己有几个分组）。
    -->
    <section class="stage">
      <div class="stage-swap" :key="stageId()">
        <slot />
      </div>
    </section>
  </div>
</template>

<style scoped>
.workbench {
  flex: 1;
  min-height: 0;
  display: flex;
  gap: 20px;
  padding: 12px 40px 28px;
  z-index: 2;
}

/* ── 步骤轨 ── */
.step-rail {
  flex: 0 0 216px;
  position: relative;          /* 滑块的定位基准 */
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-height: 0;
  overflow-y: auto;
  /* 滑块要能溢出到圆角外时被裁掉，同时不让滚动条压住它 */
  padding: 2px;
  margin: -2px;
}

/*
 * 弹簧滑块。
 * 曲线用 --ease-spring（回弹），时长比内容动画略长一点 ——
 * 让「框先到、内容随后」的因果顺序读得出来。
 */
.rail-slider {
  position: absolute;
  left: 2px;
  right: 2px;
  top: 2px;
  z-index: 0;
  border-radius: 8px;
  /* 用墨色而不是主题色。首页通篇是墨色 + 暖白，没有一处蓝；
     主题色留给「主操作」按钮，选中态不属于主操作。 */
  background: rgba(var(--ink-rgb), 0.05);
  border: 1px solid rgba(var(--ink-rgb), 0.18);
  pointer-events: none;
  transition:
    transform 560ms var(--ease-spring, cubic-bezier(0.34, 1.56, 0.64, 1)),
    height 380ms var(--ease-out, cubic-bezier(0.22, 1, 0.36, 1)),
    opacity 200ms ease;
}

.rail-step {
  position: relative;
  z-index: 1;
  display: flex;
  /* 居中对齐：之前是 flex-start，导致文字块比徽标高 2px，看起来没对轴 */
  align-items: center;
  gap: 10px;
  width: 100%;
  padding: 10px 12px;
  border: 1px solid transparent;
  border-radius: 8px;
  background: transparent;
  cursor: pointer;
  text-align: left;
  font-family: inherit;
  transition: background-color 0.2s ease;
}

.rail-step:hover:not(:disabled):not(.current) {
  background: var(--hover-color);
}

.rail-step:disabled {
  cursor: default;
  opacity: 0.45;
}

/* 序号 / 完成勾 */
.rail-index {
  flex: 0 0 22px;
  height: 22px;
  border-radius: 6px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  font-size: 11px;
  font-weight: 600;
  /* 默认是描边而不是实心色块 —— 与首页「默认安静、选中才亮」一致 */
  background: transparent;
  border: 1px solid var(--border-color);
  color: var(--text-secondary);
  transition:
    background-color 0.24s ease,
    color 0.24s ease,
    border-color 0.24s ease,
    transform 0.34s var(--ease-spring, cubic-bezier(0.34, 1.56, 0.64, 1));
}

.rail-index i {
  font-size: 13px;
  line-height: 1;
}

.rail-step.done .rail-index {
  background: var(--ink);
  border-color: var(--ink);
  color: var(--surface);
}

.rail-step.current .rail-index {
  background: var(--ink);
  border-color: var(--ink);
  color: var(--surface);
  /* 不再 scale —— 放大会把 22px 的盒子撑成 23.8px，
     和旁边的文字块不在同一条中线上，看起来就是「没对轴」 */
}

.rail-body {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.rail-label {
  font-size: 13px;
  font-weight: 500;
  color: var(--text-secondary);
  line-height: 1.3;
  transition: color 0.24s ease, font-weight 0.24s ease;
}

/* 首页的层级靠字号与颜色，不靠加粗 —— 只有选中项用正文色 */
.rail-step.current .rail-label {
  color: var(--text-color);
  font-weight: 600;
}

.rail-summary {
  font-size: 11.5px;
  color: var(--text-secondary);
  line-height: 1.35;
  overflow: hidden;
  text-overflow: ellipsis;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  line-clamp: 2;
  -webkit-box-orient: vertical;
}

/* ── 舞台 ── */
.stage {
  flex: 1;
  min-width: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.stage-swap {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  /*
   * fill-mode 必须是 backwards，不能是 both / forwards。
   *
   * both 会让动画结束后**永久保留** to 关键帧里的 `translate: 0 0`，
   * 而 translate 只要不是 none 就会成为包含块 —— 舞台里的对话框
   * （position: fixed）于是不再相对视口定位，灰底只盖住舞台区、盖不住整屏。
   * 这正是 App.vue 页面级 stagger 踩过的同一个坑。
   *
   * backwards 只在延迟期间套用 from，结束后回到自然样式，视觉上完全一样。
   */
  animation: stage-in 460ms var(--ease-out, cubic-bezier(0.22, 1, 0.36, 1)) backwards;
}

@keyframes stage-in {
  from { opacity: 0; translate: 0 10px; }
  to   { opacity: 1; translate: 0 0; }
}

@media (prefers-reduced-motion: reduce) {
  .rail-slider,
  .rail-step,
  .rail-index,
  .rail-label { transition: none; }
  .stage-swap { animation-duration: 1ms; }
}

@media (max-width: 900px) {
  .workbench {
    flex-direction: column;
    gap: 12px;
    padding: 8px 20px 20px;
  }
  .step-rail {
    flex: 0 0 auto;
    flex-direction: row;
    overflow-x: auto;
    gap: 6px;
  }
  /* 窄屏时轨道横向排列，滑块改为按 X 移动 */
  .rail-slider {
    left: 2px; right: auto; top: 2px; bottom: 2px;
    width: 0;
    transition:
      transform 560ms var(--ease-spring, cubic-bezier(0.34, 1.56, 0.64, 1)),
      width 380ms var(--ease-out, cubic-bezier(0.22, 1, 0.36, 1));
  }
  .rail-step {
    width: auto;
    flex: 0 0 auto;
    align-items: center;
  }
  .rail-summary {
    display: none;
  }
}
</style>
