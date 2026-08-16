<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { useLanguage, type SupportedLocale } from '../composables/useLanguage'

const { t, locale } = useI18n()
const { setLanguage } = useLanguage()

const emit = defineEmits<{ complete: [] }>()

const step = ref(0)
const totalSteps = 5

// Step data
const selectedLang = ref<SupportedLocale>(locale.value as SupportedLocale)
const userName = ref('')
const outputMode = ref<'follow' | 'fixed'>('follow')
const notificationEnabled = ref(true)
const notificationMode = ref<'system' | 'app' | 'both'>('both')
// Post-conversion source-pack handling + auto-open output folder.
// Default `ask` is the safest choice for first-time users: the app
// will prompt before deleting any of their packs.
const sourceHandling = ref<'ask' | 'delete' | 'keep'>('ask')
const openOutputAfterConvert = ref<boolean>(true)

// 每步的标题/描述（显示在右侧内容区顶部）
const stepMeta = computed(() => {
  const metas = [
    { title: t('oobe.step1.title'), desc: t('oobe.step1.desc') },
    { title: t('oobe.step2.title'), desc: t('oobe.step2.desc') },
    { title: t('oobe.step3.title', { name: userName.value }), desc: t('oobe.step3.desc') },
    { title: t('oobe.step4.title'), desc: t('oobe.step4.desc') },
    { title: t('oobe.step5.title', { name: userName.value }), desc: t('oobe.step5.desc') },
  ]
  return metas[step.value] ?? metas[0]
})

// ── Backup import (OOBE) ─────────────────────────────────────────
//
// When the user previously picked “Delete user profile” from
// Settings, the Rust factory_reset path writes a backup to
// `~/.2pyr/backups/...` and updates `~/.2pyr/last_backup.json`.
// On the next launch we surface that as an opt-in import on the
// language step so the user doesn't have to redo OOBE from scratch.

interface BackupInfo {
  exists: boolean
  path: string | null
  created_at: string | null
  summary: {
    user_name: string | null
    output_mode: string | null
    notification_mode: string | null
    source_handling: string | null
    open_output_after_convert: boolean | null
    theme_color: string | null
    language: string | null
  } | null
}
const backupInfo = ref<BackupInfo | null>(null)
const showBackupPreview = ref(false)
const importing = ref(false)

onMounted(async () => {
  try {
    backupInfo.value = await invoke<BackupInfo>('get_last_backup_info')
  } catch {
    backupInfo.value = { exists: false, path: null, created_at: null, summary: null }
  }
})

async function importBackup() {
  importing.value = true
  try {
    await invoke<string>('import_last_backup')
    // Restore the language from localStorage if it was saved (best
    // effort — falls through to the chosen OOBE language if missing).
    localStorage.setItem('language', selectedLang.value)
    // Hand off to App.vue — settings are now populated, OOBE is done.
    showBackupPreview.value = false
    emit('complete')
  } catch (e) {
    console.error('[StartupPage] import_last_backup failed:', e)
    importing.value = false
  }
}

const canNext = computed(() => {
  if (step.value === 0) return true
  if (step.value === 1) return userName.value.trim().length > 0
  return true
})

function nextStep() {
  if (step.value < totalSteps - 1) step.value++
}

function prevStep() {
  if (step.value > 0) step.value--
}

function selectLang(lang: SupportedLocale) {
  selectedLang.value = lang
  setLanguage(lang)
  nextStep()
}

async function finish() {
  console.log('[StartupPage] finish() called')
  try {
    console.log('[StartupPage] calling update_config...')
    const result = await invoke('update_config', {
      patch: {
        initialized: true,
        userName: userName.value.trim(),
        outputMode: outputMode.value,
        notificationEnabled: notificationEnabled.value,
        notificationMode: notificationMode.value,
        sourceHandling: sourceHandling.value,
        openOutputAfterConvert: openOutputAfterConvert.value,
      }
    })
    console.log('[StartupPage] update_config result:', result)
    localStorage.setItem('outputMode', outputMode.value)
    localStorage.setItem('notificationEnabled', String(notificationEnabled.value))
    localStorage.setItem('notificationMode', notificationMode.value)
    localStorage.setItem('sourceHandling', sourceHandling.value)
    localStorage.setItem('openOutputAfterConvert', String(openOutputAfterConvert.value))
  } catch (e) {
    console.error('[StartupPage] Failed to save startup config:', e)
  }
  emit('complete')
}
</script>

