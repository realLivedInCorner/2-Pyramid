<template>
  <transition name="sidebar-overlay-fade">
    <div v-if="visible" class="sidebar-overlay" @click="visible = false"></div>
  </transition>

  <transition
    :css="false"
    @before-enter="onBeforeEnter"
    @enter="onEnter"
    @after-enter="onAfterEnter"
    @before-leave="onBeforeLeave"
    @leave="onLeave"
    @after-leave="onAfterLeave"
  >
    <aside v-if="visible" class="sidebar-content visual-panel" @click.stop tabindex="-1">
      <div class="panel-header">
        <h2 class="panel-title">{{ t('dialog.visual.title') }}</h2>
      </div>

      <div class="panel-body">
        <!-- 渲染选项：这两个开关改的都是「怎么渲染」，归到一个小标题下。
             复用已有键，不新增 i18n -->
        <section class="option-group">
          <div class="group-header">
            <div class="group-label">{{ t('dialog.otherOptions.rendering') }}</div>
          </div>

          <div class="switch-list">
            <!-- 背包无阴影 -->
            <div class="option-item">
              <div class="option-info">
                <div class="option-label">{{ t('dialog.visual.noShadow') }}</div>
                <div class="option-desc">{{ t('dialog.visual.noShadowDesc') }}</div>
              </div>
              <label class="switch">
                <input type="checkbox" v-model="settings.no_shadow">
                <span class="slider round"></span>
              </label>
            </div>

            <!-- 附魔闪光 -->
            <div class="option-item">
              <div class="option-info">
                <div class="option-label">{{ t('dialog.visual.enchantGlint') }}</div>
                <div class="option-desc">{{ t('dialog.visual.enchantGlintDesc') }}</div>
              </div>
              <label class="switch">
                <input type="checkbox" v-model="settings.custom_glint">
                <span class="slider round"></span>
              </label>
            </div>
          </div>
        </section>

        <!-- 实体描边 -->
        <section class="option-group">
          <div class="group-header">
            <div class="group-label">{{ t('dialog.visual.outline') }}</div>
            <div class="group-desc">{{ t('dialog.visual.outlineDesc') }}</div>
          </div>

          <div class="outline-grid">
            <button
              v-for="type in outlineTypes"
              :key="type.id"
              class="outline-card"
              :class="{ active: settings.outline_type === type.id }"
              @click="settings.outline_type = type.id"
            >
              <div class="outline-preview" :style="{ background: type.color }"></div>
              <div class="outline-name">{{ type.name }}</div>
            </button>
          </div>

          <!-- 标准描边：对齐 overlay.py 的 core_outline.color / thickness -->
          <div v-if="settings.outline_type === 'default'" class="outline-detail">
            <div class="detail-row">
              <label class="detail-label">{{ t('dialog.visual.outlineColor') }}</label>
              <div class="color-row">
                <input
                  type="color"
                  class="color-input"
                  :value="hexFromRgba(settings.core_outline.color)"
                  @input="onColorInput($event)"
                />
                <span class="color-hex">{{ hexFromRgba(settings.core_outline.color) }}</span>
              </div>
            </div>
            <div class="detail-row">
              <label class="detail-label">{{ t('dialog.visual.outlineAlpha') }}</label>
              <input
                type="range"
                min="0"
                max="1"
                step="0.05"
                v-model.number="settings.core_outline.color.a"
                class="range-input"
              />
              <span class="range-val">{{ settings.core_outline.color.a.toFixed(2) }}</span>
            </div>
            <div class="detail-row">
              <label class="detail-label">{{ t('dialog.visual.outlineThickness') }}</label>
              <input
                type="range"
                min="0.5"
                max="6"
                step="0.5"
                v-model.number="settings.core_outline.thickness"
                class="range-input"
              />
              <span class="range-val">{{ settings.core_outline.thickness }}x</span>
            </div>
          </div>

          <!-- 双色循环（N 卡）：HSV 取色 + 实时预览 -->
          <div v-if="settings.outline_type === 'gradient'" class="outline-detail">
            <div class="gradient-preview" :style="gradientPreviewStyle" aria-hidden="true">
              <span class="preview-label">{{ t('dialog.visual.gradientPreview') }}</span>
            </div>

            <div class="detail-block">
              <label class="detail-label">{{ t('dialog.visual.gradientColorA') }}</label>
              <HsvColorPicker v-model="settings.core_gradient_outline.color_a" />
            </div>
            <div class="detail-block">
              <label class="detail-label">{{ t('dialog.visual.gradientColorB') }}</label>
              <HsvColorPicker v-model="settings.core_gradient_outline.color_b" />
            </div>
            <p class="detail-hint">{{ t('dialog.visual.gradientHint') }}</p>
          </div>
        </section>
      </div>

      <div class="panel-footer">
        <div class="save-status" v-if="saveStatus" :class="saveStatus.type">
          <i :class="saveStatus.type === 'success' ? 'ri-checkbox-circle-line' : 'ri-error-warning-line'"></i>
          {{ saveStatus.text }}
        </div>
        <div class="footer-btns">
          <button class="primary-btn" :disabled="isSaving" @click="handleSave">
            <i class="ri-save-line" v-if="!isSaving"></i>
            <i class="ri-loader-4-line spin" v-else></i>
            {{ isSaving ? t('common.saving') : t('common.save') }}
          </button>
          <!-- 不挂 .back-btn：全局 .app-container .back-btn 是给页面级返回钮的玻璃皮肤，
               在抽屉页脚里会和共享的 .ghost-btn 打架（同特异度，看加载顺序） -->
          <button class="ghost-btn" @click="visible = false">
            <i class="ri-arrow-go-back-line"></i>
            <span>{{ t('common.back') }}</span>
          </button>
        </div>
      </div>
    </aside>
  </transition>
