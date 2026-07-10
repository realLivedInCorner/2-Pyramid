<template>
  <div class="fanhua-home">
    <header class="home-header">
      <div class="header-brand">
        2-Pyramid
        <span class="header-version">Dev-2.0.0</span>
      </div>
    </header>

    <div v-if="userName" class="home-greeting">
      <i class="ri-user-smile-line greeting-icon"></i>
      <span class="greeting-text">{{ t('home.greeting', { name: userName }) }}</span>
    </div>

    <div class="vortex-background">
      <div class="blob b1"></div>
      <div class="blob b2"></div>
      <div class="blob b3"></div>
      <div class="blob b4"></div>
    </div>

    <main class="home-main">
      <div class="brand-block">
        <div
          class="app-icon"
          role="img"
          aria-label="2-Pyramid Icon"
          :style="{ WebkitMaskImage: `url(${appIcon})`, maskImage: `url(${appIcon})` }"
        ></div>
        <h1 class="brand-title">2-Pyramid</h1>
        <p class="brand-subtitle">{{ t('home.subtitle') }}</p>
        <p class="brand-subtitle en">The Nextgen Multi‑Version Universal Resource Pack Converter</p>
      </div>
    </main>

    <div class="engine-indicator">
      <span class="breath-dot" aria-hidden="true"></span>
      <span class="engine-text">Engine Ready</span>
    </div>

    <div class="dock-wrap">
      <nav class="action-dock">
      <button class="dock-item" @click="switchToConversion">
        <i class="ri-swap-box-line dock-icon" aria-hidden="true"></i>
        <span>{{ t('home.navConvert') }}</span>
      </button>
      <button class="dock-item" @click="switchToOverlay">
        <i class="ri-file-copy-line dock-icon" aria-hidden="true"></i>
        <span>{{ t('home.navOverlay') }}</span>
      </button>
      <button class="dock-item" @click="switchToSettings">
        <i class="ri-settings-3-line dock-icon" aria-hidden="true"></i>
        <span>{{ t('home.navSettings') }}</span>
      </button>
      </nav>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import appIcon from '../assets/app-icon.png';

const { t } = useI18n();
defineProps<{ userName?: string }>();
const emit = defineEmits(['switch-page']);

const switchToConversion = () => emit('switch-page', 'conversion');
const switchToOverlay = () => emit('switch-page', 'overlay');
const switchToSettings = () => emit('switch-page', 'settings');
</script>

<style scoped>
.fanhua-home {
  width: 100%; height: 100%;
  background: linear-gradient(180deg, #ffffff 0%, color-mix(in srgb, var(--theme-color) 8%, #ffffff) 100%);
  color: #1d1d1f;
  display: flex; flex-direction: column; overflow: hidden;
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", "PingFang SC", "Microsoft YaHei", sans-serif;
  background-size: 200% 200%;
  animation: bg-breathe 14s ease-in-out infinite;
  position: relative;
}

.home-header {
  position: fixed;
  top: 12px;
  left: 16px;
  z-index: 12;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 6px 12px;
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.7);
  border: 1px solid rgba(0, 0, 0, 0.04);
  backdrop-filter: blur(12px);
  font-weight: 700;
  font-size: 12px;
  letter-spacing: 0.4px;
  color: #111827;
}

.header-version {
  margin-left: 6px;
  padding: 2px 8px;
  border-radius: 8px;
  background: color-mix(in srgb, var(--theme-color) 12%, transparent);
  color: var(--theme-color);
  font-weight: 700;
  font-size: 11px;
}

.home-greeting {
  position: fixed;
  top: 12px;
  right: 150px;
  z-index: 12;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 14px;
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.7);
  border: 1px solid rgba(0, 0, 0, 0.04);
  backdrop-filter: blur(12px);
  font-size: 12px;
  font-weight: 600;
  color: #374151;
  transition: all 0.3s ease;
}
.home-greeting:hover {
  background: rgba(255, 255, 255, 0.9);
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.06);
}
.greeting-icon {
  font-size: 14px;
  color: var(--theme-color);
}
.greeting-text {
  letter-spacing: 0.2px;
}
.fanhua-home::before {
  content: "";
  position: fixed;
  inset: 0;
  background:
    linear-gradient(135deg, color-mix(in srgb, var(--theme-color) 5%, transparent) 0%, color-mix(in srgb, var(--theme-color) 1%, transparent) 50%, transparent 100%),
    radial-gradient(60vw 55vh at 20% 20%, color-mix(in srgb, var(--theme-color) 24%, transparent), transparent 60%),
    radial-gradient(50vw 48vh at 80% 30%, color-mix(in srgb, var(--theme-color) 20%, transparent), transparent 58%),
    radial-gradient(55vw 60vh at 50% 80%, color-mix(in srgb, var(--theme-color) 18%, transparent), transparent 65%);
  opacity: 0.85;
  animation: aurora-drift 26s ease-in-out infinite;
  pointer-events: none;
  z-index: 0;
}

.vortex-background { position: absolute; width: 100%; height: 100%; z-index: 1; overflow: hidden; }