<template>
  <div class="startup-root">
    <!-- ═══ 顶部：品牌（左） + 步骤指示器（右），同一底色无分割 ═══ -->
    <header class="startup-top">
      <div class="top-brand">
        <svg class="startup-logo" viewBox="0 0 680 680" fill="none" xmlns="http://www.w3.org/2000/svg">
          <g stroke="currentColor" stroke-linecap="round">
            <line x1="260" y1="520" x2="200" y2="390" stroke-width="7"/>
            <line x1="260" y1="520" x2="500" y2="190" stroke-width="7"/>
            <line x1="200" y1="390" x2="500" y2="190" stroke-width="7"/>
            <line x1="260" y1="520" x2="374" y2="274" stroke-width="5"/>
          </g>
        </svg>
        <div class="top-brand-text">
          <span class="top-brand-name">2-Pyramid</span>
          <span class="top-brand-tagline">{{ t('oobe.asideTagline') }}</span>
        </div>
      </div>

      <div class="panel-steps">
        <template v-for="i in totalSteps" :key="i">
          <span v-if="i > 1" class="startup-step-line" :class="{ done: i - 2 < step }"></span>
          <span class="startup-step-dot" :class="{ active: i - 1 === step, done: i - 1 < step }">
            <i v-if="i - 1 < step" class="ri-check-line"></i>
            <template v-else>{{ i }}</template>
          </span>
        </template>
      </div>
    </header>

    <!-- ═══ 内容区 ═══ -->
    <main class="startup-panel">
      <div class="panel-content">
        <Transition name="startup-slide" mode="out-in">
          <!-- Step 1: Language -->
          <div v-if="step === 0" class="startup-step" key="lang">
            <h2 class="step-title">{{ stepMeta.title }}</h2>
            <p class="step-desc">{{ stepMeta.desc }}</p>
            <div class="lang-cards">
              <button class="lang-card" :class="{ active: selectedLang === 'zh-CN' }" @click="selectLang('zh-CN')">
                <span class="lang-check"><i class="ri-check-line"></i></span>
                <span class="lang-flag">
                  <svg viewBox="0 0 40 30" fill="none"><rect width="40" height="30" rx="4" fill="#DE2910"/><path d="M12 6L13.8 11.4H19.5L14.8 14.9L16.6 20.3L12 16.8L7.4 20.3L9.2 14.9L4.5 11.4H10.2L12 6Z" fill="#FFDE00"/></svg>
                </span>
                <span class="lang-name">{{ t('oobe.step1.zh') }}</span>
              </button>
              <button class="lang-card" :class="{ active: selectedLang === 'en-US' }" @click="selectLang('en-US')">
                <span class="lang-check"><i class="ri-check-line"></i></span>
                <span class="lang-flag">
                  <svg viewBox="0 0 40 30" fill="none"><rect width="40" height="30" rx="4" fill="#012169"/><path d="M0 0L40 30M40 0L0 30" stroke="#fff" stroke-width="5"/><path d="M0 0L40 30M40 0L0 30" stroke="#C8102E" stroke-width="3"/><path d="M20 0V30M0 15H40" stroke="#fff" stroke-width="9"/><path d="M20 0V30M0 15H40" stroke="#C8102E" stroke-width="6"/></svg>
                </span>
                <span class="lang-name">{{ t('oobe.step1.en') }}</span>
              </button>
            </div>

            <button
              v-if="backupInfo?.exists"
              class="startup-restore-btn"
              type="button"
              @click="showBackupPreview = true"
            >
              <i class="ri-history-line" aria-hidden="true"></i>
              <span>{{ t('oobe.importBackup.cta') }}</span>
            </button>
          </div>

          <!-- Step 2: User Name -->
          <div v-else-if="step === 1" class="startup-step" key="name">
            <h2 class="step-title">{{ stepMeta.title }}</h2>
            <p class="step-desc">{{ stepMeta.desc }}</p>
            <div class="name-field">
              <i class="ri-user-line name-field-icon" aria-hidden="true"></i>
              <input
                v-model="userName"
                class="startup-input"
                :placeholder="t('oobe.step2.placeholder')"
                autofocus
                @keydown.enter="canNext && nextStep()"
              />
            </div>
            <p class="field-hint">{{ t('oobe.step2.hint') }}</p>
          </div>

          <!-- Step 3: Welcome -->
          <div v-else-if="step === 2" class="startup-step" key="welcome">
            <h2 class="step-title">{{ stepMeta.title }}</h2>
            <p class="step-desc">{{ stepMeta.desc }}</p>
            <div class="welcome-card">
              <i class="ri-hand-heart-line" aria-hidden="true"></i>
              <p>{{ t('oobe.step3.card', { name: userName }) }}</p>
            </div>
          </div>

          <!-- Step 4: Settings -->
          <div v-else-if="step === 3" class="startup-step" key="settings">
            <h2 class="step-title">{{ stepMeta.title }}</h2>
            <p class="step-desc">{{ stepMeta.desc }}</p>
            <div class="startup-settings">
              <!-- Output Mode -->
              <div class="startup-setting-row">
                <div class="item-icon">
                  <i class="ri-route-line" aria-hidden="true"></i>
                </div>
                <div class="startup-setting-info">
                  <span class="startup-setting-label">{{ t('oobe.step4.outputMode') }}</span>
                  <span class="startup-setting-desc">{{ t('oobe.step4.outputModeDesc') }}</span>
                </div>
                <div class="segmented">
                  <button class="seg-btn" :class="{ active: outputMode === 'follow' }" @click="outputMode = 'follow'">{{ t('oobe.step4.outputModeFollow') }}</button>
                  <button class="seg-btn" :class="{ active: outputMode === 'fixed' }" @click="outputMode = 'fixed'">{{ t('oobe.step4.outputModeFixed') }}</button>
                </div>
              </div>

              <!-- Notification -->
              <div class="startup-setting-row">
                <div class="item-icon">
                  <i class="ri-notification-3-line" aria-hidden="true"></i>
                </div>
                <div class="startup-setting-info">
                  <span class="startup-setting-label">{{ t('oobe.step4.notification') }}</span>
                  <span class="startup-setting-desc">{{ t('oobe.step4.notificationDesc') }}</span>
                </div>
                <label class="switch">
                  <input type="checkbox" v-model="notificationEnabled" />
                  <span class="slider"></span>
                </label>
              </div>

              <!-- Notification Mode -->
              <div class="startup-setting-row" v-if="notificationEnabled">
                <div class="item-icon">
                  <i class="ri-window-line" aria-hidden="true"></i>
                </div>
                <div class="startup-setting-info">
                  <span class="startup-setting-label">{{ t('settings.notificationMode.label') }}</span>
                  <span class="startup-setting-desc">{{ t('settings.notificationMode.desc') }}</span>
                </div>
                <div class="segmented">
                  <button class="seg-btn" :class="{ active: notificationMode === 'system' }" @click="notificationMode = 'system'">{{ t('settings.notificationMode.system') }}</button>
                  <button class="seg-btn" :class="{ active: notificationMode === 'app' }" @click="notificationMode = 'app'">{{ t('settings.notificationMode.app') }}</button>
                  <button class="seg-btn" :class="{ active: notificationMode === 'both' }" @click="notificationMode = 'both'">{{ t('settings.notificationMode.both') }}</button>
                </div>
              </div>

              <!-- Source Pack Handling (post-conversion) -->
              <div class="startup-setting-row">
                <div class="item-icon">
                  <i class="ri-delete-bin-2-line" aria-hidden="true"></i>
                </div>
                <div class="startup-setting-info">
                  <span class="startup-setting-label">{{ t('oobe.step4.sourceHandling.label') }}</span>
                  <span class="startup-setting-desc">{{ t('oobe.step4.sourceHandling.desc') }}</span>
                </div>
                <div class="segmented">
                  <button class="seg-btn" :class="{ active: sourceHandling === 'ask' }" @click="sourceHandling = 'ask'">{{ t('oobe.step4.sourceHandling.ask') }}</button>
                  <button class="seg-btn" :class="{ active: sourceHandling === 'delete' }" @click="sourceHandling = 'delete'">{{ t('oobe.step4.sourceHandling.delete') }}</button>
                  <button class="seg-btn" :class="{ active: sourceHandling === 'keep' }" @click="sourceHandling = 'keep'">{{ t('oobe.step4.sourceHandling.keep') }}</button>
                </div>
              </div>

              <!-- Open Output Folder After Convert -->
              <div class="startup-setting-row">
                <div class="item-icon">
                  <i class="ri-external-link-line" aria-hidden="true"></i>
                </div>
                <div class="startup-setting-info">
                  <span class="startup-setting-label">{{ t('oobe.step4.openOutputAfterConvert.label') }}</span>
                  <span class="startup-setting-desc">{{ t('oobe.step4.openOutputAfterConvert.desc') }}</span>
                </div>
                <label class="switch">
                  <input type="checkbox" v-model="openOutputAfterConvert" />
                  <span class="slider"></span>
                </label>
              </div>
            </div>
          </div>

          <!-- Step 5: Final -->
          <div v-else class="startup-step" key="done">
            <h2 class="step-title">{{ stepMeta.title }}</h2>
            <p class="step-desc">{{ stepMeta.desc }}</p>
            <div class="welcome-card">
              <i class="ri-rocket-2-line" aria-hidden="true"></i>
              <p>{{ t('oobe.step5.card', { name: userName }) }}</p>
            </div>
          </div>
        </Transition>
      </div>

      <!-- 底部药丸导航 -->
      <div class="panel-nav">
        <div class="nav-pill">
          <button
            v-if="step > 0 && step < totalSteps - 1"
            class="startup-btn startup-btn-ghost"
            @click="prevStep"
          >
            <i class="ri-arrow-left-s-line"></i>
            {{ t('oobe.back') }}
          </button>
          <button
            v-if="step < totalSteps - 1"
            class="startup-btn startup-btn-primary"
            :disabled="step === 1 && !canNext"
            @click="nextStep"
          >
            {{ t('oobe.next') }}
            <i class="ri-arrow-right-s-line"></i>
          </button>
          <button
            v-else
            class="startup-btn startup-btn-primary startup-btn-finish"
            @click="finish"
          >
            {{ t('oobe.getStarted') }}
            <i class="ri-rocket-2-line"></i>
          </button>
        </div>
      </div>
    </main>

    <!-- Backup import preview dialog. Shown only when the user
         tapped the “import previous settings” CTA on step 0 and
         `backupInfo.exists` was true. Confirming runs
         `import_last_backup` on Rust and emits `complete` so App.vue
         closes the OOBE overlay. -->
    <transition name="startup-slide">
      <div v-if="showBackupPreview" class="startup-backup-overlay" @click.self="showBackupPreview = false">
        <div class="startup-backup-card">
          <div class="startup-backup-head">
            <i class="ri-history-line" aria-hidden="true"></i>
            <h3>{{ t('oobe.importBackup.previewTitle') }}</h3>
            <button class="startup-backup-close" @click="showBackupPreview = false" :aria-label="t('common.close')">×</button>
          </div>
          <p class="startup-backup-desc">{{ t('oobe.importBackup.previewBody') }}</p>

          <ul class="startup-backup-list" v-if="backupInfo?.summary">
            <li v-if="backupInfo.summary.user_name">
              <span class="label">{{ t('settings.userName.label') }}</span>
              <span class="value">{{ backupInfo.summary.user_name }}</span>
            </li>
            <li v-if="backupInfo.summary.output_mode">
              <span class="label">{{ t('settings.outputMode.label') }}</span>
              <span class="value">{{ t(`settings.outputMode.${backupInfo.summary.output_mode}`, backupInfo.summary.output_mode) }}</span>
            </li>
            <li v-if="backupInfo.summary.source_handling">
              <span class="label">{{ t('settings.sourceHandling.label') }}</span>
              <span class="value">{{ t(`settings.sourceHandling.${backupInfo.summary.source_handling}`, backupInfo.summary.source_handling) }}</span>
            </li>
            <li v-if="backupInfo.summary.open_output_after_convert !== null">
              <span class="label">{{ t('settings.openOutputAfterConvert.label') }}</span>
              <span class="value">{{ backupInfo.summary.open_output_after_convert ? t('common.on') : t('common.off') }}</span>
            </li>
          </ul>

          <div class="startup-backup-foot">
            <button class="startup-btn startup-btn-ghost" @click="showBackupPreview = false" :disabled="importing">
              {{ t('common.cancel') }}
            </button>
            <button class="startup-btn startup-btn-primary" @click="importBackup" :disabled="importing">
              {{ importing ? t('common.loading') : t('oobe.importBackup.confirmBtn') }}
            </button>
          </div>
        </div>
      </div>
    </transition>
  </div>
