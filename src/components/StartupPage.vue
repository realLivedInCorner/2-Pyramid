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
const closeAction = ref<'ask' | 'close' | 'minimize'>('ask')
// Post-conversion source-pack handling + auto-open output folder.
// Default `ask` is the safest choice for first-time users: the app
// will prompt before deleting any of their packs.
const sourceHandling = ref<'ask' | 'delete' | 'keep'>('ask')
const openOutputAfterConvert = ref<boolean>(true)

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
    close_action: string | null
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
        closeAction: closeAction.value,
        sourceHandling: sourceHandling.value,
        openOutputAfterConvert: openOutputAfterConvert.value,
      }
    })
    console.log('[StartupPage] update_config result:', result)
    localStorage.setItem('outputMode', outputMode.value)
    localStorage.setItem('notificationEnabled', String(notificationEnabled.value))
    localStorage.setItem('notificationMode', notificationMode.value)
    localStorage.setItem('closeAction', closeAction.value)
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
    <div class="startup-aurora"></div>
    <div class="startup-blob blob-1"></div>
    <div class="startup-blob blob-2"></div>
    <div class="startup-blob blob-3"></div>
    <div class="startup-grid"></div>

    <!-- Logo + App name -->
    <div class="startup-brand">
      <svg class="startup-logo" viewBox="0 0 680 680" fill="none" xmlns="http://www.w3.org/2000/svg">
        <g stroke="#000" stroke-linecap="round">
          <line x1="260" y1="520" x2="200" y2="390" stroke-width="7"/>
          <line x1="260" y1="520" x2="500" y2="190" stroke-width="7"/>
          <line x1="200" y1="390" x2="500" y2="190" stroke-width="7"/>
          <line x1="260" y1="520" x2="374" y2="274" stroke-width="5"/>
        </g>
      </svg>
      <span class="startup-brand-name">2-Pyramid</span>
    </div>

    <div class="startup-card">
      <!-- Progress bar -->
      <div class="startup-progress">
        <div class="startup-progress-bar" :style="{ width: ((step + 1) / totalSteps * 100) + '%' }"></div>
      </div>

      <Transition name="startup-slide" mode="out-in">
        <!-- Step 1: Language -->
        <div v-if="step === 0" class="startup-step" key="lang">
          <div class="startup-illustration">
            <svg viewBox="0 0 200 200" fill="none">
              <circle cx="100" cy="100" r="72" stroke="var(--theme-color)" stroke-width="3" opacity="0.15"/>
              <circle cx="100" cy="100" r="56" stroke="var(--theme-color)" stroke-width="2" opacity="0.25"/>
              <ellipse cx="100" cy="100" rx="30" ry="70" stroke="var(--theme-color)" stroke-width="2" opacity="0.3"/>
              <line x1="28" y1="75" x2="172" y2="75" stroke="var(--theme-color)" stroke-width="1.5" opacity="0.25"/>
              <line x1="28" y1="125" x2="172" y2="125" stroke="var(--theme-color)" stroke-width="1.5" opacity="0.25"/>
              <circle cx="100" cy="100" r="72" stroke="var(--theme-color)" stroke-width="3" opacity="0.15" stroke-dasharray="8 4">
                <animateTransform attributeName="transform" type="rotate" from="0 100 100" to="360 100 100" dur="40s" repeatCount="indefinite"/>
              </circle>
              <text x="60" y="98" font-size="28" font-weight="700" fill="var(--theme-color)" opacity="0.8">A</text>
              <text x="118" y="108" font-size="22" font-weight="600" fill="var(--theme-color)" opacity="0.6">文</text>
            </svg>
          </div>
          <h2 class="startup-title">{{ t('oobe.step1.title') }}</h2>
          <p class="startup-desc">{{ t('oobe.step1.desc') }}</p>
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

          <!-- “导入上次的设置” entry point. Only shown when the
               backend reports an existing backup (i.e. the user just
               hit “Delete user profile” on the previous session).
               Clicking it opens the preview card; confirming applies
               the backup and skips straight to the home page. -->
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
          <div class="startup-illustration">
            <svg viewBox="0 0 200 200" fill="none">
              <circle cx="100" cy="72" r="32" stroke="var(--theme-color)" stroke-width="3" opacity="0.6"/>
              <path d="M45 165C45 130 68 108 100 108C132 108 155 130 155 165" stroke="var(--theme-color)" stroke-width="3" stroke-linecap="round" opacity="0.4"/>
              <circle cx="100" cy="100" r="85" stroke="var(--theme-color)" stroke-width="1.5" opacity="0.1" stroke-dasharray="6 6">
                <animateTransform attributeName="transform" type="rotate" from="0 100 100" to="-360 100 100" dur="30s" repeatCount="indefinite"/>
              </circle>
            </svg>
          </div>
          <h2 class="startup-title">{{ t('oobe.step2.title') }}</h2>
          <p class="startup-desc">{{ t('oobe.step2.desc') }}</p>
          <input
            v-model="userName"
            class="startup-input"
            :placeholder="t('oobe.step2.placeholder')"
            autofocus
            @keydown.enter="canNext && nextStep()"
          />
        </div>

        <!-- Step 3: Welcome -->
        <div v-else-if="step === 2" class="startup-step" key="welcome">
          <div class="startup-illustration">
            <svg viewBox="0 0 200 200" fill="none">
              <circle cx="100" cy="100" r="60" fill="var(--theme-color)" opacity="0.08"/>
              <path d="M70 105L90 125L135 75" stroke="var(--theme-color)" stroke-width="6" stroke-linecap="round" stroke-linejoin="round" opacity="0.7"/>
              <circle cx="100" cy="100" r="78" stroke="var(--theme-color)" stroke-width="1.5" opacity="0.15" stroke-dasharray="4 8">
                <animateTransform attributeName="transform" type="rotate" from="0 100 100" to="360 100 100" dur="20s" repeatCount="indefinite"/>
              </circle>
              <circle cx="55" cy="45" r="4" fill="var(--theme-color)" opacity="0.3"><animate attributeName="opacity" values="0.3;0.8;0.3" dur="2s" repeatCount="indefinite"/></circle>
              <circle cx="150" cy="50" r="3" fill="var(--theme-color)" opacity="0.4"><animate attributeName="opacity" values="0.4;0.9;0.4" dur="2.5s" repeatCount="indefinite"/></circle>
              <circle cx="145" cy="155" r="3.5" fill="var(--theme-color)" opacity="0.35"><animate attributeName="opacity" values="0.35;0.85;0.35" dur="1.8s" repeatCount="indefinite"/></circle>
              <circle cx="50" cy="150" r="3" fill="var(--theme-color)" opacity="0.3"><animate attributeName="opacity" values="0.3;0.7;0.3" dur="2.2s" repeatCount="indefinite"/></circle>
            </svg>
          </div>
          <h2 class="startup-title startup-title-big">{{ t('oobe.step3.title', { name: userName }) }}</h2>
          <p class="startup-desc">{{ t('oobe.step3.desc') }}</p>
        </div>

        <!-- Step 4: Settings -->
        <div v-else-if="step === 3" class="startup-step" key="settings">
          <div class="startup-illustration startup-illustration-small">
            <svg viewBox="0 0 200 200" fill="none">
              <circle cx="100" cy="100" r="28" stroke="var(--theme-color)" stroke-width="3" opacity="0.5"/>
              <circle cx="100" cy="100" r="12" fill="var(--theme-color)" opacity="0.2"/>
              <g stroke="var(--theme-color)" stroke-width="2.5" opacity="0.4">
                <line x1="100" y1="58" x2="100" y2="45" stroke-linecap="round"/>
                <line x1="100" y1="155" x2="100" y2="142" stroke-linecap="round"/>
                <line x1="58" y1="100" x2="45" y2="100" stroke-linecap="round"/>
                <line x1="155" y1="100" x2="142" y2="100" stroke-linecap="round"/>
                <line x1="70" y1="70" x2="61" y2="61" stroke-linecap="round"/>
                <line x1="130" y1="130" x2="139" y2="139" stroke-linecap="round"/>
                <line x1="130" y1="70" x2="139" y2="61" stroke-linecap="round"/>
                <line x1="70" y1="130" x2="61" y2="139" stroke-linecap="round"/>
              </g>
              <circle cx="100" cy="100" r="50" stroke="var(--theme-color)" stroke-width="1" opacity="0.1" stroke-dasharray="4 4">
                <animateTransform attributeName="transform" type="rotate" from="0 100 100" to="360 100 100" dur="15s" repeatCount="indefinite"/>
              </circle>
            </svg>
          </div>
          <h2 class="startup-title">{{ t('oobe.step4.title') }}</h2>
          <p class="startup-desc">{{ t('oobe.step4.desc') }}</p>

          <div class="startup-settings">
            <!-- Output Mode -->
            <div class="startup-setting-row">
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

            <!-- Close Action -->
            <div class="startup-setting-row">
              <div class="startup-setting-info">
                <span class="startup-setting-label">{{ t('oobe.step4.closeAction') }}</span>
                <span class="startup-setting-desc">{{ t('oobe.step4.closeActionDesc') }}</span>
              </div>
              <div class="segmented">
                <button class="seg-btn" :class="{ active: closeAction === 'ask' }" @click="closeAction = 'ask'">{{ t('oobe.step4.closeActionAsk') }}</button>
                <button class="seg-btn" :class="{ active: closeAction === 'close' }" @click="closeAction = 'close'">{{ t('oobe.step4.closeActionClose') }}</button>
                <button class="seg-btn" :class="{ active: closeAction === 'minimize' }" @click="closeAction = 'minimize'">{{ t('oobe.step4.closeActionMinimize') }}</button>
              </div>
            </div>

            <!-- Source Pack Handling (post-conversion) -->
            <div class="startup-setting-row">
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

        <!-- Step 5: Final Welcome -->
        <div v-else-if="step === 4" class="startup-step" key="done">
          <div class="startup-illustration">
            <svg viewBox="0 0 200 200" fill="none">
              <path d="M100 30L110 70H150L118 92L128 132L100 110L72 132L82 92L50 70H90L100 30Z" fill="var(--theme-color)" opacity="0.15"/>
              <path d="M100 50L107 75H133L112 90L119 115L100 100L81 115L88 90L67 75H93L100 50Z" stroke="var(--theme-color)" stroke-width="2" opacity="0.5"/>
              <circle cx="100" cy="100" r="85" stroke="var(--theme-color)" stroke-width="1" opacity="0.1"/>
              <circle cx="50" cy="40" r="3" fill="var(--theme-color)" opacity="0.4"><animate attributeName="opacity" values="0.4;1;0.4" dur="1.5s" repeatCount="indefinite"/></circle>
              <circle cx="155" cy="35" r="2.5" fill="var(--theme-color)" opacity="0.5"><animate attributeName="opacity" values="0.5;1;0.5" dur="1.8s" repeatCount="indefinite"/></circle>
              <circle cx="160" cy="160" r="3" fill="var(--theme-color)" opacity="0.3"><animate attributeName="opacity" values="0.3;0.9;0.3" dur="2s" repeatCount="indefinite"/></circle>
              <circle cx="40" cy="165" r="2" fill="var(--theme-color)" opacity="0.45"><animate attributeName="opacity" values="0.45;1;0.45" dur="1.6s" repeatCount="indefinite"/></circle>
              <circle cx="100" cy="25" r="2.5" fill="var(--theme-color)" opacity="0.5"><animate attributeName="opacity" values="0.5;1;0.5" dur="2.2s" repeatCount="indefinite"/></circle>
            </svg>
          </div>
          <h2 class="startup-title startup-title-big">{{ t('oobe.step5.title', { name: userName }) }}</h2>
          <p class="startup-desc">{{ t('oobe.step5.desc') }}</p>
        </div>
      </Transition>

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
              <li v-if="backupInfo.summary.close_action">
                <span class="label">{{ t('settings.closeAction.label') }}</span>
                <span class="value">{{ t(`settings.closeAction.${backupInfo.summary.close_action}`, backupInfo.summary.close_action) }}</span>
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

      <!-- Navigation -->
      <div class="startup-nav">
        <button v-if="step > 0 && step < totalSteps - 1" class="startup-btn startup-btn-ghost" @click="prevStep">
          <i class="ri-arrow-left-s-line"></i>
          {{ t('oobe.back') }}
        </button>
        <div v-else class="startup-nav-spacer"></div>

        <div class="startup-dots">
          <span v-for="i in totalSteps" :key="i" class="startup-dot" :class="{ active: i - 1 === step, done: i - 1 < step }"></span>
        </div>

        <button
          v-if="step >= 1 && step < totalSteps - 1"
          class="startup-btn startup-btn-primary"
          :disabled="step === 1 && !canNext"
          @click="nextStep"
        >
          {{ t('oobe.next') }}
          <i class="ri-arrow-right-s-line"></i>
        </button>
        <button
          v-else-if="step === totalSteps - 1"
          class="startup-btn startup-btn-primary startup-btn-finish"
          @click="finish"
        >
          {{ t('oobe.getStarted') }}
          <i class="ri-rocket-2-line"></i>
        </button>
        <div v-else class="startup-nav-spacer"></div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.startup-root {
  position: fixed;
  inset: 0;
  z-index: 10000;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(160deg, #f6f8fd 0%, #eef2fb 45%, #e7ecf9 100%);
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", "PingFang SC", "Microsoft YaHei", Roboto, "Helvetica Neue", Arial, sans-serif;
  overflow: hidden;
}

/* Layered aurora — soft, wide colour washes that slowly drift. */
.startup-aurora {
  position: absolute;
  inset: -30%;
  background:
    radial-gradient(1100px 600px at 22% 20%, color-mix(in srgb, var(--theme-color) 18%, transparent), transparent 62%),
    radial-gradient(900px 520px at 78% 25%, color-mix(in srgb, var(--theme-color) 14%, transparent), transparent 60%),
    radial-gradient(1000px 600px at 60% 85%, color-mix(in srgb, var(--theme-color) 12%, transparent), transparent 62%),
    radial-gradient(700px 450px at 35% 70%, color-mix(in srgb, #818cf8 10%, transparent), transparent 60%);
  opacity: 0.75;
  filter: blur(70px);
  animation: startup-aurora 24s ease-in-out infinite;
  pointer-events: none;
}

@keyframes startup-aurora {
  0%, 100% { transform: translate3d(0, 0, 0) scale(1); }
  33% { transform: translate3d(40px, -25px, 0) scale(1.04); }
  66% { transform: translate3d(-30px, 20px, 0) scale(0.98); }
}

/* Floating colour blobs for depth. */
.startup-blob {
  position: absolute;
  border-radius: 50%;
  filter: blur(50px);
  opacity: 0.5;
  pointer-events: none;
  animation: blob-drift 18s ease-in-out infinite;
}
.blob-1 {
  width: 420px; height: 420px;
  top: -120px; right: -80px;
  background: color-mix(in srgb, var(--theme-color) 22%, transparent);
}
.blob-2 {
  width: 360px; height: 360px;
  bottom: -100px; left: -60px;
  background: color-mix(in srgb, #6366f1 18%, transparent);
  animation-delay: -6s;
}
.blob-3 {
  width: 240px; height: 240px;
  top: 40%; left: 12%;
  background: color-mix(in srgb, #22d3ee 14%, transparent);
  animation-delay: -12s;
}

@keyframes blob-drift {
  0%, 100% { transform: translate3d(0, 0, 0); }
  50% { transform: translate3d(30px, -30px, 0); }
}

/* Faint dot-grid overlay for texture. */
.startup-grid {
  position: absolute;
  inset: 0;
  background-image: radial-gradient(rgba(0, 0, 0, 0.05) 1px, transparent 1px);
  background-size: 26px 26px;
  opacity: 0.35;
  pointer-events: none;
}

/* Brand */
.startup-brand {
  position: absolute;
  top: 28px;
  left: 32px;
  display: flex;
  align-items: center;
  gap: 10px;
  z-index: 1;
}

.startup-logo {
  width: 56px;
  height: 56px;
  filter: drop-shadow(0 4px 10px rgba(0, 0, 0, 0.12));
}

.startup-brand-name {
  font-size: 16px;
  font-weight: 800;
  color: #1a1a2e;
  letter-spacing: -0.5px;
}

/* Card — premium glassmorphism with a top highlight strip. */
.startup-card {
  position: relative;
  width: 520px;
  max-width: 92vw;
  max-height: 88vh;
  background: linear-gradient(165deg, rgba(255, 255, 255, 0.92) 0%, rgba(255, 255, 255, 0.72) 100%);
  backdrop-filter: blur(40px) saturate(1.5);
  -webkit-backdrop-filter: blur(40px) saturate(1.5);
  border-radius: 28px;
  border: 1px solid rgba(255, 255, 255, 0.85);
  box-shadow:
    0 24px 70px rgba(15, 23, 42, 0.10),
    0 4px 16px rgba(15, 23, 42, 0.06),
    inset 0 1px 0 rgba(255, 255, 255, 0.9);
  overflow: hidden;
  display: flex;
  flex-direction: column;
  animation: card-in 0.6s cubic-bezier(0.22, 1, 0.36, 1) both;
}

@keyframes card-in {
  from { opacity: 0; transform: translateY(24px) scale(0.98); }
  to   { opacity: 1; transform: translateY(0) scale(1); }
}

/* Top highlight strip (theme gradient). */
.startup-card::before {
  content: "";
  position: absolute;
  top: 0; left: 0; right: 0;
  height: 3px;
  background: linear-gradient(90deg,
    color-mix(in srgb, var(--theme-color) 85%, #fff),
    color-mix(in srgb, var(--theme-color) 45%, #fff),
    color-mix(in srgb, #6366f1 80%, #fff));
  opacity: 0.9;
}

/* Progress — gradient bar with glow. */
.startup-progress {
  height: 3px;
  background: rgba(0, 0, 0, 0.06);
  flex-shrink: 0;
}

.startup-progress-bar {
  height: 100%;
  background: linear-gradient(90deg, var(--theme-color), color-mix(in srgb, var(--theme-color) 55%, #fff));
  border-radius: 0 3px 3px 0;
  box-shadow: 0 0 10px color-mix(in srgb, var(--theme-color) 45%, transparent);
  transition: width 0.5s cubic-bezier(0.4, 0, 0.2, 1);
}

/* Step container */
.startup-step {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 40px 40px 20px;
  overflow-y: auto;
  min-height: 0;
}

/* Illustration */
.startup-illustration {
  width: 160px;
  height: 160px;
  margin-bottom: 24px;
  flex-shrink: 0;
}

.startup-illustration-small {
  width: 110px;
  height: 110px;
  margin-bottom: 16px;
}

.startup-illustration svg {
  width: 100%;
  height: 100%;
}

/* Text */
.startup-title {
  font-size: 23px;
  font-weight: 800;
  color: #111827;
  margin: 0 0 8px;
  text-align: center;
  letter-spacing: -0.4px;
  background: linear-gradient(120deg, #111827, color-mix(in srgb, var(--theme-color) 70%, #111827));
  -webkit-background-clip: text;
  background-clip: text;
  -webkit-text-fill-color: transparent;
}

.startup-title-big {
  font-size: 30px;
  letter-spacing: -0.6px;
}

.startup-desc {
  font-size: 14px;
  color: #6b7280;
  margin: 0 0 28px;
  text-align: center;
  line-height: 1.7;
  max-width: 380px;
}

/* Language cards */
.lang-cards {
  display: flex;
  gap: 16px;
  width: 100%;
  max-width: 360px;
}

.lang-card {
  position: relative;
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 10px;
  padding: 22px 16px 18px;
  background: rgba(255, 255, 255, 0.62);
  border: 2px solid rgba(0, 0, 0, 0.07);
  border-radius: 18px;
  cursor: pointer;
  transition: all 0.28s cubic-bezier(0.22, 1, 0.36, 1);
  font-family: inherit;
  overflow: hidden;
}

.lang-card:hover {
  border-color: color-mix(in srgb, var(--theme-color) 45%, transparent);
  background: rgba(255, 255, 255, 0.92);
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

/* Check badge — scales in when selected. */
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

/* Input */
.startup-input {
  width: 100%;
  max-width: 320px;
  padding: 14px 20px;
  font-size: 16px;
  font-family: inherit;
  border: 2px solid rgba(0, 0, 0, 0.1);
  border-radius: 14px;
  background: rgba(255, 255, 255, 0.7);
  color: #1a1a2e;
  outline: none;
  transition: all 0.25s ease;
  text-align: center;
}

.startup-input:focus {
  border-color: var(--theme-color);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--theme-color) 15%, transparent);
  background: #fff;
}

.startup-input::placeholder {
  color: #9ca3af;
}

/* Settings rows — subtle inset cards */
.startup-settings {
  width: 100%;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.startup-setting-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 18px;
  background: rgba(255, 255, 255, 0.55);
  border: 1px solid rgba(0, 0, 0, 0.05);
  border-radius: 16px;
  gap: 16px;
  transition: background 0.2s ease, border-color 0.2s ease, box-shadow 0.2s ease;
}
.startup-setting-row:hover {
  background: rgba(255, 255, 255, 0.85);
  border-color: color-mix(in srgb, var(--theme-color) 22%, transparent);
  box-shadow: 0 3px 12px rgba(15, 23, 42, 0.04);
}

.startup-setting-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
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

/* Segmented control */
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

/* Toggle switch */
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

/* Navigation */
.startup-nav {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 32px 28px;
  flex-shrink: 0;
}

.startup-nav-spacer {
  width: 80px;
}

.startup-dots {
  display: flex;
  gap: 8px;
}

.startup-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: rgba(0, 0, 0, 0.12);
  transition: all 0.35s cubic-bezier(0.22, 1, 0.36, 1);
}

.startup-dot.active {
  background: linear-gradient(90deg, var(--theme-color), color-mix(in srgb, var(--theme-color) 55%, #fff));
  width: 26px;
  border-radius: 4px;
  box-shadow: 0 0 8px color-mix(in srgb, var(--theme-color) 40%, transparent);
}

.startup-dot.done {
  background: color-mix(in srgb, var(--theme-color) 45%, transparent);
}

/* Buttons */
.startup-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 10px 24px;
  font-size: 14px;
  font-weight: 600;
  font-family: inherit;
  border: none;
  border-radius: 12px;
  cursor: pointer;
  transition: all 0.25s ease;
}

.startup-btn-primary {
  background: linear-gradient(135deg, var(--theme-color), color-mix(in srgb, var(--theme-color) 72%, #4338ca));
  color: #fff;
  box-shadow: 0 4px 14px color-mix(in srgb, var(--theme-color) 35%, transparent);
}

.startup-btn-primary:hover:not(:disabled) {
  transform: translateY(-2px);
  box-shadow: 0 8px 22px color-mix(in srgb, var(--theme-color) 42%, transparent);
}

.startup-btn-primary:active:not(:disabled) {
  transform: translateY(0);
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
}

.startup-btn-finish {
  padding: 12px 28px;
  font-size: 15px;
}

/* Transitions — smoother 3D-feel step change */
.startup-slide-enter-active,
.startup-slide-leave-active {
  transition: all 0.4s cubic-bezier(0.22, 1, 0.36, 1);
}

.startup-slide-enter-from {
  opacity: 0;
  transform: translateX(46px) scale(0.99);
}

.startup-slide-leave-to {
  opacity: 0;
  transform: translateX(-46px) scale(0.99);
}

/* ── Backup import (step 0 CTA + preview dialog) ─────────────────
   The CTA is a subtle ghost-style link beneath the language cards.
   The preview dialog overlays the whole startup screen with a
   centred card so the user can sanity-check what they're about to
   restore before confirming. */
.startup-restore-btn {
  margin-top: 22px;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  background: transparent;
  border: 1px dashed rgba(0, 0, 0, 0.15);
  border-radius: 10px;
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
</style>