.blob {
  position: absolute;
  border-radius: 50%;
  background: radial-gradient(circle at 30% 30%, color-mix(in srgb, var(--theme-color) 40%, transparent), color-mix(in srgb, var(--theme-color) 8%, transparent) 60%, transparent 75%);
  opacity: 0.7;
  animation: blob-float 24s ease-in-out infinite;
  will-change: transform;
}
.b1 { top: 10%; left: 10%; width: 280px; height: 280px; animation-delay: 0s; }
.b2 { top: 20%; right: 10%; width: 360px; height: 360px; animation-delay: 3s; }
.b3 { top: 55%; left: 8%; width: 300px; height: 300px; animation-delay: 6s; }
.b4 { top: 60%; right: 18%; width: 220px; height: 220px; animation-delay: 9s; }

@keyframes blob-float {
  0%, 100% { transform: translate3d(0, 0, 0) scale(1); }
  25% { transform: translate3d(30px, -20px, 0) scale(1.05); }
  50% { transform: translate3d(-20px, 25px, 0) scale(0.97); }
  75% { transform: translate3d(20px, 30px, 0) scale(1.03); }
}

@keyframes bg-breathe {
  0% { background-position: 0% 0%; }
  50% { background-position: 100% 50%; }
  100% { background-position: 0% 100%; }
}

@keyframes aurora-drift {
  0%, 100% { transform: translate3d(0, 0, 0); opacity: 0.75; }
  50% { transform: translate3d(40px, -20px, 0); opacity: 0.9; }
}

.home-main {
  flex: 1; z-index: 5;
  display: flex; align-items: center; justify-content: center;
  padding: 0 40px 40px;
}

.brand-block { text-align: center; display: flex; flex-direction: column; align-items: center; gap: 6px; }
.app-icon {
  width: 128px;
  height: 128px;
  background: var(--theme-color);
  -webkit-mask-repeat: no-repeat;
  mask-repeat: no-repeat;
  -webkit-mask-position: center;
  mask-position: center;
  -webkit-mask-size: contain;
  mask-size: contain;
  filter: drop-shadow(0 12px 24px rgba(0,0,0,0.12));
}
.brand-title { font-size: 64px; font-weight: 900; letter-spacing: -2.5px; margin: 0; }
.brand-subtitle { font-size: 16px; color: #6b7280; margin-top: 12px; font-weight: 600; }
.brand-subtitle.en { font-size: 12px; letter-spacing: 0.6px; text-transform: uppercase; color: #9ca3af; margin-top: 6px; }

.engine-indicator {
  position: fixed; left: 20px; bottom: 20px; z-index: 10;
  display: inline-flex; align-items: center; gap: 8px;
  font-size: 12px; font-weight: 700; color: #0f172a;
  letter-spacing: 0.6px; text-transform: uppercase;
  padding: 6px 10px;
}
.breath-dot {
  width: 10px; height: 10px; border-radius: 50%;
  background: #22c55e;
  box-shadow: 0 0 12px rgba(34,197,94,0.6);
  animation: breath 2.8s ease-in-out infinite;
}
.engine-text { opacity: 0.7; }

@keyframes breath {
  0%, 100% { transform: scale(0.8); opacity: 0.4; }
  50% { transform: scale(1.2); opacity: 1; }
}

.dock-wrap {
  position: fixed; left: 0; right: 0; bottom: 0; height: 80px; z-index: 10;
  display: flex; align-items: flex-end; justify-content: center;
  pointer-events: none;
}
.dock-wrap::before {
  content: "";
  position: absolute; left: 0; right: 0; bottom: 0; height: 30px;
}
.action-dock {
  pointer-events: auto;
  margin-bottom: 16px;
  display: flex; gap: 12px;
  background: rgba(255, 255, 255, 0.75); backdrop-filter: blur(20px);
  padding: 10px 16px; border-radius: 30px; border: 1px solid rgba(0,0,0,0.04);
  box-shadow: 0 12px 30px rgba(0,0,0,0.12);
  transform: translateY(6px) scale(0.97);
  opacity: 0.55;
  transition: transform 0.25s ease, opacity 0.25s ease, background 0.25s ease;
}
.dock-wrap:hover .action-dock {
  transform: translateY(0) scale(1);
  opacity: 1;
  background: rgba(255, 255, 255, 0.9);
}
.dock-item {
  background: none; border: none; display: flex; align-items: center; gap: 8px;
  padding: 10px 14px; cursor: pointer; transition: 0.3s;
  color: #1d1d1f; font-weight: 600; border-radius: 999px;
}
.dock-item:hover { color: var(--theme-color); background: color-mix(in srgb, var(--theme-color) 10%, transparent); transform: translateY(-1px); }
.dock-icon { font-size: 18px; line-height: 1; }

@media (max-width: 720px) {
  .action-dock { flex-direction: column; }
  .dock-wrap { height: 120px; }
  .engine-indicator { left: 14px; bottom: 14px; }
  .brand-title { font-size: 54px; }
}
</style>