</template>

<style scoped>
.startup-root {
  position: fixed;
  inset: 0;
  z-index: 10000;
  display: flex;
  flex-direction: column;
  background: #f8fafc;
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", "PingFang SC", "Microsoft YaHei", Roboto, "Helvetica Neue", Arial, sans-serif;
  overflow: hidden;
}

/* ═══ 顶部品牌 + 步骤指示器 ═══ */
.startup-top {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 24px 36px 0;
  flex-shrink: 0;
}

.top-brand {
  display: flex;
  align-items: center;
  gap: 12px;
  color: var(--theme-color);
  min-width: 0;
}

.startup-logo {
  width: 40px;
  height: 40px;
  flex-shrink: 0;
}

.top-brand-text {
  display: flex;
  flex-direction: column;
  gap: 1px;
  min-width: 0;
}

.top-brand-name {
  font-size: 16px;
  font-weight: 800;
  letter-spacing: -0.4px;
  color: #1a1a2e;
  line-height: 1.2;
}

.top-brand-tagline {
  font-size: 11.5px;
  color: #94a3b8;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* ═══ 主内容区 ═══ */
.startup-panel {
  flex: 1 1 auto;
  display: flex;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
}

.panel-steps {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
}

.startup-step-dot {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 12px;
  font-weight: 700;
  font-family: inherit;
  background: rgba(0, 0, 0, 0.06);
  color: #94a3b8;
  transition: all 0.3s cubic-bezier(0.22, 1, 0.36, 1);
}

.startup-step-dot i { font-size: 14px; }

.startup-step-dot.active {
  background: var(--theme-color);
  color: #fff;
  box-shadow: 0 2px 10px color-mix(in srgb, var(--theme-color) 40%, transparent);
}

.startup-step-dot.done {
  background: color-mix(in srgb, var(--theme-color) 18%, transparent);
  color: var(--theme-color);
}

.startup-step-line {
  width: 16px;
  height: 2px;
  border-radius: 1px;
  background: rgba(0, 0, 0, 0.08);
  transition: background 0.3s ease;
}

.startup-step-line.done {
  background: color-mix(in srgb, var(--theme-color) 45%, transparent);
}

.panel-content {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px 48px 12px;
  min-height: 0;
  overflow-y: auto;
}

.startup-step {
  width: 100%;
  max-width: 500px;
  display: flex;
  flex-direction: column;
  align-items: center;
}

.step-title {
  margin: 0 0 8px;
  font-size: 24px;
  font-weight: 800;
  letter-spacing: -0.4px;
  color: #111827;
  text-align: center;
}

.step-desc {
  margin: 0 0 26px;
  font-size: 13.5px;
  line-height: 1.7;
  color: #6b7280;
  text-align: center;
  max-width: 380px;
}

/* 语言卡 */
.lang-cards {
  display: flex;
  gap: 16px;
  width: 100%;
}

.lang-card {
  position: relative;
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 10px;
  padding: 26px 16px 20px;
  background: #fff;
  border: 2px solid rgba(0, 0, 0, 0.07);
  border-radius: 18px;
  cursor: pointer;
  transition: all 0.28s cubic-bezier(0.22, 1, 0.36, 1);
  font-family: inherit;
  overflow: hidden;
}

.lang-card:hover {
  border-color: color-mix(in srgb, var(--theme-color) 45%, transparent);
  transform: translateY(-3px);
  box-shadow: 0 10px 26px rgba(15, 23, 42, 0.08);
}

.lang-card.active {
  border-color: var(--theme-color);
  background: linear-gradient(160deg, color-mix(in srgb, var(--theme-color) 7%, white) 0%, #fff 100%);
  box-shadow:
    0 0 0 3px color-mix(in srgb, var(--theme-color) 18%, transparent),
    0 8px 24px color-mix(in srgb, var(--theme-color) 14%, transparent);
}

.lang-check {
  position: absolute;
  top: 10px;
  right: 10px;
  width: 20px;
  height: 20px;
  border-radius: 50%;
  background: var(--theme-color);
  color: #fff;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 12px;
  transform: scale(0);
  opacity: 0;
  transition: transform 0.3s cubic-bezier(0.34, 1.56, 0.64, 1), opacity 0.2s;
  box-shadow: 0 2px 8px color-mix(in srgb, var(--theme-color) 40%, transparent);
}
.lang-card.active .lang-check {
  transform: scale(1);
  opacity: 1;
}

.lang-flag {
  width: 44px;
  height: 33px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.lang-flag svg {
  width: 100%;
  height: 100%;
  border-radius: 5px;
  box-shadow: 0 2px 6px rgba(0, 0, 0, 0.12);
  transition: transform 0.28s ease;
}
.lang-card:hover .lang-flag svg {
  transform: scale(1.06);
}

.lang-name {
  font-size: 14px;
  font-weight: 600;
  color: #374151;
}

/* 名字输入 */
.name-field {
  position: relative;
  width: 100%;
}

.name-field-icon {
  position: absolute;
  left: 18px;
  top: 50%;
  transform: translateY(-50%);
  font-size: 18px;
  color: #9ca3af;
  pointer-events: none;
}

.startup-input {
  width: 100%;
  padding: 15px 20px 15px 46px;
  font-size: 16px;
  font-family: inherit;
  border: 2px solid rgba(0, 0, 0, 0.1);
  border-radius: 14px;
  background: #fff;
  color: #1a1a2e;
  outline: none;
  transition: all 0.25s ease;
}

.startup-input:focus {
  border-color: var(--theme-color);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--theme-color) 15%, transparent);
}

.startup-input::placeholder {
  color: #9ca3af;
}

.field-hint {
  margin: 12px 0 0;
  font-size: 12.5px;
  color: #94a3b8;
  text-align: center;
}

/* 欢迎/完成卡片 */
.welcome-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 14px;
  padding: 32px 28px;
  background: #fff;
  border: 1px solid rgba(0, 0, 0, 0.06);
  border-radius: 18px;
  box-shadow: 0 4px 20px rgba(15, 23, 42, 0.05);
  text-align: center;
}

