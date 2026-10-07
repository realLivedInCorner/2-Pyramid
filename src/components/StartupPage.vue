<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { useLanguage, type SupportedLocale } from '../composables/useLanguage'
import { useAppInfo } from '../composables/useAppInfo'
import StageBackdrop from './shell/StageBackdrop.vue'
import { useTheme, revealInk } from '../composables/useTheme'

const { t, locale } = useI18n()
const { setLanguage } = useLanguage()
// 品牌块的版本行，与首页同一组数据
const { version: appVersion, isBeta: appIsBeta } = useAppInfo()
// 右上角的深浅切换 —— OOBE 是用户第一次看到这个软件，深浅应当在这里就能定
const { isDark, toggleTheme } = useTheme()

/** 步骤计数补零成 01 / 03 的形式，与首页的模块计数同构 */
const pad = (n: number) => String(n).padStart(2, '0')

const emit = defineEmits<{ complete: [] }>()

const step = ref(0)
/**
 * 三步。
 *
 * 原本是五步，其中「欢迎」和「完成」是两张只换了图标的卡片，中间隔着一个
 * 十几个选项的设置页 —— 用卡片把步骤数凑到 5。它们不承载信息：删掉之后
 * 「欢迎」由第 1 步的语言问候承担，「完成」由第 3 步的「开始使用」按钮承担。
 */
const totalSteps = 3

// Step data
const selectedLang = ref<SupportedLocale>(locale.value as SupportedLocale)
const userName = ref('')
const outputMode = ref<'follow' | 'fixed'>('follow')
// 下面这些虽然不再出现在 OOBE 界面上，仍要保留默认值：
// finish() 会把它们一起写进配置，删掉会让新用户的配置少几个字段。
const notificationEnabled = ref(true)
const notificationMode = ref<'system' | 'app' | 'both'>('app')
// Post-conversion source-pack handling + auto-open output folder.
// Default `ask` is the safest choice for first-time users: the app
// will prompt before deleting any of their packs.
const sourceHandling = ref<'ask' | 'delete' | 'keep'>('ask')
const openOutputAfterConvert = ref<boolean>(true)
const updateChannel = ref<'master' | 'unstable' | 'both'>('master')
const autoCheckUpdate = ref(true)
const conversionThreads = ref(2)
const namingTemplate = ref('[Ver][Name]')
// namingPreview 随命名模板的界面一起删了 —— 那个输入框现在只在设置页里，
// 预览也该跟着走，留在 OOBE 就是没人读的死代码。

