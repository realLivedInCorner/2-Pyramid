<template>
  <div class="hsv-picker" :style="{ '--hue': hue, '--picker-color': hex }">
    <div class="sv-panel" ref="svRef" @mousedown="startPick">
      <div class="sv-white"></div>
      <div class="sv-black"></div>
      <div class="sv-cursor" :style="{ left: sat + '%', top: (100 - val) + '%' }"></div>
    </div>
    <div class="picker-side">
      <input
        class="hue-slider"
        type="range"
        min="0"
        max="360"
        :value="hue"
        :style="{ accentColor: hex }"
        @input="onHueChange"
      />
      <div class="color-value">{{ hex }}</div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, onBeforeUnmount } from 'vue';

const props = defineProps<{
  modelValue: { r: number; g: number; b: number; a?: number };
}>();

const emit = defineEmits<{
  'update:modelValue': [value: { r: number; g: number; b: number; a?: number }];
}>();

const hue = ref(0);
const sat = ref(0);
const val = ref(0);
const hex = ref('#ffffff');
const svRef = ref<HTMLElement | null>(null);
const isPicking = ref(false);
let syncing = false;

function hsvToHex(h: number, s: number, v: number) {
  const satN = s / 100;
  const valN = v / 100;
  const c = valN * satN;
  const x = c * (1 - Math.abs(((h / 60) % 2) - 1));
  const m = valN - c;
  let r = 0, g = 0, b = 0;
  if (h < 60) { r = c; g = x; b = 0; }
  else if (h < 120) { r = x; g = c; b = 0; }
  else if (h < 180) { r = 0; g = c; b = x; }
  else if (h < 240) { r = 0; g = x; b = c; }
  else if (h < 300) { r = x; g = 0; b = c; }
  else { r = c; g = 0; b = x; }
  const toHex = (n: number) => {
    const num = Math.round((n + m) * 255);
    return num.toString(16).padStart(2, '0');
  };
  return `#${toHex(r)}${toHex(g)}${toHex(b)}`;
}

function hexToHsv(hex: string) {
  const normalized = hex.replace('#', '');
  const r = parseInt(normalized.substring(0, 2), 16) / 255;
  const g = parseInt(normalized.substring(2, 4), 16) / 255;
  const b = parseInt(normalized.substring(4, 6), 16) / 255;
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const d = max - min;
  let h = 0;
  if (d !== 0) {
    if (max === r) h = ((g - b) / d) % 6;
    else if (max === g) h = (b - r) / d + 2;
    else h = (r - g) / d + 4;
    h = Math.round(h * 60);
    if (h < 0) h += 360;
  }
  const s = max === 0 ? 0 : Math.round((d / max) * 100);
  const v = Math.round(max * 100);
  return { h, s, v };
}

function rgbaToHex(c: { r: number; g: number; b: number }) {
  const to = (v: number) => Math.round(Math.min(1, Math.max(0, v)) * 255).toString(16).padStart(2, '0');
  return `#${to(c.r)}${to(c.g)}${to(c.b)}`;
}

function syncFromProps() {
  syncing = true;
  hex.value = rgbaToHex(props.modelValue);
  const hsv = hexToHsv(hex.value);
  hue.value = hsv.h;
  sat.value = hsv.s;
  val.value = hsv.v;
  syncing = false;
}

function emitCurrent() {
  if (syncing) return;
  hex.value = hsvToHex(hue.value, sat.value, val.value);
  const n = hex.value.replace('#', '');
  const a = props.modelValue.a ?? 1;
  emit('update:modelValue', {
    r: parseInt(n.slice(0, 2), 16) / 255,
    g: parseInt(n.slice(2, 4), 16) / 255,
    b: parseInt(n.slice(4, 6), 16) / 255,
    a
  });
}

watch(() => props.modelValue, () => {
  // 外部改动（加载配置）时同步；拖动时 props 由 emit 回写，跳过避免抢焦点
  const incoming = rgbaToHex(props.modelValue);
  if (incoming.toLowerCase() === hex.value.toLowerCase()) return;
  syncFromProps();
}, { deep: true });