.welcome-card i {
  font-size: 40px;
  color: var(--theme-color);
}

.welcome-card p {
  margin: 0;
  font-size: 14px;
  line-height: 1.7;
  color: #475569;
}

/* 设置行 —— 与设置页同款卡片 */
.startup-settings {
  width: 100%;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.startup-setting-row {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 14px;
  background: #fff;
  border: 1px solid rgba(0, 0, 0, 0.06);
  border-radius: 14px;
  box-shadow: 0 1px 3px rgba(15, 23, 42, 0.04);
  transition: border-color 0.2s ease, box-shadow 0.2s ease;
}
.startup-setting-row:hover {
  border-color: color-mix(in srgb, var(--theme-color) 25%, transparent);
  box-shadow: 0 3px 12px rgba(15, 23, 42, 0.05);
}

.item-icon {
  flex: 0 0 36px;
  width: 36px;
  height: 36px;
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 17px;
  color: var(--theme-color);
  background: color-mix(in srgb, var(--theme-color) 12%, transparent);
}

.startup-setting-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
  flex: 1 1 auto;
}

.startup-setting-label {
  font-size: 14px;
  font-weight: 600;
  color: #1a1a2e;
}

.startup-setting-desc {
  font-size: 12px;
  color: #9ca3af;
}

/* 分段按钮 */
.segmented {
  display: flex;
  background: rgba(0, 0, 0, 0.05);
  border-radius: 10px;
  padding: 3px;
  flex-shrink: 0;
}

