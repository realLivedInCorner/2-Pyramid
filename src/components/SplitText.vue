<template>
  <!--
    逐字显性。
    每个字符一个 span，靠 --i 排开动画延迟 —— 「文字逐渐显出来」是逐字发生的，
    不是整块淡入。字符以 `${i}-${ch}` 作 key，所以 text 一变就重建元素、动画自动重播。
  -->
  <span class="split" :class="{ on: active }" :aria-label="text" role="text">
    <span
      v-for="(ch, i) in chars"
      :key="`${i}-${ch}`"
      class="split-ch"
      :style="{ '--i': i }"
      aria-hidden="true"
    >{{ ch === ' ' ? '\u00A0' : ch }}</span>
  </span>
</template>

<script setup lang="ts">
import { computed } from 'vue';

const props = withDefaults(
  defineProps<{
    /** 要逐字显示的文本 */
    text: string;
    /** 相邻字符的延迟（ms） */
    stagger?: number;
    /** 整段动画的起始延迟（ms） */
    delay?: number;
    /** 关掉时保持隐藏，等父级放行 */
    active?: boolean;
  }>(),
  { stagger: 34, delay: 0, active: true },
);

const chars = computed(() => Array.from(props.text));
</script>

<style scoped>
.split { display: inline; }

/*
 * 基线规则**不能**带 opacity: 0。
 *
 * 动画用的是 backwards（不是 both）—— backwards 只在延迟期间套用 from，
 * 结束后回到自然样式。基线若是 opacity: 0，动画一结束文字就整体消失。
 * 这正是 App.vue 页面级 stagger 注释里写明的那条：基线里不能再写隐形状态。
 *
 * 所以「隐形」完全交给动画的 from 关键帧；
 * 「未激活、根本不播动画」这种静态情况用下面单独的规则显式隐藏。
 */
.split-ch {
  display: inline-block;
  white-space: pre;
}

.split:not(.on) .split-ch { opacity: 0; }

.split.on .split-ch {
  animation: split-in 560ms var(--ease-out, cubic-bezier(0.22, 1, 0.36, 1)) backwards;
  animation-delay: calc(var(--i) * var(--split-stagger, 34ms) + var(--split-delay, 0ms));
}

@keyframes split-in {
  from { opacity: 0; translate: 0 0.42em; filter: blur(3px); }
  to   { opacity: 1; translate: 0 0;      filter: blur(0); }
}

@media (prefers-reduced-motion: reduce) {
  .split-ch { filter: none; }
  .split.on .split-ch { animation-duration: 1ms; animation-delay: 0ms; }
}
</style>