// 每步的标题/描述（显示在右侧内容区顶部）
const stepMeta = computed(() => {
  // 三步的文案复用原来的 oobe.step1 / step2 / step4 键。
  // step3 与 step5 那两组已经用不到，但**留在语言文件里** ——
  // 删掉会让「这几个 key 是给谁用的」变成悬案，留着只是几行字符串。
  const metas = [
    { title: t('oobe.step1.title'), desc: t('oobe.step1.desc') },
    { title: t('oobe.step2.title'), desc: t('oobe.step2.desc') },
    { title: t('oobe.step4.title'), desc: t('oobe.step4.desc') },
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

/*
 * 步骤切换走墨水揭示 —— 与主题切换**同一条路径**（revealInk）。
 *
 * 此前是 <Transition name="startup-slide"> 的幻灯片，那是第二套动效语言。
 * 用户在首页学到「切换长什么样」，在 OOBE 里必须同样成立。
 *
 * 遮罩挂在 ::view-transition-new(root) 上，所以这里不需要传任何样式；
 * 方向（上切 / 下切）也由 revealInk 内部的随机化决定，和主题切换共享
 * 同一个交替状态，不会两套各转各的。
 */
function nextStep() {
  if (step.value >= totalSteps - 1) return
  revealInk(() => { step.value++ })
}

function prevStep() {
  if (step.value <= 0) return
  revealInk(() => { step.value-- })
}

function selectLang(lang: SupportedLocale) {
  // 选中态与切步要落在同一次快照里，否则会看到两次揭示
  revealInk(() => {
    selectedLang.value = lang
    setLanguage(lang)
    if (step.value < totalSteps - 1) step.value++
  })
}

function quitApp() {
  // 退出整个应用（与主界面的 Ctrl+Shift+Q 同路径）
  invoke('force_quit').catch((e) => {
    console.error('[StartupPage] force_quit failed:', e)
  })
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
        conversionThreads: conversionThreads.value,
        outputNaming: namingTemplate.value,
        updateChannel: updateChannel.value,
        autoCheckUpdate: autoCheckUpdate.value,
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
    <!--
      固定阶梯背景，与首页同一张。
      「背景是固定的舞台，不参与叙事」是首页的第二条规则 —— 它没有动画、
      不可交互，也不随步骤变化。
    -->
    <StageBackdrop />

    <!-- ═══ 顶部：品牌（左） + 计数与主题（右） ═══ -->
    <header class="startup-top">
      <div class="top-brand">
        <!--
          描边必须用 vector-effect="non-scaling-stroke"。
          这个 viewBox 是 680×680，而元素只显示 40×40，缩放比 0.0588 ——
          原来的 stroke-width="7" 实际渲染出来是 0.41px，内线 5 只有 0.29px，
          亚像素，无论什么颜色都看不见。non-scaling-stroke 让 stroke-width
          以 CSS 像素表达，不再随 viewBox 缩放。

          **它必须写在每个 <line> 上，不能写在 <g> 上** —— vector-effect 不是
          继承属性，挂在 <g> 上等于没挂（第一版就是这么白改的）。
        -->
        <svg class="startup-logo" viewBox="0 0 680 680" fill="none" xmlns="http://www.w3.org/2000/svg">
          <g stroke="currentColor" stroke-linecap="round">
            <line x1="260" y1="520" x2="200" y2="390" stroke-width="1.6" vector-effect="non-scaling-stroke"/>
            <line x1="260" y1="520" x2="500" y2="190" stroke-width="1.6" vector-effect="non-scaling-stroke"/>
            <line x1="200" y1="390" x2="500" y2="190" stroke-width="1.6" vector-effect="non-scaling-stroke"/>
            <line x1="260" y1="520" x2="374" y2="274" stroke-width="1.15" vector-effect="non-scaling-stroke"/>
          </g>
        </svg>
        <div class="top-brand-text">
          <span class="top-brand-name">2-Pyramid</span>
          <span class="top-brand-tagline">{{ t('oobe.asideTagline') }}</span>
          <!-- 版本行与首页同构：logo → 名称 → 标语 → 版本 · 引擎 -->
          <span class="top-brand-meta">
            v{{ appVersion }}<template v-if="appIsBeta"> Beta</template>
            <i aria-hidden="true"></i>
            {{ t('home.engineReady') }}
          </span>
        </div>
      </div>

      <div class="panel-aside">
        <button
          class="theme-toggle"
          :class="{ dark: isDark }"
          :aria-label="isDark ? t('home.themeToLight') : t('home.themeToDark')"
          :title="isDark ? t('home.themeToLight') : t('home.themeToDark')"
          @click="toggleTheme"
        >
          <i class="ri-sun-line tt-sun" aria-hidden="true"></i>
          <i class="ri-moon-line tt-moon" aria-hidden="true"></i>
        </button>
      </div>
    </header>

    <!-- ═══ 内容区 ═══ -->
    <main class="startup-panel">
      <!--
        步骤计数，与首页的模块计数同构：MONITOR / SELECT + 01 / 03。
        取代了原来的「五个圆点 + 连线」—— 那是另一套进度语言，而首页
        用计数表达「第几件 / 共几件」，进度条式的圆点在这里是多余的。
      -->
      <div class="panel-kicker">
        <span class="kicker-label">ONBOARDING / STEP</span>
        <span class="kicker-count">
          <b>{{ pad(step + 1) }}</b><i>/</i>{{ pad(totalSteps) }}
        </span>
      </div>

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
                  <svg viewBox="0 0 40 30" fill="none"><rect width="40" height="30" rx="4" fill="#012169"/><path d="M0 0L40 30M40 0L0 30" stroke="var(--surface)" stroke-width="5"/><path d="M0 0L40 30M40 0L0 30" stroke="var(--danger)" stroke-width="3"/><path d="M20 0V30M0 15H40" stroke="var(--surface)" stroke-width="9"/><path d="M20 0V30M0 15H40" stroke="var(--danger)" stroke-width="6"/></svg>
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

          <!-- Step 4: Settings -->
          <div v-else class="startup-step startup-step-wide" key="settings">
            <h2 class="step-title">{{ stepMeta.title }}</h2>
            <p class="step-desc">{{ stepMeta.desc }}</p>
            <div class="startup-settings">
              <div class="startup-settings-grid">
                <!-- 转换 -->
                <div class="startup-group-card">
                  <h3 class="startup-group-title">{{ t('oobe.step4.groups.convert') }}</h3>
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
                </div>
              </div>
            </div>
          </div>
        </Transition>
      </div>

      <!-- 底部药丸导航 -->
      <div class="panel-nav">
        <div class="nav-pill">
          <button
            class="startup-btn startup-btn-quit"
            :title="t('oobe.quit')"
            :aria-label="t('oobe.quit')"
            @click="quitApp"
          >
            <i class="ri-logout-box-r-line"></i>
          </button>
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
  /* 底色交给 StageBackdrop，这里必须透明 —— 否则会盖住阶梯背景。
     原本是 background: var(--surface-2)，那正是阶梯被埋掉的原因。 */
  background: transparent;
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", "PingFang SC", "Microsoft YaHei", Roboto, "Helvetica Neue", Arial, sans-serif;
  overflow: hidden;
}

/* 进入动画：品牌、内容、导航依次上浮淡入（工厂重置后平滑过渡） */
.startup-top {
  animation: oobe-rise 0.55s cubic-bezier(0.22, 1, 0.36, 1) 0.08s backwards;
}

.panel-content {
  animation: oobe-rise 0.55s cubic-bezier(0.22, 1, 0.36, 1) 0.18s backwards;
}

.panel-nav {
  animation: oobe-rise 0.55s cubic-bezier(0.22, 1, 0.36, 1) 0.3s backwards;
}

@keyframes oobe-rise {
  from {
    opacity: 0;
    transform: translateY(18px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
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
  color: var(--accent);
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
  /* 600 是上限，见 dialog-spec.md。此前 800 属于「用字重拉层级」 */
  font-weight: 600;
  letter-spacing: -0.4px;
  color: var(--ink);
  line-height: 1.2;
}

.top-brand-tagline {
  font-size: 11.5px;
  color: var(--ink-4);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* 版本行：与首页 .brand-meta 同构 —— v版本 · 引擎状态，中间一个分隔点 */
.top-brand-meta {
  display: flex;
  align-items: center;
  gap: 7px;
  font-size: 11px;
  letter-spacing: 0.04em;
  color: var(--ink-4);
  margin-top: 2px;
}
.top-brand-meta i {
  width: 3px;
  height: 3px;
  border-radius: 50%;
  background: var(--ink-4);
  flex-shrink: 0;
}

/* ── 右上角：主题切换 ── */
.panel-aside {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-shrink: 0;
}
/*
 * 从首页整段搬来，不动数值 —— 两处的按钮必须是同一个东西，
 * 否则用户在 OOBE 和首页会看到两个手感不同的开关。
 */
.theme-toggle {
  position: relative;
  width: 26px;
  height: 26px;
  flex-shrink: 0;
  padding: 0;
  border: none;
  background: transparent;
  border-radius: 6px;
  cursor: pointer;
  color: var(--ink-3);
  transition: background 0.18s ease, color 0.18s ease;
}
.theme-toggle:hover { background: rgba(var(--warm-rgb), 0.09); color: var(--ink); }
.theme-toggle i {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 17px;
  transition:
    transform 420ms var(--ease-spring, cubic-bezier(0.34, 1.56, 0.64, 1)),
    opacity 220ms var(--ease-out, cubic-bezier(0.22, 1, 0.36, 1));
}
.tt-sun  { transform: rotate(0deg)   scale(1);   opacity: 1; }
.tt-moon { transform: rotate(-90deg) scale(0.4); opacity: 0; }
.theme-toggle.dark .tt-sun  { transform: rotate(90deg) scale(0.4); opacity: 0; }
.theme-toggle.dark .tt-moon { transform: rotate(0deg) scale(1);    opacity: 1; }

/* ── 步骤计数 ── */
/*
 * 与首页 .subject 的 counter 同构：等宽数字，当前步用主墨色、总数用最弱灰。
 * 刻意不做成进度条 —— 圆点和连线是另一套语言，而这里表达的是「第几件 / 共几件」。
 */
.panel-kicker {
  display: flex;
  align-items: baseline;
  gap: 14px;
  padding: 0 36px 6px;
  flex-shrink: 0;
}
.kicker-label {
  font-size: 10px;
  font-weight: 400;
  letter-spacing: 0.22em;
  color: var(--ink-4);
  text-transform: uppercase;
}
.kicker-count {
  display: inline-flex;
  align-items: baseline;
  gap: 3px;
  font-size: 13px;
  font-variant-numeric: tabular-nums;
  letter-spacing: 0.04em;
  color: var(--ink-4);
}
.kicker-count b {
  font-size: 17px;
  font-weight: 600;
  color: var(--ink);
}
.kicker-count i { font-style: normal; opacity: 0.6; }

/* ═══ 主内容区 ═══ */
.startup-panel {
  flex: 1 1 auto;
  display: flex;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
}

/*
 * 内容必须自己建立层叠上下文。
 *
 * StageBackdrop 是 `position: absolute`，而这两块是**非定位元素** ——
 * 定位元素绘制在非定位内容之上，与 DOM 顺序无关。所以背景把品牌块、
 * 计数和步骤内容整个盖住了（实测这些元素的 rect / opacity / color 全部
 * 正确，就是一个像素都看不见）。
 *
 * 首页没有这个问题：它的四个角块自己就是 position: absolute，
 * 和背景同层，靠 DOM 顺序决定前后。
 */
.startup-top,
.startup-panel {
  position: relative;
  z-index: 1;
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
  background: rgba(var(--ink-rgb), 0.05);
  color: var(--ink-4);
  transition: all 0.3s cubic-bezier(0.22, 1, 0.36, 1);
}

.startup-step-dot i { font-size: 14px; }

.startup-step-dot.active {
  background: var(--accent);
  color: var(--surface);
  box-shadow: 0 2px 10px color-mix(in srgb, var(--accent) 40%, transparent);
}

.startup-step-dot.done {
  background: color-mix(in srgb, var(--accent) 18%, transparent);
  color: var(--accent);
}

.startup-step-line {
  width: 16px;
  height: 2px;
  border-radius: 1px;
  background: rgba(var(--ink-rgb), 0.08);
  transition: background 0.3s ease;
}

.startup-step-line.done {
  background: color-mix(in srgb, var(--accent) 45%, transparent);
}

/*
 * 内容区左对齐，**不居中**。
 *
 * 此前是 `align-items: safe center` + `justify-content: center`，配合
 * 标题与描述的 `text-align: center`，整个 OOBE 是居中的 —— 与首页的
 * 第一条硬约束直接冲突：「我不希望看到任何居中元素，本身就抢占焦点」。
 *
 * 左内边距取 36px 与顶部品牌块对齐，两处的左边缘因此在同一条竖线上。
 */
.panel-content {
  flex: 1;
  display: flex;
  align-items: flex-start;
  justify-content: flex-start;
  padding: 24px 36px 20px;
  min-height: 0;
  overflow-y: auto;
}

.startup-step {
  width: 100%;
  max-width: 500px;
  display: flex;
  flex-direction: column;
  align-items: flex-start;
}

/* 设置步骤内容更多：放宽容器，避免卡片被挤成单字竖排 */
.startup-step-wide {
  max-width: min(960px, 100%);
}

.step-title {
  margin: 0 0 8px;
  font-size: 24px;
  /* 600 是上限，见 dialog-spec.md。此前 800 属于「用字重拉层级」，
     而这里的层级本就该由字号和位置表达。 */
  font-weight: 600;
  letter-spacing: -0.4px;
  color: var(--ink);
  text-align: left;
}

.step-desc {
  margin: 0 0 26px;
  font-size: 13.5px;
  line-height: 1.7;
  color: var(--ink-3);
  text-align: left;
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
  background: var(--surface);
  border: 2px solid rgba(var(--ink-rgb), 0.08);
  border-radius: 18px;
  cursor: pointer;
  transition: all 0.28s cubic-bezier(0.22, 1, 0.36, 1);
  font-family: inherit;
  overflow: hidden;
}

.lang-card:hover {
  border-color: color-mix(in srgb, var(--accent) 45%, transparent);
  transform: translateY(-3px);
  box-shadow: 0 10px 26px rgba(var(--ink-rgb), 0.08);
}

.lang-card.active {
  border-color: var(--accent);
  /*
   * 渐变端必须是 var(--surface) 而不是写死的 white。
   * `color-mix(in srgb, var(--accent) 7%, white)` 是「7% 强调色 + 93% 白」，
   * 浅色下没问题（--surface 本来就是 #ffffff，两者等价），深色下就是一块
   * 近白的板 —— 整页最亮的东西。这和阶梯、--panel-* 是同一个 bug 家族：
   * 把「提亮」的方向写死在只对浅色成立的值上。
   */
  background: linear-gradient(
    160deg,
    color-mix(in srgb, var(--accent) 7%, var(--surface)) 0%,
    var(--surface) 100%
  );
  box-shadow:
    0 0 0 3px color-mix(in srgb, var(--accent) 18%, transparent),
    0 8px 24px color-mix(in srgb, var(--accent) 14%, transparent);
}

.lang-check {
  position: absolute;
  top: 10px;
  right: 10px;
  width: 20px;
  height: 20px;
  border-radius: 50%;
  background: var(--accent);
  color: var(--surface);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 12px;
  transform: scale(0);
  opacity: 0;
  transition: transform 0.3s cubic-bezier(0.34, 1.56, 0.64, 1), opacity 0.2s;
  box-shadow: 0 2px 8px color-mix(in srgb, var(--accent) 40%, transparent);
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
  box-shadow: 0 2px 6px rgba(var(--ink-rgb), 0.12);
  transition: transform 0.28s ease;
}
.lang-card:hover .lang-flag svg {
  transform: scale(1.06);
}

.lang-name {
  font-size: 14px;
  font-weight: 600;
  color: var(--ink-2);
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
  color: var(--ink-4);
  pointer-events: none;
}

.startup-input {
  width: 100%;
  padding: 15px 20px 15px 46px;
  font-size: 16px;
  font-family: inherit;
  border: 2px solid rgba(var(--ink-rgb), 0.12);
  border-radius: 14px;
  background: var(--surface);
  color: var(--ink);
  outline: none;
  transition: all 0.25s ease;
}

.startup-input:focus {
  border-color: var(--accent);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 15%, transparent);
}

.startup-input::placeholder {
  color: var(--ink-4);
}

.field-hint {
  margin: 12px 0 0;
  font-size: 12.5px;
  color: var(--ink-4);
  text-align: center;
}

/* 欢迎/完成卡片 */
.welcome-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 14px;
  padding: 32px 28px;
  background: var(--surface);
  border: 1px solid rgba(var(--ink-rgb), 0.05);
  border-radius: 18px;
  box-shadow: 0 4px 20px rgba(var(--ink-rgb), 0.05);
  text-align: center;
}

.welcome-card i {
  font-size: 40px;
  color: var(--accent);
}

.welcome-card p {
  margin: 0;
  font-size: 14px;
  line-height: 1.7;
  color: var(--ink-2);
}

/* 设置行 —— 与设置页同款卡片 */
.startup-settings {
  width: 100%;
}

.startup-settings-grid {
  display: grid;
  /* 列宽不足 380px 就自动变单列，避免卡片过窄导致正文逐字换行 */
  grid-template-columns: repeat(auto-fit, minmax(380px, 1fr));
  gap: 14px;
  align-items: start;
}

.startup-group-card {
  /* 卡片表面（背景/边框/圆角/阴影）由 App.vue 全局 --ui-* 令牌统一提供，
     这里只保留布局与裁剪，避免与全局规则打架 */
  border-radius: 20px;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  min-width: 0;
}

/* 宽版设置步骤：整体更紧凑，四张卡同屏无需滚动 */
.startup-step-wide .step-desc {
  margin-bottom: 18px;
}
.startup-step-wide .startup-setting-row {
  padding: 10px 16px;
}
.startup-step-wide .startup-settings-grid {
  gap: 12px;
}

.startup-group-title {
  margin: 0;
  padding: 12px 16px 6px;
  font-size: 12px;
  font-weight: 600;
  letter-spacing: 0.02em;
  color: var(--ink-3);
}

.startup-setting-row {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 16px;
  border-top: 1px solid rgba(var(--ink-rgb), 0.03);
  transition: background 0.2s ease;
  min-width: 0;
}
.startup-setting-row:hover {
  background: rgba(var(--ink-rgb), 0.03);
}

/* 行内控件不参与压缩（否则分段按钮会被挤掉、正文被压成竖排） */
.startup-setting-row > .segmented,
.startup-setting-row > .switch {
  flex: 0 0 auto;
}

@media (max-width: 1040px) {
  .startup-settings-grid {
    grid-template-columns: 1fr;
  }
}

/* 窄窗口：控件换行到正文下方，仍然可读 */
@media (max-width: 620px) {
  .startup-setting-row {
    flex-wrap: wrap;
  }
  .startup-setting-row > .segmented,
  .startup-setting-row > .switch {
    margin-left: 48px;
  }
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
  color: var(--accent);
  background: color-mix(in srgb, var(--accent) 12%, transparent);
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
  color: var(--ink);
}

.startup-setting-desc {
  font-size: 12px;
  color: var(--ink-4);
}

/* 命名模板行内编辑器 */
.naming-row-inline {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 4px 16px 14px;
}
.naming-row-plain {
  border-top: none;
  padding-bottom: 4px;
}

.naming-input {
  width: 100%;
  padding: 9px 12px;
  font-size: 13px;
  font-family: inherit;
  border: 1.5px solid rgba(var(--ink-rgb), 0.12);
  border-radius: 10px;
  background: rgba(var(--ink-rgb), 0.05);
  color: var(--ink);
  outline: none;
}

.naming-input:focus {
  border-color: var(--accent);
  background: var(--surface);
}

.naming-preview-inline {
  font-size: 12px;
  color: var(--ink-3);
  word-break: break-all;
}

.naming-preview-inline b {
  color: var(--ink);
}

/* 分段按钮 */
.segmented {
  display: flex;
  background: rgba(var(--ink-rgb), 0.05);
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
  color: var(--ink-3);
  cursor: pointer;
  transition: all 0.2s ease;
  white-space: nowrap;
}

.seg-btn.active {
  background: var(--surface);
  color: var(--accent);
  box-shadow: 0 1px 4px rgba(var(--ink-rgb), 0.08);
}

.seg-btn:hover:not(.active) {
  color: var(--ink-2);
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
  background: var(--surface-6);
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
  background: var(--surface);
  border-radius: 50%;
  transition: all 0.3s ease;
  box-shadow: 0 1px 3px rgba(var(--ink-rgb), 0.18);
}

.switch input:checked + .slider {
  background: var(--accent);
}

.switch input:checked + .slider::before {
  transform: translateX(20px);
}

/* ═══ 底部药丸导航 ═══ */
/*
 * 靠右，与首页右下的动作区同位置。
 * 此前是 `justify-content: center` —— 底部药丸水平居中，同样是居中元素。
 * 右内边距取 36px，与顶部品牌块、内容区的左边缘对称。
 */
.panel-nav {
  display: flex;
  justify-content: flex-end;
  padding: 8px 36px 26px;
  flex-shrink: 0;
}

.nav-pill {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px;
  background: var(--surface);
  border: 1px solid rgba(var(--ink-rgb), 0.05);
  border-radius: 999px;
  box-shadow:
    0 10px 34px rgba(var(--ink-rgb), 0.12),
    0 2px 8px rgba(var(--ink-rgb), 0.05);
  transition: transform 0.28s cubic-bezier(0.22, 1, 0.36, 1), box-shadow 0.28s ease;
}

.nav-pill:hover {
  transform: translateY(-3px);
  box-shadow:
    0 16px 40px rgba(var(--ink-rgb), 0.12),
    0 4px 12px rgba(var(--ink-rgb), 0.08);
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
  transition: transform 0.2s cubic-bezier(0.22, 1, 0.36, 1), background 0.18s ease, color 0.18s ease, box-shadow 0.18s ease, opacity 0.18s ease;
}

.startup-btn-primary {
  background: var(--accent);
  color: var(--surface);
  box-shadow: 0 4px 14px color-mix(in srgb, var(--accent) 35%, transparent);
}

.startup-btn-primary:hover:not(:disabled) {
  transform: translateY(-1px);
  box-shadow: 0 8px 20px color-mix(in srgb, var(--accent) 42%, transparent);
}

.startup-btn-primary:active:not(:disabled) {
  transform: translateY(0);
  opacity: 0.9;
}

.startup-btn-primary:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.startup-btn-ghost {
  background: rgba(var(--ink-rgb), 0.05);
  color: var(--ink-3);
}

.startup-btn-ghost:hover {
  background: rgba(var(--ink-rgb), 0.08);
  color: var(--ink-2);
  transform: translateY(-1px);
}

.startup-btn-ghost:active {
  transform: translateY(0);
  opacity: 0.9;
}

/* 退出按钮 —— 圆形图标，悬停变红 */
.startup-btn-quit {
  width: 38px;
  height: 38px;
  padding: 0;
  justify-content: center;
  background: transparent;
  color: var(--ink-4);
  font-size: 17px;
}

.startup-btn-quit:hover {
  background: color-mix(in srgb, var(--danger) 10%, transparent);
  color: var(--danger);
  transform: scale(1.06);
}

.startup-btn-quit:active {
  transform: scale(0.94);
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
  border: 1px dashed rgba(var(--ink-rgb), 0.18);
  border-radius: 999px;
  padding: 8px 14px;
  font-size: 12px;
  font-weight: 700;
  color: var(--ink-2);
  cursor: pointer;
  transition: background 0.15s, border-color 0.15s, color 0.15s;
}
.startup-restore-btn i { font-size: 14px; }
.startup-restore-btn:hover {
  background: rgba(var(--ink-rgb), 0.05);
  border-color: var(--accent, var(--ink));
  color: var(--accent, var(--ink));
}

/* ═══ 备份导入对话框 ═══ */
.startup-backup-overlay {
  position: fixed;
  inset: 0;
  z-index: 50;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(var(--ink-rgb), 0.45);
  backdrop-filter: blur(8px);
  -webkit-backdrop-filter: blur(8px);
}
.startup-backup-card {
  width: 460px;
  max-width: 92vw;
  background: var(--surface);
  border-radius: 18px;
  padding: 22px 22px 18px;
  box-shadow: 0 24px 48px rgba(var(--ink-rgb), 0.18);
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.startup-backup-head {
  display: flex; align-items: center; gap: 10px;
}
.startup-backup-head i { font-size: 22px; color: var(--accent, var(--ink)); }
.startup-backup-head h3 {
  margin: 0; flex: 1 1 auto;
  font-size: 16px; font-weight: 800; color: var(--ink);
}
.startup-backup-close {
  background: transparent; border: none;
  font-size: 18px; line-height: 1;
  color: var(--ink-4); cursor: pointer;
  width: 28px; height: 28px;
  border-radius: 8px;
  transition: background 0.15s, color 0.15s;
}
.startup-backup-close:hover { background: rgba(var(--ink-rgb), 0.05); color: var(--ink-2); }
.startup-backup-desc {
  margin: 0;
  font-size: 12.5px; line-height: 1.5; color: var(--ink-2);
}
.startup-backup-list {
  list-style: none;
  margin: 0; padding: 10px 12px;
  background: rgba(var(--ink-rgb), 0.03);
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
  color: var(--ink-3); font-weight: 600;
}
.startup-backup-list li .value {
  flex: 1 1 auto; color: var(--ink); font-weight: 700;
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