.seg-btn {
  padding: 6px 14px;
  font-size: 12px;
  font-weight: 600;
  font-family: inherit;
  border: none;
  border-radius: 8px;
  background: transparent;
  color: #6b7280;
  cursor: pointer;
  transition: all 0.2s ease;
  white-space: nowrap;
}

.seg-btn.active {
  background: #fff;
  color: var(--theme-color);
  box-shadow: 0 1px 4px rgba(0, 0, 0, 0.08);
}

.seg-btn:hover:not(.active) {
  color: #374151;
}

/* 开关 */
.switch {
  position: relative;
  width: 44px;
  height: 24px;
  flex-shrink: 0;
}

.switch input {
  opacity: 0;
  width: 0;
  height: 0;
}

.slider {
  position: absolute;
  inset: 0;
  background: #d1d5db;
  border-radius: 24px;
  cursor: pointer;
  transition: all 0.3s ease;
}

.slider::before {
  content: '';
  position: absolute;
  width: 18px;
  height: 18px;
  left: 3px;
  bottom: 3px;
  background: #fff;
  border-radius: 50%;
  transition: all 0.3s ease;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.15);
}

.switch input:checked + .slider {
  background: var(--theme-color);
}

.switch input:checked + .slider::before {
  transform: translateX(20px);
}