const onHueChange = (e: Event) => {
  hue.value = Number((e.target as HTMLInputElement).value);
  emitCurrent();
};

const handlePick = (event: MouseEvent) => {
  if (!svRef.value) return;
  const rect = svRef.value.getBoundingClientRect();
  const x = Math.min(Math.max(event.clientX - rect.left, 0), rect.width);
  const y = Math.min(Math.max(event.clientY - rect.top, 0), rect.height);
  sat.value = Math.round((x / rect.width) * 100);
  val.value = Math.round(100 - (y / rect.height) * 100);
  emitCurrent();
};

const endPick = () => {
  isPicking.value = false;
  window.removeEventListener('mousemove', handlePick);
  window.removeEventListener('mouseup', endPick);
};

const startPick = (event: MouseEvent) => {
  isPicking.value = true;
  handlePick(event);
  window.addEventListener('mousemove', handlePick);
  window.addEventListener('mouseup', endPick);
};

onBeforeUnmount(endPick);
syncFromProps();
</script>

<style scoped>
.hsv-picker {
  display: flex;
  align-items: center;
  gap: 14px;
  width: 100%;
}

.sv-panel {
  position: relative;
  flex: 1;
  min-width: 0;
  height: 120px;
  border-radius: 12px;
  background: linear-gradient(90deg, #fff, hsl(var(--hue), 100%, 50%));
  overflow: hidden;
  border: 1px solid #111;
  box-shadow:
    0 0 0 1px rgba(0, 0, 0, 0.35),
    0 8px 20px rgba(0, 0, 0, 0.12),
    0 0 18px color-mix(in srgb, var(--picker-color) 28%, transparent);
  cursor: crosshair;
}

.sv-white {
  position: absolute;
  inset: 0;
  background: linear-gradient(90deg, #fff, rgba(255, 255, 255, 0));
}

.sv-black {
  position: absolute;
  inset: 0;
  background: linear-gradient(0deg, #000, rgba(0, 0, 0, 0));
}

.sv-cursor {
  position: absolute;
  width: 12px;
  height: 12px;
  border: 2px solid #fff;
  border-radius: 50%;
  box-shadow: 0 0 0 2px rgba(0, 0, 0, 0.4), 0 0 12px color-mix(in srgb, var(--picker-color) 60%, transparent);
  transform: translate(-6px, -6px);
  pointer-events: none;
}

.picker-side {
  display: flex;
  flex-direction: column;
  gap: 10px;
  align-items: flex-start;
  flex-shrink: 0;
}

.hue-slider {
  width: 120px;
  background: linear-gradient(90deg, #ff2b2b, #ffd12b, #2bff6a, #2be6ff, #2b5bff, #b42bff, #ff2b9a);
  border-radius: 999px;
  height: 8px;
  appearance: none;
  box-shadow: inset 0 0 0 1px #111, 0 0 10px color-mix(in srgb, var(--picker-color) 35%, transparent);
}

.hue-slider::-webkit-slider-thumb {
  appearance: none;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: var(--picker-color);
  border: 2px solid #111;
  box-shadow: 0 0 10px color-mix(in srgb, var(--picker-color) 70%, transparent), 0 2px 8px rgba(0, 0, 0, 0.25);
  cursor: pointer;
  margin-top: -4px;
}

.hue-slider::-moz-range-thumb {
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: var(--picker-color);
  border: 2px solid #111;
  box-shadow: 0 0 10px color-mix(in srgb, var(--picker-color) 70%, transparent), 0 2px 8px rgba(0, 0, 0, 0.25);
  cursor: pointer;
}

.hue-slider::-webkit-slider-runnable-track {
  height: 8px;
  border-radius: 999px;
  border: 1px solid #111;
}

.hue-slider::-moz-range-track {
  height: 8px;
  border-radius: 999px;
  border: 1px solid #111;
  background: linear-gradient(90deg, #ff2b2b, #ffd12b, #2bff6a, #2be6ff, #2b5bff, #b42bff, #ff2b9a);
}

.color-value {
  font-size: 12px;
  color: #64748b;
  font-variant-numeric: tabular-nums;
}
</style>