</template>

<script setup lang="ts">
import { ref, onMounted, reactive, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { invoke } from '@tauri-apps/api/core';
import HsvColorPicker from './HsvColorPicker.vue';
import { useSidebarSlide } from '../composables/useSidebarSlide';

const { t } = useI18n();

const visible = defineModel<boolean>({ required: true });

const props = defineProps<{
  projectName: string
}>();

// 投影必须是暖色：合成器的默认值是 rgba(0,0,0,·) 冷灰，压在暖白面板上会发脏
const {
  onBeforeEnter,
  onEnter,
  onAfterEnter,
  onBeforeLeave,
  onLeave,
  onAfterLeave,
} = useSidebarSlide({ shadow: '-12px 0 36px rgba(28, 24, 18, 0.12)' });

const isSaving = ref(false);
const saveStatus = ref<{ text: string, type: 'success' | 'error' } | null>(null);

const settings = reactive({
  no_shadow: false,
  custom_glint: false,
  outline_type: 'none' as 'none' | 'default' | 'rainbow' | 'rainbow_hexian' | 'gradient',
  core_outline: {
    color: { r: 1, g: 1, b: 1, a: 1 },
    thickness: 2
  },
  // 双色循环（N 卡）：默认浅紫 ↔ 黑
  core_gradient_outline: {
    color_a: { r: 0.7, g: 0.4, b: 0.9, a: 1 },
    color_b: { r: 0, g: 0, b: 0, a: 1 }
  }
});

const outlineTypes = [
  // 「无」的色板用暖白：原 #f1f5f9 是冷板岩灰，和这一版暖色发丝线不是一支
  { id: 'none' as const, name: t('dialog.visual.outlineTypes.none'), color: '#f2efe9' },
  // core_rainbow_outline：N 卡彩虹
  { id: 'rainbow' as const, name: t('dialog.visual.outlineTypes.rainbow'), color: 'linear-gradient(45deg, #ff0000, #00ff00, #0000ff)' },
  // core_rainbow_outline_hexian：其他显卡彩虹
  { id: 'rainbow_hexian' as const, name: t('dialog.visual.outlineTypes.rainbowHexian'), color: 'linear-gradient(135deg, #ff8a00, #e52e71, #9b59b6)' },
  // core_gradient_outline：N 卡双色循环
  { id: 'gradient' as const, name: t('dialog.visual.outlineTypes.gradient'), color: 'linear-gradient(90deg, #b366e6, #000000)' },
  // core_outline：可改颜色 / 粗细
  { id: 'default' as const, name: t('dialog.visual.outlineTypes.standard'), color: '#fff' }
];

function hexFromRgba(c: { r: number; g: number; b: number }) {
  const to = (v: number) => Math.round(Math.min(1, Math.max(0, v)) * 255).toString(16).padStart(2, '0');
  return `#${to(c.r)}${to(c.g)}${to(c.b)}`;
}

function rgbaCss(c: { r: number; g: number; b: number; a: number }) {
  const r = Math.round(Math.min(1, Math.max(0, c.r)) * 255);
  const g = Math.round(Math.min(1, Math.max(0, c.g)) * 255);
  const b = Math.round(Math.min(1, Math.max(0, c.b)) * 255);
  return `rgba(${r}, ${g}, ${b}, ${c.a})`;
}

/** 贴近着色器：sin(GameTime·5000 + pos) 在 A/B 间往返 */
const gradientPreviewStyle = computed(() => {
  const a = rgbaCss(settings.core_gradient_outline.color_a);
  const b = rgbaCss(settings.core_gradient_outline.color_b);
  return {
    '--grad-a': a,
    '--grad-b': b,
    background: `linear-gradient(90deg, ${a}, ${b}, ${a})`,
    backgroundSize: '200% 100%',
    animation: 'grad-cycle 2.4s linear infinite'
  };
});

function onColorInput(e: Event) {
  const hex = (e.target as HTMLInputElement).value.replace('#', '');
  if (hex.length !== 6) return;
  settings.core_outline.color.r = parseInt(hex.slice(0, 2), 16) / 255;
  settings.core_outline.color.g = parseInt(hex.slice(2, 4), 16) / 255;
  settings.core_outline.color.b = parseInt(hex.slice(4, 6), 16) / 255;
}

const loadSettings = async () => {
  try {
    const data = await invoke<any>('get_overlay_json', { projectName: props.projectName });
    settings.no_shadow = !!data.no_shadow;
    settings.custom_glint = !!data.custom_glint;
    const ot = data.outline_type || 'none';
    settings.outline_type = (
      ot === 'none' || ot === 'default' || ot === 'rainbow' || ot === 'rainbow_hexian' || ot === 'gradient'
    ) ? ot : 'none';
    // 兼容 Python core_outline.color / thickness
    const co = data.core_outline;
    if (co && typeof co === 'object') {
      if (co.color && typeof co.color === 'object') {
        settings.core_outline.color.r = Number(co.color.r ?? 1);
        settings.core_outline.color.g = Number(co.color.g ?? 1);
        settings.core_outline.color.b = Number(co.color.b ?? 1);
        settings.core_outline.color.a = Number(co.color.a ?? 1);
      }
      if (typeof co.thickness === 'number') {
        settings.core_outline.thickness = co.thickness;
      }
    }
    const go = data.core_gradient_outline;
    if (go && typeof go === 'object') {
      if (go.color_a && typeof go.color_a === 'object') {
        settings.core_gradient_outline.color_a.r = Number(go.color_a.r ?? 0.7);
        settings.core_gradient_outline.color_a.g = Number(go.color_a.g ?? 0.4);
        settings.core_gradient_outline.color_a.b = Number(go.color_a.b ?? 0.9);
        settings.core_gradient_outline.color_a.a = Number(go.color_a.a ?? 1);
      }
      if (go.color_b && typeof go.color_b === 'object') {
        settings.core_gradient_outline.color_b.r = Number(go.color_b.r ?? 0);
        settings.core_gradient_outline.color_b.g = Number(go.color_b.g ?? 0);
        settings.core_gradient_outline.color_b.b = Number(go.color_b.b ?? 0);
        settings.core_gradient_outline.color_b.a = Number(go.color_b.a ?? 1);
      }
    }
  } catch (e) {
    console.error('加载设置失败:', e);
  }
};

const handleSave = async () => {
  isSaving.value = true;
  saveStatus.value = null;
  try {
    const currentData = await invoke<any>('get_overlay_json', { projectName: props.projectName });
    const ot = settings.outline_type;
    // 同时写入 Vue 简化键 + Python 兼容键，保证打包逻辑与旧 overlay.json 都能读
    const mergedData = {
      ...currentData,
      no_shadow: settings.no_shadow,
      custom_glint: settings.custom_glint,
      outline_type: ot,
      core_shadow: { enabled: settings.no_shadow },
      core_outline: {
        enabled: ot === 'default',
        color: { ...settings.core_outline.color },
        thickness: settings.core_outline.thickness
      },
      core_outline_rainbow: { enabled: ot === 'rainbow' || ot === 'rainbow_hexian' },
      core_gradient_outline: {
        enabled: ot === 'gradient',
        color_a: { ...settings.core_gradient_outline.color_a },
        color_b: { ...settings.core_gradient_outline.color_b }
      }
    };
    await invoke('save_overlay_json', { projectName: props.projectName, data: mergedData });
    saveStatus.value = { text: t('dialog.visual.saved'), type: 'success' };
    setTimeout(() => { visible.value = false; }, 1500);
  } catch (e) {
    saveStatus.value = { text: t('dialog.visual.saveFailed', { error: e }), type: 'error' };
  } finally {
    isSaving.value = false;
  }
};

onMounted(loadSettings);
</script>

<style scoped>
/*
 * ── 右侧抽屉外壳 ────────────────────────────────────────────────────
 *
 * 这一份是「侧滑面板」，不是居中弹窗，所以不套 shared.css 的 .dialog-overlay /
 * .dialog-content（那对是居中 flex 布局，套上去面板就不再贴右边缘）。
 * 但取值必须是同一套：暖色遮罩、暖色发丝线、同一组字号与圆角。
 *
 * 原值 rgba(15, 23, 42, ·) 是冷蓝灰，压在暖白界面上会发脏。
 */
/* .sidebar-overlay 已统一到 shared.css
   （本地那份与全局逐字相同，属于纯重复，删掉）。 */

.visual-panel.sidebar-content {
  position: fixed;
  top: 0;
  right: 0;
  width: min(420px, 94vw);
  height: 100vh;
  z-index: 201;
  background: #fff;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  outline: none;
  /* App.vue 的 .page-shell > * > * 入场 stagger 会锁 opacity，这里必须压回 1 */
  opacity: 1 !important;
  transform: translateX(100%);
  /* 抽屉是真脱离文档流的浮层，投影保留（卡片才一律不要）—— 但换成暖色。
     取值必须与 useSidebarSlide 传入的 inline 投影一致，否则进场瞬间会闪一下 */
  box-shadow: -12px 0 36px rgba(28, 24, 18, 0.12);
  /* 靠右贴边，单靠投影分不出边界；补一条暖色发丝线，和页面分隔线同一支 */
  border-left: 1px solid rgba(120, 110, 95, 0.14);
  will-change: transform, box-shadow;
}

.panel-header {
  /* 顶部避开浮动窗口控制按钮 */
  padding: 56px 1.5rem 14px;
  border-bottom: 1px solid rgba(120, 110, 95, 0.14);
  flex-shrink: 0;
}

/* 15px / 600：与 shared.css 的 .dialog-header h3 同一档。
   原来是 18px/700 + 冷蓝 #0f172a —— 层级由字号与留白建立，不靠字重压人 */
.panel-title {
  margin: 0;
  font-size: 15px;
  font-weight: 600;
  letter-spacing: -0.01em;
  color: #14171a;
}

.panel-body {
  flex: 1;
  padding: 18px 1.5rem 20px;
  display: flex;
  flex-direction: column;
  /* 区块之间靠留白分组，所以这里的间距比任何组内间距都大 */
  gap: 24px;
  overflow-y: auto;
  min-height: 0;
}
.panel-body::-webkit-scrollbar { width: 6px; }
.panel-body::-webkit-scrollbar-track { background: transparent; }
.panel-body::-webkit-scrollbar-thumb {
  /* 滚动条也是冷灰重灾区：rgba(0,0,0,·) 换成暖色 */
  background: rgba(120, 110, 95, 0.2);
  border-radius: 999px;
}

/*
 * ── 区块：小字标签 + 内容 ────────────────────────────────────────────
 * 原来是一个没有分界的竖列（两个 #f8fafc 冷灰填充块 + 一组卡片），
 * 看不出哪些控件属于同一件事。现在用小字标签分组、组内用发丝线分隔。
 */
.option-group { display: flex; flex-direction: column; gap: 10px; }

.group-header { display: flex; flex-direction: column; gap: 3px; }

/* 10.5px / .18em 小字标签：全项目统一的分组标记 */
.group-label {
  font-size: 10.5px;
  font-weight: 600;
  letter-spacing: 0.18em;
  text-transform: uppercase;
  color: #7d776d;
}

/* 分组说明比标签再弱一档，免得和标签抢注意力 */
.group-desc { font-size: 12px; line-height: 1.5; color: #9c968c; }

/* ── 开关行 ── */
.switch-list { display: flex; flex-direction: column; }

.option-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 16px;
  padding: 12px 0;
}
/* 行间一条发丝线就够分组了。原来每行套一个 #f8fafc 冷灰填充块，
   两块叠在一起反而看不出它们属于同一类设置 */
.option-item + .option-item { border-top: 1px solid rgba(120, 110, 95, 0.14); }

.option-info { min-width: 0; }
.option-label { font-size: 14px; font-weight: 500; color: #14171a; }
.option-desc { margin-top: 3px; font-size: 12px; line-height: 1.5; color: #7d776d; }

/* ── 描边选项卡 ── */
.outline-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; }

/* 页内块：一枚暖色发丝边 + 8px 圆角，无投影。
   原来 2px #f1f5f9 冷灰描边 + 12px 圆角，看着像另一种控件 */
.outline-card {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  /* App.vue 的 `.app-container button` 会给按钮挂 nowrap + overflow:hidden，
     两列布局下「彩虹描边（其他显卡）」会被直接切掉 —— 这里显式恢复折行 */
  white-space: normal;
  border: 1px solid rgba(120, 110, 95, 0.18);
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.6);
  color: #5f5a51;
  font-family: inherit;
  font-size: 13px;
  text-align: left;
  cursor: pointer;
  transition: border-color 0.18s ease, background 0.18s ease, color 0.18s ease;
}
.outline-card:hover {
  border-color: rgba(120, 110, 95, 0.36);
  background: #fff;
  color: #14171a;
}

/* 选中态用墨色描边，与首页一致；主题色只留给主操作按钮 */
.outline-card.active { border-color: rgba(20, 23, 26, 0.5); background: rgba(20, 23, 26, 0.035); }
.outline-card.active .outline-name { color: #14171a; }

/* 色板：50% 圆点，与开关滑块、状态点同一档圆角 */
.outline-preview {
  width: 32px;
  height: 32px;
  flex-shrink: 0;
  border-radius: 50%;
  /* 浅色色板（无描边 / 白色）需要一圈发丝线才看得出边界 —— 暖色，不用 rgba(0,0,0,·) */
  border: 1px solid rgba(120, 110, 95, 0.22);
}

/* 名字最长的是「彩虹描边（其他显卡）」，两列下必然折行，行高压到 1.35 才不散 */
.outline-name { font-size: 13px; font-weight: 500; line-height: 1.35; color: inherit; }

/* ── 细节区（标准描边 / 双色循环） ── */
.outline-detail {
  padding: 12px;
  border-radius: 8px;
  /* 暖色淡面替掉原来的 rgba(20, 23, 26, 0.03) + #eef2f7 冷边 */
  background: rgba(120, 110, 95, 0.05);
  border: 1px solid rgba(120, 110, 95, 0.14);
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.detail-row {
  display: grid;
  grid-template-columns: 96px minmax(0, 1fr) auto;
  gap: 10px;
  align-items: center;
  font-size: 13px;
}

.detail-label { font-size: 12px; font-weight: 500; line-height: 1.35; color: #7d776d; }

.detail-block { display: flex; flex-direction: column; gap: 8px; }

.color-row { display: flex; align-items: center; gap: 8px; }
/* 28px 高的小控件，圆角取小图标钮那一档（6px），不用按钮的 8px */
.color-input {
  width: 36px;
  height: 28px;
  padding: 0;
  border: 1px solid rgba(120, 110, 95, 0.22);
  border-radius: 6px;
  background: none;
  cursor: pointer;
}
.color-hex { font-family: ui-monospace, monospace; font-size: 12px; color: #5f5a51; }

.range-input { width: 100%; min-width: 0; accent-color: #14171a; }
.range-val {
  min-width: 40px;
  text-align: right;
  font-size: 12px;
  font-weight: 500;
  font-variant-numeric: tabular-nums;
  color: #14171a;
}

.detail-hint { margin: 0; font-size: 12px; line-height: 1.55; color: #9c968c; }

/* ── 双色循环预览条 ── */
/* 这条显示的是用户选的两种颜色，所以它自身不带任何颜色，只负责给出边界 */
.gradient-preview {
  height: 44px;
  border-radius: 8px;
  border: 1px solid rgba(120, 110, 95, 0.18);
  position: relative;
  overflow: hidden;
  display: flex;
  align-items: center;
  justify-content: center;
}

.preview-label {
  font-size: 10.5px;
  font-weight: 600;
  letter-spacing: 0.18em;
  text-transform: uppercase;
  color: rgba(255, 255, 255, 0.94);
  /* 底色由用户决定，可能很浅；描影保证可读 —— 原来是 rgba(0,0,0,·) 冷黑 */
  text-shadow: 0 1px 2px rgba(28, 24, 18, 0.55);
  pointer-events: none;
}

@keyframes grad-cycle {
  from { background-position: 0% 50%; }
  to { background-position: 200% 50%; }
}

/* ── 开关 ── */
/* 与设置页 / 转换页的开关同尺寸（42×24、滑块 18px、行程 18px），勾选态用墨色 */
.switch { position: relative; display: inline-block; width: 42px; height: 24px; flex-shrink: 0; }
.switch input { opacity: 0; width: 0; height: 0; }
.slider {
  position: absolute;
  inset: 0;
  cursor: pointer;
  /* 未选中态用暖灰：原 #cbd5e1 是冷板岩灰 */
  background-color: rgba(120, 110, 95, 0.28);
  transition: background-color 0.2s ease;
  border-radius: 999px;
}
.slider:before {
  position: absolute;
  content: "";
  height: 18px;
  width: 18px;
  left: 3px;
  bottom: 3px;
  background-color: #fff;
  transition: transform 0.2s var(--ease-out);
  border-radius: 50%;
}
input:checked + .slider { background-color: #14171a; }
input:checked + .slider:before { transform: translateX(18px); }
/* 原生 checkbox 已被压成 0×0，键盘聚焦只能画在轨道上才看得见 */
input:focus-visible + .slider { outline: 2px solid rgba(20, 23, 26, 0.4); outline-offset: 2px; }

.panel-footer {
  padding: 12px 1.5rem;
  /* 暖色发丝线，和页面里的分隔线同一支 */
  border-top: 1px solid rgba(120, 110, 95, 0.14);
  background: #fff;
  display: flex;
  justify-content: flex-end;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}

.save-status {
  /* 状态靠左、按钮靠右：用 margin-right:auto 而不是 space-between ——
     状态是 v-if，换成 space-between 后没有状态时按钮会跳到左边 */
  margin-right: auto;
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  font-weight: 500;
  line-height: 1.4;
}
.save-status.success { color: #10b981; }
/* 危险色用规格里的 #b3261e（原 #ef4444 是偏冷的 Tailwind 红） */
.save-status.error { color: #b3261e; }

.footer-btns { display: flex; gap: 8px; align-items: center; flex-shrink: 0; }

/* 主按钮只补尺寸差异：共享的 .primary-btn 高 40px，而 .app-container .primary-btn
   会把圆角统一到 10px（--ui-radius-btn）—— 都偏离 8px 阶梯，也和旁边的 .ghost-btn
   （36px / 8px）高低不齐。用 .panel-footer 前缀提高特异度，免得靠加载顺序赢全局规则 */
.panel-footer .primary-btn {
  height: 36px;
  border-radius: 8px;
}

/*
 * 动效：抽屉的进出场由 useSidebarSlide 用 inline transition 驱动（transform + box-shadow），
 * 本文件没有任何 transform 关键帧动画 —— 所以不存在 fill-mode 写成 both/forwards 后
 * 永久留一个 translate: 0 0、把面板变成包含块（fixed 子元素跟着它走）的隐患。
 * 唯一的循环动画是预览条，它动的是 background-position，不是 transform。
 */
@media (prefers-reduced-motion: reduce) {
  /* 预览条的循环渐变是装饰，冻结在静止帧上两种颜色仍然看得见。
     动效是 inline style 挂上去的，所以这里必须用 !important 才压得住。
     转圈图标是状态反馈，App.vue 已显式保留，这里不动它 */
  .gradient-preview { animation: none !important; }
  .outline-card,
  .slider,
  .slider:before,
  .panel-footer .primary-btn { transition: none; }
}
</style>