/* ═══ 底部药丸导航 ═══ */
.panel-nav {
  display: flex;
  justify-content: center;
  padding: 8px 24px 26px;
  flex-shrink: 0;
}

.nav-pill {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px;
  background: #fff;
  border: 1px solid rgba(0, 0, 0, 0.06);
  border-radius: 999px;
  box-shadow:
    0 10px 34px rgba(15, 23, 42, 0.10),
    0 2px 8px rgba(15, 23, 42, 0.05);
  transition: transform 0.35s cubic-bezier(0.34, 1.56, 0.64, 1), box-shadow 0.35s ease;
}

.nav-pill:hover {
  transform: translateY(-2px);
  box-shadow:
    0 16px 40px rgba(15, 23, 42, 0.13),
    0 4px 12px rgba(15, 23, 42, 0.07);
}

.startup-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 11px 26px;
  font-size: 14px;
  font-weight: 600;
  font-family: inherit;
  border: none;
  border-radius: 999px;
  cursor: pointer;
  transition: all 0.3s cubic-bezier(0.34, 1.56, 0.64, 1);
}

.startup-btn-primary {
  background: var(--theme-color);
  color: #fff;
  box-shadow: 0 4px 14px color-mix(in srgb, var(--theme-color) 35%, transparent);
}

.startup-btn-primary:hover:not(:disabled) {
  transform: scale(1.05);
  box-shadow: 0 8px 22px color-mix(in srgb, var(--theme-color) 45%, transparent);
}

.startup-btn-primary:active:not(:disabled) {
  transform: scale(0.96);
}

.startup-btn-primary:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.startup-btn-ghost {
  background: rgba(0, 0, 0, 0.05);
  color: #6b7280;
}

.startup-btn-ghost:hover {
  background: rgba(0, 0, 0, 0.08);
  color: #374151;
  transform: scale(1.05);
}

.startup-btn-ghost:active {
  transform: scale(0.96);
}

.startup-btn-finish {
  padding: 12px 30px;
  font-size: 15px;
}

/* 步骤切换动画 */
.startup-slide-enter-active,
.startup-slide-leave-active {
  transition: all 0.4s cubic-bezier(0.22, 1, 0.36, 1);
}

.startup-slide-enter-from {
  opacity: 0;
  transform: translateX(40px);
}

.startup-slide-leave-to {
  opacity: 0;
  transform: translateX(-40px);
}

/* 恢复入口 */
.startup-restore-btn {
  margin-top: 24px;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  background: transparent;
  border: 1px dashed rgba(0, 0, 0, 0.15);
  border-radius: 999px;
  padding: 8px 14px;
  font-size: 12px;
  font-weight: 700;
  color: #475569;
  cursor: pointer;
  transition: background 0.15s, border-color 0.15s, color 0.15s;
}
.startup-restore-btn i { font-size: 14px; }
.startup-restore-btn:hover {
  background: rgba(0, 0, 0, 0.04);
  border-color: var(--theme-color, #007bff);
  color: var(--theme-color, #007bff);
}

/* ═══ 备份导入对话框 ═══ */
.startup-backup-overlay {
  position: fixed;
  inset: 0;
  z-index: 50;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(15, 23, 42, 0.45);
  backdrop-filter: blur(8px);
  -webkit-backdrop-filter: blur(8px);
}
.startup-backup-card {
  width: 460px;
  max-width: 92vw;
  background: #fff;
  border-radius: 18px;
  padding: 22px 22px 18px;
  box-shadow: 0 24px 48px rgba(0, 0, 0, 0.22);
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.startup-backup-head {
  display: flex; align-items: center; gap: 10px;
}
.startup-backup-head i { font-size: 22px; color: var(--theme-color, #007bff); }
.startup-backup-head h3 {
  margin: 0; flex: 1 1 auto;
  font-size: 16px; font-weight: 800; color: #0f172a;
}
.startup-backup-close {
  background: transparent; border: none;
  font-size: 18px; line-height: 1;
  color: #94a3b8; cursor: pointer;
  width: 28px; height: 28px;
  border-radius: 8px;
  transition: background 0.15s, color 0.15s;
}
.startup-backup-close:hover { background: rgba(0, 0, 0, 0.06); color: #475569; }
.startup-backup-desc {
  margin: 0;
  font-size: 12.5px; line-height: 1.5; color: #475569;
}
.startup-backup-list {
  list-style: none;
  margin: 0; padding: 10px 12px;
  background: rgba(0, 0, 0, 0.03);
  border-radius: 12px;
  display: flex; flex-direction: column; gap: 6px;
  max-height: 200px;
  overflow-y: auto;
}
.startup-backup-list li {
  display: flex; align-items: baseline; gap: 12px;
  font-size: 12.5px;
}
.startup-backup-list li .label {
  flex: 0 0 140px;
  color: #64748b; font-weight: 600;
}
.startup-backup-list li .value {
  flex: 1 1 auto; color: #0f172a; font-weight: 700;
}
.startup-backup-foot {
  display: flex; justify-content: flex-end; gap: 8px; margin-top: 4px;
}
.startup-backup-foot .startup-btn:disabled {
  opacity: 0.55; cursor: not-allowed;
}

/* ═══ 窄屏响应式 ═══ */
@media (max-width: 900px) {
  .startup-top {
    padding: 18px 24px 0;
  }
  .top-brand-tagline {
    display: none;
  }
  .panel-content {
    padding: 20px 24px 8px;
  }
  .panel-nav {
    padding: 6px 24px 22px;
  }
}
</style>
