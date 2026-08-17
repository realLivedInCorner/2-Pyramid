<template>
  <div class="fanhua-settings">
    <header class="settings-header">
      <div class="header-left">
        <button class="back-btn" @click="goBack" :aria-label="t('common.backToHome')">
          <i class="ri-arrow-left-line back-icon" aria-hidden="true"></i>
          <span>{{ t('common.back') }}</span>
        </button>
        <div class="title-group">
          <h1 class="page-title">{{ t('settings.title') }}</h1>
          <p class="page-subtitle">{{ t('settings.subtitle') }}</p>
        </div>
      </div>
      <div class="header-search">
        <i class="ri-search-line search-icon"></i>
        <input 
          v-model="searchQuery" 
          :placeholder="t('settings.searchPlaceholder')"
          class="search-input"
        />
      </div>
      <div class="header-right"></div>
    </header>

    <main class="settings-scroll-area">
      <!-- 新增语言切换 (置于全局设置最前) -->
      <section class="settings-group" v-if="shouldShowGroup('language')">
        <h3 class="group-title">{{ t('settings.groups.language') }}</h3>
        <div class="group-card">
          <div class="setting-item" v-if="shouldShowItem('language')">
            <div class="item-icon">
              <i class="ri-global-line" aria-hidden="true"></i>
            </div>
            <div class="item-info">
              <div class="label">{{ t('settings.language.label') }}</div>
              <div class="desc">{{ t('settings.language.desc') }}</div>
            </div>
            <div class="item-action">
              <div class="segmented">
                <button class="seg-btn" :class="{ active: locale === 'zh-CN' }" @click="setLanguage('zh-CN')">中文</button>
                <button class="seg-btn" :class="{ active: locale === 'en-US' }" @click="setLanguage('en-US')">English</button>
              </div>
            </div>
          </div>
        </div>
      </section>

      <!-- 个人信息 -->
      <section class="settings-group" v-if="shouldShowGroup('personal')">
        <h3 class="group-title">{{ t('settings.groups.personal') }}</h3>
        <div class="group-card">
          <div class="setting-item" v-if="shouldShowItem('userName')">
            <div class="item-icon">
              <i class="ri-user-3-line" aria-hidden="true"></i>
            </div>
            <div class="item-info">
              <div class="label">{{ t('settings.userName.label') }}</div>
              <div class="desc">{{ t('settings.userName.desc', { name: localUserName || '—' }) }}</div>
            </div>
            <div class="item-action">
              <input
                class="inline-input"
                v-model="localUserName"
                :placeholder="t('settings.userName.placeholder')"
              />
            </div>
          </div>
        </div>
      </section>

      <!-- 全局设置优先 -->
      <section class="settings-group" v-if="shouldShowGroup('global')">
        <h3 class="group-title">{{ t('settings.groups.global') }}</h3>
        <div class="group-card">
          <div class="setting-item clickable" @click="openThemeDialog" v-if="shouldShowItem('theme')">
            <div class="item-icon">
              <i class="ri-palette-line" aria-hidden="true"></i>
            </div>
            <div class="item-info">
              <div class="label">{{ t('settings.theme.label') }}</div>
              <div class="desc">{{ t('settings.theme.desc', { color: themeColor }) }}</div>
            </div>
            <div class="item-action theme-swatches">
              <div class="swatch-block">
                <div class="swatch-label">{{ t('common.current') }}</div>
                <span class="color-swatch" :style="{ background: themeColor }"></span>
              </div>
              <div class="swatch-block">
                <div class="swatch-label">{{ t('common.default') }}</div>
                <button class="color-swatch reset-swatch" @click.stop="showResetDialog = true" :style="{ background: defaultThemeColor }"></button>
              </div>
            </div>
          </div>
          <div class="setting-item clickable" @click="openBackgroundDialog" v-if="shouldShowItem('background')">
            <div class="item-icon">
              <i class="ri-image-line" aria-hidden="true"></i>
            </div>
            <div class="item-info">
              <div class="label">{{ t('settings.background.label') }}</div>
              <div class="desc">{{ t('settings.background.desc') }}</div>
            </div>
            <div class="item-arrow">→</div>
          </div>
          <div class="setting-item" v-if="shouldShowItem('uiStyle')">
            <div class="item-icon">
              <i class="ri-contrast-2-line" aria-hidden="true"></i>
            </div>
            <div class="item-info">
              <div class="label">{{ t('settings.uiStyle.label') }}</div>
              <div class="desc">{{ t('settings.uiStyle.desc') }}</div>
            </div>
            <div class="item-action">
              <div class="segmented">
                <button class="seg-btn" :class="{ active: uiStyle === 'glass' }" @click="uiStyle = 'glass'">{{ t('settings.uiStyle.glass') }}</button>
                <button class="seg-btn" :class="{ active: uiStyle === 'frosted' }" @click="uiStyle = 'frosted'">{{ t('settings.uiStyle.frosted') }}</button>
              </div>
            </div>
          </div>
        </div>
      </section>

      <!-- 转换设置 -->
      <section class="settings-group" v-if="shouldShowGroup('convert')">
        <h3 class="group-title">{{ t('settings.groups.convert') }}</h3>
        <div class="group-card">
          <div class="setting-item" v-if="shouldShowItem('outputMode')">
            <div class="item-icon">
              <i class="ri-route-line" aria-hidden="true"></i>
            </div>
            <div class="item-info">
              <div class="label">{{ t('settings.outputMode.label') }}</div>
              <div class="desc">{{ t('settings.outputMode.desc') }}</div>
            </div>
            <div class="item-action">
              <div class="segmented">
                <button class="seg-btn" :class="{ active: outputMode === 'follow' }" @click="outputMode = 'follow'">{{ t('settings.outputMode.follow') }}</button>
                <button class="seg-btn" :class="{ active: outputMode === 'fixed' }" @click="outputMode = 'fixed'">{{ t('settings.outputMode.fixed') }}</button>
              </div>
            </div>
          </div>

          <div class="setting-item" v-if="outputMode === 'fixed' && shouldShowItem('outputMode')">
            <div class="item-icon">
              <i class="ri-folder-2-line" aria-hidden="true"></i>
            </div>
            <div class="item-info">
              <div class="label">{{ t('settings.outputPath.label') }}</div>
              <div class="desc">{{ outputPath }}</div>
            </div>
            <div class="item-action">
              <button class="btn-text" @click="showOutputDialog = true">{{ t('common.choose') }}</button>
            </div>
          </div>

          <div class="setting-item" v-if="shouldShowItem('sourceHandling')">
            <div class="item-icon">
              <i class="ri-delete-bin-2-line" aria-hidden="true"></i>
            </div>
            <div class="item-info">
              <div class="label">{{ t('settings.sourceHandling.label') }}</div>
              <div class="desc">{{ t('settings.sourceHandling.desc') }}</div>
            </div>
            <div class="item-action">
              <div class="segmented">
                <button class="seg-btn" :class="{ active: sourceHandling === 'ask' }" @click="sourceHandling = 'ask'">{{ t('settings.sourceHandling.ask') }}</button>
                <button class="seg-btn" :class="{ active: sourceHandling === 'delete' }" @click="sourceHandling = 'delete'">{{ t('settings.sourceHandling.delete') }}</button>
                <button class="seg-btn" :class="{ active: sourceHandling === 'keep' }" @click="sourceHandling = 'keep'">{{ t('settings.sourceHandling.keep') }}</button>
              </div>
            </div>
          </div>

          <div class="setting-item" v-if="shouldShowItem('openOutputAfterConvert')">
            <div class="item-icon">
              <i class="ri-external-link-line" aria-hidden="true"></i>
            </div>
            <div class="item-info">
              <div class="label">{{ t('settings.openOutputAfterConvert.label') }}</div>
              <div class="desc">{{ t('settings.openOutputAfterConvert.desc') }}</div>
            </div>
            <div class="item-action">
              <label class="switch">
                <input type="checkbox" v-model="openOutputAfterConvert" />
                <span class="slider"></span>
              </label>
            </div>
          </div>

          <div class="setting-item" v-if="shouldShowItem('conversionThreads')">
            <div class="item-icon">
              <i class="ri-cpu-line" aria-hidden="true"></i>
            </div>
            <div class="item-info">
              <div class="label">{{ t('settings.conversionThreads.label') }}</div>
              <div class="desc">{{ t('settings.conversionThreads.desc') }}</div>
            </div>
            <div class="item-action">
              <div class="segmented">
                <button
                  v-for="opt in conversionThreadsOptions"
                  :key="opt"
                  class="seg-btn"
                  :class="{ active: conversionThreads === opt }"
                  @click="conversionThreads = opt"
                >{{ opt }}</button>
              </div>
            </div>
          </div>

          <div class="setting-item clickable" @click="openNamingDialog" v-if="shouldShowItem('outputNaming')">
            <div class="item-icon">
              <i class="ri-file-text-line" aria-hidden="true"></i>
            </div>
            <div class="item-info">
              <div class="label">{{ t('settings.outputNaming.label') }}</div>
              <div class="desc">{{ t('settings.outputNaming.desc') }}：{{ namingTemplate }}</div>
            </div>
            <div class="item-arrow">→</div>
          </div>
        </div>
      </section>

      <!-- 转换历史 -->
      <section class="settings-group" v-if="shouldShowGroup('conversionHistory')">
        <h3 class="group-title">{{ t('settings.conversionHistory.groupTitle') }}</h3>
        <div class="group-card">
          <div class="setting-item clickable" @click="openHistoryDialog" v-if="shouldShowItem('conversionHistory')">
            <div class="item-icon">
              <i class="ri-history-line" aria-hidden="true"></i>
            </div>
            <div class="item-info">
              <div class="label">{{ t('settings.conversionHistory.label') }}</div>
              <div class="desc">{{ t('settings.conversionHistory.desc') }}（{{ historyEntries.length }}）</div>
            </div>
            <div class="item-arrow">→</div>
          </div>
        </div>
      </section>

      <!-- 通知设置 -->
      <section class="settings-group" v-if="shouldShowGroup('notification')">
        <h3 class="group-title">{{ t('settings.groups.notification') }}</h3>
        <div class="group-card">
          <div class="setting-item" v-if="shouldShowItem('notification')">
            <div class="item-icon">
              <i class="ri-notification-3-line" aria-hidden="true"></i>
            </div>
            <div class="item-info">
              <div class="label">{{ t('settings.notification.label') }}</div>
              <div class="desc">{{ t('settings.notification.desc') }}</div>
            </div>
            <div class="item-action">
              <label class="switch">
                <input type="checkbox" v-model="notificationEnabled" />
                <span class="slider"></span>
              </label>
            </div>
          </div>

          <div class="setting-item" v-if="notificationEnabled && shouldShowItem('notificationMode')">
            <div class="item-icon">
              <i class="ri-window-line" aria-hidden="true"></i>
            </div>
            <div class="item-info">
              <div class="label">{{ t('settings.notificationMode.label') }}</div>
              <div class="desc">{{ t('settings.notificationMode.desc') }}</div>
            </div>
            <div class="item-action">
              <div class="segmented">
                <button class="seg-btn" :class="{ active: notificationMode === 'system' }" @click="notificationMode = 'system'">{{ t('settings.notificationMode.system') }}</button>
                <button class="seg-btn" :class="{ active: notificationMode === 'app' }" @click="notificationMode = 'app'">{{ t('settings.notificationMode.app') }}</button>
                <button class="seg-btn" :class="{ active: notificationMode === 'both' }" @click="notificationMode = 'both'">{{ t('settings.notificationMode.both') }}</button>
              </div>
            </div>
          </div>

          <div class="setting-item clickable" @click="testNotification" v-if="shouldShowItem('testNotification')">
            <div class="item-icon">
              <i class="ri-test-tube-line" aria-hidden="true"></i>
            </div>
            <div class="item-info">
              <div class="label">{{ t('settings.testNotification.label') }}</div>
              <div class="desc">{{ t('settings.testNotification.desc') }}</div>
            </div>
            <div class="item-arrow">→</div>
          </div>

          <div class="setting-item" v-if="shouldShowItem('toastDuration')">
            <div class="item-icon">
              <i class="ri-timer-line" aria-hidden="true"></i>
            </div>
            <div class="item-info">
              <div class="label">{{ t('settings.toastDuration.label') }}</div>
              <div class="desc">{{ t('settings.toastDuration.desc') }}</div>
            </div>
            <div class="item-action">
              <div class="segmented">
                <button
                  v-for="opt in toastDurationOptions"
                  :key="opt"
                  class="seg-btn"
                  :class="{ active: toastDuration === opt }"
                  @click="toastDuration = opt"
                >{{ opt / 1000 }}s</button>
              </div>
            </div>
          </div>

          <div class="setting-item" v-if="shouldShowItem('toastPosition')">
            <div class="item-icon">
              <i class="ri-layout-masonry-line" aria-hidden="true"></i>
            </div>
            <div class="item-info">
              <div class="label">{{ t('settings.toastPosition.label') }}</div>
              <div class="desc">{{ t('settings.toastPosition.desc') }}</div>
            </div>
            <div class="item-action">
              <div class="segmented">
                <button class="seg-btn" :class="{ active: toastPosition === 'top-left' }" @click="toastPosition = 'top-left'">{{ t('settings.toastPosition.topLeft') }}</button>
                <button class="seg-btn" :class="{ active: toastPosition === 'top-right' }" @click="toastPosition = 'top-right'">{{ t('settings.toastPosition.topRight') }}</button>
                <button class="seg-btn" :class="{ active: toastPosition === 'bottom-left' }" @click="toastPosition = 'bottom-left'">{{ t('settings.toastPosition.bottomLeft') }}</button>
                <button class="seg-btn" :class="{ active: toastPosition === 'bottom-right' }" @click="toastPosition = 'bottom-right'">{{ t('settings.toastPosition.bottomRight') }}</button>
              </div>
            </div>
          </div>
        </div>
      </section>

      <!-- 动画速率设置 -->
      <section class="settings-group" v-if="shouldShowGroup('animationSpeed')">
        <h3 class="group-title">{{ t('settings.groups.animation') }}</h3>
        <div class="group-card">
          <div class="setting-item" v-if="shouldShowItem('animationSpeed')">
            <div class="item-icon">
              <i class="ri-speed-up-line" aria-hidden="true"></i>
            </div>
            <div class="item-info">
              <div class="label">{{ t('settings.animationSpeed.label') }}</div>
              <div class="desc">{{ t('settings.animationSpeed.desc') }}</div>
            </div>
            <div class="item-action">
              <div class="segmented">
                <button
                  v-for="opt in animationSpeedOptions"
                  :key="opt.value"
                  class="seg-btn"
                  :class="{ active: animationSpeed === opt.value }"
                  @click="animationSpeed = opt.value"
                >{{ t(opt.labelKey) }}</button>
              </div>
            </div>
          </div>
        </div>
      </section>

      <!-- 关闭行为设置已移除：关闭窗口现在始终直接退出应用 -->

      <!-- 高级设置 (工厂重置) -->
      <section class="settings-group" v-if="shouldShowGroup('factoryReset')">
        <h3 class="group-title">{{ t('settings.factoryReset.groupTitle') }}</h3>
        <div class="group-card">
          <div class="setting-item" v-if="shouldShowItem('factoryReset')">
            <div class="item-icon">
              <i class="ri-restart-line" aria-hidden="true"></i>
            </div>
            <div class="item-info">
              <div class="label">{{ t('settings.factoryReset.label') }}</div>
              <div class="desc">{{ t('settings.factoryReset.desc') }}</div>
            </div>
            <div class="item-action">
              <button class="btn-text danger" @click="showFactoryResetDialog = true">{{ t('settings.factoryReset.btn') }}</button>
            </div>
          </div>
        </div>
      </section>

      <!-- 版本设置 (新增) -->
      <section class="settings-group" v-if="shouldShowGroup('version')">
        <h3 class="group-title">{{ t('settings.groups.version') }}</h3>
        <div class="group-card">
          <div class="setting-item" v-if="shouldShowItem('channel')">
            <div class="item-icon">
              <i class="ri-git-branch-line" aria-hidden="true"></i>
            </div>
            <div class="item-info">
              <div class="label">{{ t('settings.updateChannel.label') }}</div>
              <div class="desc">{{ t('settings.updateChannel.desc') }}</div>
            </div>
            <div class="item-action">
              <div class="segmented">
                <button class="seg-btn" :class="{ active: updateChannel === 'master' }" @click="changeChannel('master')">{{ t('settings.updateChannel.stable') }}</button>
                <button class="seg-btn" :class="{ active: updateChannel === 'unstable' }" @click="changeChannel('unstable')">{{ t('settings.updateChannel.unstable') }}</button>
              </div>
            </div>
          </div>
          <div class="setting-item clickable" @click="showVersionInfo = true" v-if="shouldShowItem('versionInfo')">
            <div class="item-icon">
              <i class="ri-information-line" aria-hidden="true"></i>
            </div>
            <div class="item-info">
              <div class="label">{{ t('settings.versionInfo.label') }}</div>
              <div class="desc">{{ t('settings.versionInfo.desc') }}</div>
            </div>
            <div class="item-arrow">→</div>
          </div>
          <div class="setting-item clickable" @click="checkUpdate" v-if="shouldShowItem('update')">
            <div class="item-icon">
              <i v-if="updateChecking" class="ri-loader-4-line ri-spin" aria-hidden="true"></i>
              <i v-else class="ri-refresh-line" aria-hidden="true"></i>
            </div>
            <div class="item-info">
              <div class="label">{{ t('settings.checkUpdate.label') }}</div>
              <div class="desc">
                <template v-if="updateChecking">{{ t('settings.checkUpdate.checking') }}</template>
                <template v-else-if="updateError">{{ updateError }}</template>
                <template v-else>{{ t('settings.checkUpdate.desc', { version: currentVersion }) }}</template>
              </div>
            </div>
            <div class="item-arrow">→</div>
          </div>
        </div>
      </section>
    </main>

    <!-- 版本信息弹窗 (新增) -->
    <transition name="dialog-pop-quick">
      <div v-if="showVersionInfo" class="dialog-overlay" @click="showVersionInfo = false">
        <div class="dialog-content version-dialog" @click.stop>
          <div class="dialog-header">
            <h3>{{ t('settings.versionInfo.dialogTitle') }}</h3>
            <button class="dialog-close" @click="showVersionInfo = false">×</button>
          </div>
          <div class="dialog-body">
            <div class="version-hero">
              <img src="/favicon-192.png" class="version-logo" alt="2-Pyramid logo" />
              <button class="version-tag version-tap-target" @click="onVersionTap">2-Pyramid v{{ currentVersion }}</button>
              <div class="version-build">{{ appFullVersion }}</div>
              <div v-if="appIsDev" class="version-build-mode">Dev build</div>
              <div v-if="appIsBeta" class="version-build-mode beta">Beta 版本 · 测试渠道</div>
              <div v-if="devHint" class="dev-hint">{{ devHint }}</div>
            </div>
            <div class="changelog-area">
              <h4>{{ t('settings.versionInfo.changelogTitle') }}</h4>
              <ul>
                <li>{{ t('settings.versionInfo.changelog1') }}</li>
                <li>{{ t('settings.versionInfo.changelog2') }}</li>
                <li>{{ t('settings.versionInfo.changelog3') }}</li>
                <li>{{ t('settings.versionInfo.changelog4') }}</li>
              </ul>
            </div>
          </div>
          <div class="dialog-footer">
            <button class="btn-text" @click="showVersionInfo = false">{{ t('common.close') }}</button>
          </div>
        </div>
      </div>
    </transition>

    <transition name="dialog-pop">
      <div v-if="showDevUnlockDialog" class="dialog-overlay" @click="cancelDevUnlock">
        <div class="dialog-content dev-dialog" @click.stop>
          <div class="dialog-header">
            <h3>{{ t('settings.devMode.title') }}</h3>
            <button class="dialog-close" @click="cancelDevUnlock" :aria-label="t('common.close')">×</button>
          </div>
          <div class="dialog-body">
            <div class="dialog-hint">{{ t('settings.devMode.hint') }}</div>
            <div class="dev-code">DeveloperEnable</div>
            <div class="dialog-input-row">
              <input class="dialog-input" v-model="devUnlockInput" :placeholder="t('settings.devMode.placeholder')" />
            </div>
            <div v-if="devUnlockError" class="dev-error">{{ devUnlockError }}</div>
          </div>
          <div class="dialog-footer">
            <button class="btn-text secondary" @click="cancelDevUnlock">{{ t('common.cancel') }}</button>
            <button class="btn-text" @click="confirmDevUnlock">{{ t('settings.devMode.enter') }}</button>
          </div>
        </div>
      </div>
    </transition>

    <transition name="dialog-pop">
      <div v-if="showOutputDialog" class="dialog-overlay" @click="showOutputDialog = false">
        <div class="dialog-content" @click.stop>
          <div class="dialog-header">
            <h3>{{ t('settings.outputPath.dialogTitle') }}</h3>
            <button class="dialog-close" @click="showOutputDialog = false" :aria-label="t('common.close')">×</button>
          </div>
          <div class="dialog-body">
            <label class="dialog-label">{{ t('settings.outputPath.dialogLabel') }}</label>
            <div class="dialog-input-row">
              <input class="dialog-input" v-model="outputPath" :placeholder="t('settings.outputPath.dialogPlaceholder')" />
              <button class="btn-text" @click="pickOutputFolder">{{ t('common.choose') }}</button>
            </div>
            <p class="dialog-hint">{{ t('settings.outputPath.dialogHint') }}</p>
          </div>
          <div class="dialog-footer">
            <button class="btn-text secondary" @click="showOutputDialog = false">{{ t('common.cancel') }}</button>
            <button class="btn-text" @click="saveOutputPath">{{ t('common.save') }}</button>
          </div>
        </div>
      </div>
    </transition>

    <transition name="dialog-pop">
      <div v-if="showNamingDialog" class="dialog-overlay">
        <div class="dialog-content naming-dialog" @click.stop>
          <div class="dialog-header">
            <h3>{{ t('settings.outputNaming.dialogTitle') }}</h3>
            <button class="dialog-close" @click="showNamingDialog = false" :aria-label="t('common.close')">×</button>
          </div>
          <div class="dialog-body">
            <input
              v-model="namingDraft"
              class="naming-input"
              :placeholder="t('settings.outputNaming.placeholder')"
              spellcheck="false"
              maxlength="200"
              @keyup.enter="saveNamingDialog"
            />
            <div class="naming-row">
              <span class="naming-hint-label">{{ t('settings.outputNaming.tags') }}</span>
              <button
                v-for="tag in namingTags"
                :key="tag.token"
                class="naming-tag-btn"
                @click="insertNamingTag(tag.token)"
              >
                <span class="naming-tag-token">{{ tag.token }}</span>
                <span class="naming-tag-desc">{{ tag.desc }}</span>
              </button>
            </div>
            <div class="naming-preview">
              {{ t('settings.outputNaming.preview') }}:
              <b>{{ namingPreview }}</b>
            </div>
          </div>
          <div class="dialog-footer">
            <button class="btn-text secondary" @click="showNamingDialog = false">{{ t('common.cancel') }}</button>
            <button class="btn-text" @click="saveNamingDialog">{{ t('common.save') }}</button>
          </div>
        </div>
      </div>
    </transition>

    <transition name="dialog-pop">
      <div v-if="showHistoryDialog" class="dialog-overlay">
        <div class="dialog-content history-dialog" @click.stop>
          <div class="dialog-header">
            <h3>{{ t('settings.conversionHistory.groupTitle') }}</h3>
            <button class="dialog-close" @click="showHistoryDialog = false" :aria-label="t('common.close')">×</button>
          </div>
          <div class="dialog-body">
            <div class="history-list" v-if="historyEntries.length > 0">
              <div class="history-item" v-for="(h, i) in historyEntries" :key="i">
                <i class="history-status" :class="h.status === 'success' ? 'ok' : h.status === 'cancelled' ? 'cancelled' : 'fail'" aria-hidden="true"></i>
                <div class="history-info">
                  <div class="history-name">{{ historyFileName(h.input) }}</div>
                  <div class="history-meta">{{ h.time }} · {{ h.duration_s.toFixed(1) }}s</div>
                </div>
                <button
                  v-if="h.status === 'success' && h.output"
                  class="btn-text"
                  @click="openHistoryOutput(h.output)"
                >{{ t('settings.conversionHistory.openOutput') }}</button>
              </div>
            </div>
            <div class="history-empty" v-else>{{ t('settings.conversionHistory.empty') }}</div>
          </div>
          <div class="dialog-footer">
            <button class="btn-text secondary" @click="clearConversionHistory" :disabled="historyEntries.length === 0">
              {{ t('settings.conversionHistory.clear') }}
            </button>
            <button class="btn-text" @click="showHistoryDialog = false">{{ t('common.close') }}</button>
          </div>
        </div>
      </div>
    </transition>

    <!-- 更换背景对话框 -->
    <transition name="dialog-pop">
      <div v-if="showBackgroundDialog" class="dialog-overlay">
        <div class="dialog-content background-dialog" @click.stop>
          <div class="dialog-header">
            <h3>{{ t('settings.background.dialogTitle') }}</h3>
            <button class="dialog-close" @click="closeBackgroundDialog" :aria-label="t('common.close')">×</button>
          </div>
          <div class="dialog-body">
            <!-- 预览区 -->
            <div class="bg-preview" :style="bgPreviewStyle">
              <div v-if="!bgDraftPreview" class="bg-preview-empty">
                <i class="ri-image-add-line" aria-hidden="true"></i>
                <span>{{ t('settings.background.noImage') }}</span>
              </div>
            </div>
            <div class="dialog-input-row">
              <button class="btn-text" @click="pickBackgroundImage" :disabled="bgSyncing">
                {{ t('settings.background.choose') }}
              </button>
              <button
                v-if="currentBackgroundPath"
                class="btn-text danger"
                @click="removeBackground"
                :disabled="bgSyncing"
              >
                {{ t('settings.background.remove') }}
              </button>
            </div>

            <!-- 展示方式 -->
            <div class="bg-row">
              <span class="bg-row-label">{{ t('settings.background.fit') }}</span>
              <div class="segmented">
                <button class="seg-btn" :class="{ active: bgDraftFit === 'cover' }" @click="bgDraftFit = 'cover'">{{ t('settings.background.fitCover') }}</button>
                <button class="seg-btn" :class="{ active: bgDraftFit === 'contain' }" @click="bgDraftFit = 'contain'">{{ t('settings.background.fitContain') }}</button>
                <button class="seg-btn" :class="{ active: bgDraftFit === 'stretch' }" @click="bgDraftFit = 'stretch'">{{ t('settings.background.fitStretch') }}</button>
                <button class="seg-btn" :class="{ active: bgDraftFit === 'tile' }" @click="bgDraftFit = 'tile'">{{ t('settings.background.fitTile') }}</button>
              </div>
            </div>

            <!-- 透色强度 -->
            <div class="bg-row">
              <span class="bg-row-label">{{ t('settings.background.opacity') }}</span>
              <input
                type="range"
                min="20"
                max="100"
                step="5"
                v-model.number="bgDraftOpacity"
                class="bg-range"
              />
              <span class="bg-opacity-val">{{ bgDraftOpacity }}%</span>
            </div>

            <!-- 自取色 -->
            <div class="bg-row">
              <span class="bg-row-label">{{ t('settings.background.extractColor') }}</span>
              <label class="switch">
                <input type="checkbox" v-model="bgExtractColor" />
                <span class="slider"></span>
              </label>
            </div>
            <p class="dialog-hint">{{ t('settings.background.extractColorDesc') }}</p>
          </div>
          <div class="dialog-footer">
            <button class="btn-text secondary" @click="closeBackgroundDialog" :disabled="bgSyncing">
              {{ t('common.cancel') }}
            </button>
            <button
              class="btn-text"
              @click="submitBackground"
              :disabled="bgSyncing || (!bgDraftFile && !currentBackgroundPath)"
            >
              {{ t('settings.background.apply') }}
            </button>
          </div>

          <!-- Syncing 遮罩：提交处理中，中央旋转圆圈 -->
          <div v-if="bgSyncing" class="bg-syncing">
            <i class="ri-loader-4-line ri-spin" aria-hidden="true"></i>
            <span>Syncing...</span>
          </div>
        </div>
      </div>
    </transition>

    <transition name="dialog-pop">
      <div v-if="showThemeDialog" class="dialog-overlay" @click="showThemeDialog = false">
        <div class="dialog-content theme-dialog" @click.stop>
          <div class="dialog-header">
            <h3>{{ t('settings.theme.dialogTitle') }}</h3>
            <button class="dialog-close" @click="showThemeDialog = false" :aria-label="t('common.close')">×</button>
          </div>
          <div class="dialog-body">
            <label class="dialog-label">{{ t('settings.theme.currentColor') }}</label>
            <div class="theme-preview">
              <div class="preview-chip" :style="{ background: tempThemeColor }"></div>
              <div class="preview-text">
                <div class="preview-title">2-Pyramid Theme</div>
                <div class="preview-sub">{{ t('settings.theme.previewSubtitle') }}</div>
              </div>
            </div>
            <div class="picker-area" :style="{ '--hue': hue, '--picker-color': tempThemeColor }">
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
                  :style="{ accentColor: tempThemeColor }"
                  @input="onHueChange"
                />
                <div class="color-value">{{ tempThemeColor }}</div>
              </div>
            </div>
            <p class="dialog-hint">{{ t('settings.theme.applyHint') }}</p>
          </div>
          <div class="dialog-footer">
            <button class="btn-text secondary" @click="showThemeDialog = false">{{ t('common.back') }}</button>
            <button class="btn-text" @click="confirmThemeColor">{{ t('common.confirm') }}</button>
          </div>
        </div>
      </div>
    </transition>

    <transition name="dialog-pop">
      <div v-if="showResetDialog" class="dialog-overlay" @click="showResetDialog = false">
        <div class="dialog-content" @click.stop>
          <div class="dialog-header">
            <h3>{{ t('settings.theme.resetTitle') }}</h3>
            <button class="dialog-close" @click="showResetDialog = false" :aria-label="t('common.close')">×</button>
          </div>
          <div class="dialog-body">
            <p class="dialog-hint">{{ t('settings.theme.resetBody') }}</p>
            <div class="theme-preview">
              <div class="preview-chip" :style="{ background: defaultThemeColor }"></div>
              <div class="preview-text">
                <div class="preview-title">{{ t('settings.theme.resetPreview') }}</div>
                <div class="preview-sub">{{ defaultThemeColor }}</div>
              </div>
            </div>
          </div>
          <div class="dialog-footer">
            <button class="btn-text secondary" @click="showResetDialog = false">{{ t('common.back') }}</button>
            <button class="btn-text" @click="resetThemeColor">{{ t('common.confirm') }}</button>
          </div>
        </div>
      </div>
    </transition>

    <section class="settings-group" v-if="devModeEnabled">
      <h3 class="group-title">{{ t('settings.groups.dev') }}</h3>
      <div class="group-card">
        <div class="setting-item clickable" @click="showLogWindow = true">
          <div class="item-icon">
            <i class="ri-terminal-box-line" aria-hidden="true"></i>
          </div>
          <div class="item-info">
            <div class="label">2-PyramidLogWindow</div>
            <div class="desc">{{ t('settings.devMode.viewLog') }}</div>
          </div>
          <div class="item-arrow">→</div>
        </div>
        <div class="setting-item clickable danger" @click="showClearConfigDialog = true">
          <div class="item-icon">
            <i class="ri-delete-bin-line" aria-hidden="true"></i>
          </div>
          <div class="item-info">
            <div class="label">{{ t('settings.devMode.clearConfig') }}</div>
            <div class="desc">{{ t('settings.devMode.clearConfigDesc') }}</div>
          </div>
          <div class="item-arrow">→</div>
        </div>
        <div class="setting-item">
          <div class="item-icon">
            <i class="ri-radar-line" aria-hidden="true"></i>
          </div>
          <div class="item-info">
            <div class="label">{{ t('settings.devMode.actionMonitor') }}</div>
            <div class="desc">{{ t('settings.devMode.actionMonitorDesc') }}</div>
          </div>
          <div class="item-action">
            <label class="switch">
              <input type="checkbox" v-model="actionMonitorEnabled" />
              <span class="slider"></span>
            </label>
          </div>
        </div>
      </div>
    </section>

    <transition name="dialog-pop">
      <div v-if="showClearConfigDialog" class="dialog-overlay" @click="showClearConfigDialog = false">
        <div class="dialog-content confirm-dialog" @click.stop>
          <div class="dialog-header">
            <h3>{{ t('settings.devMode.clearConfigConfirmTitle') }}</h3>
            <button class="dialog-close" @click="showClearConfigDialog = false" :aria-label="t('common.close')">×</button>
          </div>
          <div class="dialog-body">
            <p>{{ t('settings.devMode.clearConfigConfirmBody') }}</p>
          </div>
          <div class="dialog-footer">
            <button class="btn-text secondary" @click="showClearConfigDialog = false">
              {{ t('settings.devMode.clearConfigConfirmCancel') }}
            </button>
            <button class="btn-text danger" @click="confirmClearConfig">
              {{ t('settings.devMode.clearConfigConfirmOk') }}
            </button>
          </div>
        </div>
      </div>
    </transition>

    <!-- Factory reset (delete user profile → next launch goes through OOBE) -->
    <transition name="dialog-pop-quick">
      <div v-if="showFactoryResetDialog" class="dialog-overlay" @click.self="showFactoryResetDialog = false">
        <div class="dialog-content confirm-dialog" @click.stop>
          <div class="dialog-header">
            <h3>{{ t('settings.factoryReset.confirmTitle') }}</h3>
            <button class="dialog-close" @click="showFactoryResetDialog = false" :aria-label="t('common.close')">×</button>
          </div>
          <div class="dialog-body">
            <p>{{ t('settings.factoryReset.confirmBody') }}</p>
            <label class="dialog-checkbox-row" @click.stop>
              <input type="checkbox" v-model="factoryResetDeep" />
              <span class="dialog-checkbox-text">{{ t('settings.factoryReset.deepLabel') }}</span>
              <span class="dialog-checkbox-hint">{{ t('settings.factoryReset.deepHint') }}</span>
            </label>
          </div>
          <div class="dialog-footer">
            <button class="btn-text secondary" @click="showFactoryResetDialog = false">
              {{ t('settings.factoryReset.cancelBtn') }}
            </button>
            <button class="btn-text danger" @click="confirmFactoryReset" :disabled="factoryResetBusy">
              {{ factoryResetBusy ? t('common.loading') : t('settings.factoryReset.confirmBtn') }}
            </button>
          </div>
        </div>
      </div>
    </transition>

    <transition name="dialog-pop">
      <div v-if="showLogWindow" class="dialog-overlay" @click="closeLogWindow">
        <div class="dialog-content log-dialog" @click.stop>
          <div class="dialog-header">
            <h3>2-PyramidLogWindow</h3>
            <button class="dialog-close" @click="closeLogWindow" :aria-label="t('common.close')">×</button>
          </div>
          <div class="dialog-body">
            <div class="log-toolbar">
              <button class="btn-text secondary" @click="refreshLogs">{{ t('common.refresh') }}</button>
              <button class="btn-text" @click="exportLog">{{ t('settings.devMode.exportLog') }}</button>
            </div>
            <pre class="log-output">{{ logsText || t('common.noLogs') }}</pre>
          </div>
        </div>
      </div>
    </transition>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch, onUnmounted } from 'vue';
import { useI18n } from 'vue-i18n'
import { open, save } from '@tauri-apps/plugin-dialog';
import { invoke } from '@tauri-apps/api/core';
import { getVersion } from '@tauri-apps/api/app';
import { resolveImageUrl } from '../utils/assetUrl';
import { useUpdater } from '../composables/useUpdater';
import { useNotification, type NotificationMode } from '../composables/useNotification';
import { useLanguage } from '../composables/useLanguage';
import { useAppInfo } from '../composables/useAppInfo';
const { t } = useI18n()
const { locale, setLanguage } = useLanguage()

const props = defineProps<{
  devMode?: boolean;
  userName?: string;
  sourceHandling?: 'ask' | 'delete' | 'keep';
  openOutputAfterConvert?: boolean;
}>();
const emit = defineEmits([
  'switch-page',
  'update:dev-mode',
  'update:user-name',
  'update:animation-speed',
  'update:source-handling',
  'update:open-output-after-convert',
  'update:action-monitor',
  'update:background',
  'update:ui-style',
  'show-update-dialog',
  'reset-to-oobe',
]);

const { notify, setNotificationEnabled, setNotificationMode, setToastDuration } = useNotification();
const { full: appFullVersion, isDev: appIsDev, isBeta: appIsBeta } = useAppInfo();

const outputMode = ref<'follow' | 'fixed'>('follow');
const outputPath = ref('C:/Users/Admin/Documents/2-Pyramid/Output');
const showOutputDialog = ref(false);
const notificationEnabled = ref(true);
const notificationMode = ref<NotificationMode>('both');
type AnimationSpeed = 'slow' | 'normal' | 'fast';
const animationSpeed = ref<AnimationSpeed>('normal');
const animationSpeedOptions: { value: AnimationSpeed; labelKey: string }[] = [
  { value: 'slow', labelKey: 'settings.animationSpeed.slow' },
  { value: 'normal', labelKey: 'settings.animationSpeed.normal' },
  { value: 'fast', labelKey: 'settings.animationSpeed.fast' },
];
const sourceHandling = ref<'ask' | 'delete' | 'keep'>(props.sourceHandling ?? 'ask');
const openOutputAfterConvert = ref<boolean>(props.openOutputAfterConvert ?? true);
// Toast 通知自定义：显示时长（秒）与屏幕角落位置
const toastDuration = ref(8000);
const toastDurationOptions = [4000, 6000, 8000, 10000, 12000];
const toastPosition = ref<'top-left' | 'top-right' | 'bottom-left' | 'bottom-right'>('top-right');
// 批量转换并行资源包数
const conversionThreads = ref(2);
const conversionThreadsOptions = [1, 2, 4];
// 输出文件命名模板（占位符：[Name] [Ver] [Time]）
const namingWelcome = t('settings.outputNaming.defaultName');
const namingTemplate = ref('[Ver][Name]');
const showNamingDialog = ref(false);
const namingDraft = ref('[Ver][Name]');
const namingTags = [
  { token: '[Name]', label: 'Name', desc: t('settings.outputNaming.nameDesc') },
  { token: '[Ver]', label: 'Version', desc: t('settings.outputNaming.verDesc') },
  { token: '[Time]', label: 'Time', desc: t('settings.outputNaming.timeDesc') },
];
const namingPreview = computed(() => {
  const render = (tpl: string) => tpl
    // [Name] 是原材质包名；预览里用欢迎文案演示
    .replace(/\[Name\]/g, namingWelcome)
    .replace(/\[Ver\]/g, '[Java 1.20-1.20.1]')
    .replace(/\[Time\]/g, '20260816-101234');
  const rendered = render(namingDraft.value).trim();
  return (rendered || namingWelcome) + '.zip';
});
const openNamingDialog = () => {
  namingDraft.value = namingTemplate.value;
  showNamingDialog.value = true;
};
const saveNamingDialog = () => {
  namingTemplate.value = namingDraft.value;
  showNamingDialog.value = false;
};
const insertNamingTag = (token: string) => {
  const current = namingDraft.value.trimEnd();
  namingDraft.value = current ? `${current} ${token}` : token;
};
// 转换历史
interface HistoryEntry {
  input: string;
  output: string | null;
  status: string;
  error: string | null;
  time: string;
  duration_s: number;
}
const historyEntries = ref<HistoryEntry[]>([]);
const showHistoryDialog = ref(false);

const openHistoryDialog = () => {
  showHistoryDialog.value = true;
  loadConversionHistory();
};

const loadConversionHistory = () => {
  invoke<HistoryEntry[]>('get_conversion_history')
    .then((entries) => { historyEntries.value = entries ?? []; })
    .catch(() => {});
};

const clearConversionHistory = async () => {
  try {
    await invoke('clear_conversion_history');
    historyEntries.value = [];
    notify({
      title: t('settings.conversionHistory.groupTitle'),
      body: t('settings.conversionHistory.cleared'),
      type: 'success',
      source: 'system',
    });
  } catch { /* ignore */ }
};

const openHistoryOutput = (path: string) => {
  invoke('open_folder', { path }).catch(() => {});
};

const historyFileName = (p: string) => p.split(/[\\/]/).pop() ?? p;
const showThemeDialog = ref(false);
const showResetDialog = ref(false);
const showClearConfigDialog = ref(false);
const showFactoryResetDialog = ref(false);
// 动作监视（开发者诊断）：记录前端所有点击行为到日志
const actionMonitorEnabled = ref(false);

// ── 自定义背景 ────────────────────────────────────────────────
const showBackgroundDialog = ref(false);
const bgDraftFile = ref<string | null>(null);
const bgDraftPreview = ref('');
const bgDraftFit = ref<'cover' | 'contain' | 'stretch' | 'tile'>('cover');
const bgDraftOpacity = ref(80); // 百分比 20–100
const bgExtractColor = ref(true);
const bgSyncing = ref(false);
const currentBackgroundPath = ref<string | null>(null);
const currentBackgroundFit = ref<'cover' | 'contain' | 'stretch' | 'tile'>('cover');
const currentBackgroundOpacity = ref(1);
// 控件表面样式：玻璃 / 磨砂
const uiStyle = ref<'glass' | 'frosted'>('glass');

const bgPreviewStyle = computed(() => {
  if (!bgDraftPreview.value) return {};
  return {
    backgroundImage: `url(${bgDraftPreview.value})`,
    backgroundSize: bgDraftFit.value === 'stretch' ? '100% 100%' : bgDraftFit.value,
    backgroundRepeat: bgDraftFit.value === 'tile' ? 'repeat' : 'no-repeat',
    backgroundPosition: 'center',
    opacity: bgDraftOpacity.value / 100,
  };
});

const openBackgroundDialog = () => {
  bgDraftFile.value = null;
  bgDraftPreview.value = '';
  bgDraftFit.value = currentBackgroundFit.value;
  bgDraftOpacity.value = Math.round(currentBackgroundOpacity.value * 100);
  bgExtractColor.value = true;
  bgSyncing.value = false;
  showBackgroundDialog.value = true;
  // 已有背景时直接加载预览，便于只调透色/展示方式
  if (currentBackgroundPath.value) {
    resolveImageUrl(currentBackgroundPath.value)
      .then((url) => { bgDraftPreview.value = url; })
      .catch(() => {});
  }
};

const closeBackgroundDialog = () => {
  if (bgSyncing.value) return;
  showBackgroundDialog.value = false;
};

const pickBackgroundImage = async () => {
  try {
    const selected = await open({
      multiple: false,
      filters: [{
        name: t('settings.background.filter'),
        extensions: ['png', 'jpg', 'jpeg', 'webp', 'gif', 'bmp'],
      }],
    });
    if (selected && typeof selected === 'string') {
      bgDraftFile.value = selected;
      bgDraftPreview.value = await resolveImageUrl(selected);
    }
  } catch (e) {
    console.error('[background] pick failed:', e);
  }
};

const submitBackground = async () => {
  // 既没有新图片、也没有已设置的背景 → 无操作
  if (bgSyncing.value || (!bgDraftFile.value && !currentBackgroundPath.value)) return;
  bgSyncing.value = true;
  try {
    if (bgDraftFile.value) {
      const result = await invoke<{ background_path: string | null; theme_color: string | null }>(
        'set_background',
        {
          filePath: bgDraftFile.value,
          fit: bgDraftFit.value,
          opacity: bgDraftOpacity.value / 100,
          extractColor: bgExtractColor.value,
        },
      );
      currentBackgroundPath.value = result.background_path;
      currentBackgroundFit.value = bgDraftFit.value;
      currentBackgroundOpacity.value = bgDraftOpacity.value / 100;
      if (result.theme_color) {
        themeColor.value = result.theme_color;
        tempThemeColor.value = result.theme_color;
      }
      emit('update:background', {
        path: result.background_path,
        fit: bgDraftFit.value,
        opacity: bgDraftOpacity.value / 100,
        themeColor: result.theme_color,
      });
    } else {
      // 仅调整透色强度 / 展示方式，无需重选图片
      await invoke('update_background_settings', {
        fit: bgDraftFit.value,
        opacity: bgDraftOpacity.value / 100,
      });
      currentBackgroundFit.value = bgDraftFit.value;
      currentBackgroundOpacity.value = bgDraftOpacity.value / 100;
      emit('update:background', {
        path: currentBackgroundPath.value,
        fit: bgDraftFit.value,
        opacity: bgDraftOpacity.value / 100,
        themeColor: null,
      });
    }
    showBackgroundDialog.value = false;
    notify({
      title: t('settings.background.dialogTitle'),
      body: t('settings.background.success'),
      type: 'success',
      source: 'system',
    });
  } catch (e) {
    notify({
      title: t('settings.background.dialogTitle'),
      body: t('settings.background.failed', { error: String(e) }),
      type: 'error',
      source: 'system',
    });
  } finally {
    bgSyncing.value = false;
  }
};

const removeBackground = async () => {
  if (bgSyncing.value) return;
  bgSyncing.value = true;
  try {
    await invoke('clear_background');
    currentBackgroundPath.value = null;
    bgDraftFile.value = null;
    bgDraftPreview.value = '';
    emit('update:background', { path: null, fit: 'cover', opacity: 1, themeColor: null });
    notify({
      title: t('settings.background.dialogTitle'),
      body: t('settings.background.removed'),
      type: 'success',
      source: 'system',
    });
  } catch (e) {
    notify({
      title: t('settings.background.dialogTitle'),
      body: t('settings.background.failed', { error: String(e) }),
      type: 'error',
      source: 'system',
    });
  } finally {
    bgSyncing.value = false;
  }
};
const factoryResetDeep = ref(false);
const factoryResetBusy = ref(false);
const defaultThemeColor = '#007bff';
const themeColor = ref('#007bff');
const tempThemeColor = ref('#007bff');
const hue = ref(210);
const sat = ref(100);
const val = ref(100);
const svRef = ref<HTMLElement | null>(null);
const isPicking = ref(false);

const localUserName = ref(props.userName || '');
const searchQuery = ref('');
const showVersionInfo = ref(false);
const devModeEnabled = ref(!!props.devMode);
const versionTapCount = ref(0);
const devHint = ref('');
const showDevUnlockDialog = ref(false);
const devUnlockInput = ref('');
const devUnlockError = ref('');
const showLogWindow = ref(false);
const logsText = ref('');
let logTimer: ReturnType<typeof setInterval> | null = null;

// ── Updater ──────────────────────────────────────
const { checkForUpdate, getChannel } = useUpdater();
const updateChannel = ref('master');
const updateChecking = ref(false);
const updateError = ref('');
const currentVersion = ref('');

async function loadUpdateChannel() {
  try {
    updateChannel.value = await getChannel();
  } catch { /* use default */ }
}

async function currentVersionFromConfig() {
  try {
    const cfg = await invoke<any>('get_config');
    if (cfg?.update_channel) updateChannel.value = cfg.update_channel;
  } catch { /* use default */ }
}

async function changeChannel(ch: string) {
  updateChannel.value = ch;
  try {
    await invoke('set_update_channel', { channel: ch });
  } catch (e) {
    console.error('set_update_channel failed', e);
  }
}

const settingItems = [
  { id: 'language', group: 'language', label: t('settings.language.label'), desc: t('settings.language.desc') },
  { id: 'userName', group: 'personal', label: t('settings.userName.label'), desc: t('settings.userName.desc', { name: localUserName.value || '—' }) },
  { id: 'theme', group: 'global', label: t('settings.theme.label'), desc: t('settings.theme.searchDesc') },
  { id: 'background', group: 'global', label: t('settings.background.label'), desc: t('settings.background.desc') },
  { id: 'uiStyle', group: 'global', label: t('settings.uiStyle.label'), desc: t('settings.uiStyle.desc') },
  { id: 'outputMode', group: 'convert', label: t('settings.outputMode.label'), desc: t('settings.outputMode.desc') },
  { id: 'notification', group: 'notification', label: t('settings.notification.label'), desc: t('settings.notification.desc') },
  { id: 'notificationMode', group: 'notification', label: t('settings.notificationMode.label'), desc: t('settings.notificationMode.desc') },
  { id: 'testNotification', group: 'notification', label: t('settings.testNotification.label'), desc: t('settings.testNotification.desc') },
  { id: 'toastDuration', group: 'notification', label: t('settings.toastDuration.label'), desc: t('settings.toastDuration.desc') },
  { id: 'toastPosition', group: 'notification', label: t('settings.toastPosition.label'), desc: t('settings.toastPosition.desc') },
  { id: 'conversionThreads', group: 'convert', label: t('settings.conversionThreads.label'), desc: t('settings.conversionThreads.desc') },
  { id: 'outputNaming', group: 'convert', label: t('settings.outputNaming.label'), desc: t('settings.outputNaming.desc') },
  { id: 'conversionHistory', group: 'conversionHistory', label: t('settings.conversionHistory.label'), desc: t('settings.conversionHistory.desc') },
  { id: 'channel', group: 'version', label: t('settings.updateChannel.label'), desc: t('settings.updateChannel.desc') },
  { id: 'animationSpeed', group: 'animationSpeed', label: t('settings.animationSpeed.label'), desc: t('settings.animationSpeed.desc') },
  { id: 'versionInfo', group: 'version', label: t('settings.versionInfo.label'), desc: t('settings.versionInfo.desc') },
  { id: 'update', group: 'version', label: t('settings.checkUpdate.label'), desc: t('settings.checkUpdate.searchDesc') }
];

const shouldShowGroup = (groupId: string) => {
  if (!searchQuery.value) return true;
  return settingItems.some(item => 
    item.group === groupId && 
    (item.label.toLowerCase().includes(searchQuery.value.toLowerCase()) || 
     item.desc.toLowerCase().includes(searchQuery.value.toLowerCase()))
  );
};

const shouldShowItem = (itemId: string) => {
  if (!searchQuery.value) return true;
  const item = settingItems.find(i => i.id === itemId);
  if (!item) return false;
  return item.label.toLowerCase().includes(searchQuery.value.toLowerCase()) || 
         item.desc.toLowerCase().includes(searchQuery.value.toLowerCase());
};

const backToHome = () => emit('switch-page', 'home');
const goBack = backToHome;
const checkUpdate = async () => {
  updateChecking.value = true;
  updateError.value = '';
  try {
    const channel = await getChannel();
    const result = await checkForUpdate(channel);
    if (result.hasUpdate) {
      emit('show-update-dialog', result);
    } else {
      updateError.value = t('settings.checkUpdate.alreadyLatest');
    }
  } catch (e: any) {
    updateError.value = typeof e === 'string' ? e : t('settings.checkUpdate.checkFailed');
  } finally {
    updateChecking.value = false;
  }
};

const testNotification = async () => {
  await notify({
    title: t('settings.testNotification.testTitle'),
    body: t('settings.testNotification.testBody'),
    type: 'info',
    source: 'system'
  });
};

const onVersionTap = async () => {
  if (devModeEnabled.value) return;
  if (showDevUnlockDialog.value) return;
  versionTapCount.value += 1;
  if (versionTapCount.value >= 7) {
    devHint.value = 'DeveloperEnable';
    showDevUnlockDialog.value = true;
    return;
  }
  devHint.value = t('settings.devMode.hintBefore', { count: 7 - versionTapCount.value });
};

const cancelDevUnlock = () => {
  showDevUnlockDialog.value = false;
  devUnlockInput.value = '';
  devUnlockError.value = '';
};

const confirmDevUnlock = async () => {
  devUnlockError.value = '';
  if (devUnlockInput.value.trim() !== 'DeveloperEnable') {
    devUnlockError.value = t('settings.devMode.error');
    return;
  }
  devModeEnabled.value = true;
  showDevUnlockDialog.value = false;
  devUnlockInput.value = '';
  emit('update:dev-mode', true);
  try {
    await invoke('set_dev_mode', { enabled: true });
  } catch {
    // ignore
  }
};

const refreshLogs = async () => {
  try {
    logsText.value = await invoke<string>('get_logs');
  } catch (e) {
    logsText.value = t('settings.devMode.logError', { error: e });
  }
};

const exportLog = async () => {
  try {
    const defaultPath = await invoke<string | null>('get_log_path');
    const dest = await save({
      defaultPath: defaultPath || undefined,
      filters: [{ name: 'Log', extensions: ['log', 'txt'] }],
    });
    if (dest) {
      const result = await invoke<string>('export_logs', { dest });
      await notify({
        title: t('common.success'),
        body: t('settings.devMode.exportSuccess', { path: result }),
        type: 'success',
        source: 'system',
      });
    }
  } catch (e) {
    await notify({
      title: t('common.error'),
      body: t('settings.devMode.exportFailed', { error: String(e) }),
      type: 'error',
      source: 'system',
    });
  }
};

/**
 * Dev-only: wipe the on-disk settings.json and every localStorage key
 * the app has written, then reload the page. Intended for QA regression
 * loops where the user wants to start from a clean slate.
 */
const confirmClearConfig = async () => {
  showClearConfigDialog.value = false;
  try {
    // 1. Delete the Tauri-side settings.json
    await invoke<string>('clear_config');
    // 2. Wipe localStorage (this is origin-scoped to the Tauri webview, so
    //    it only removes app keys, not other browser data)
    localStorage.clear();
    // 3. Notify the user before we reload so the message is visible
    await notify({
      title: t('settings.devMode.clearConfigDoneTitle'),
      body: t('settings.devMode.clearConfigDoneBody'),
      type: 'success',
      source: 'system',
    });
  } catch (e) {
    await notify({
      title: t('settings.devMode.clearConfigErrorTitle'),
      body: t('settings.devMode.clearConfigErrorBody', { error: String(e) }),
      type: 'error',
      source: 'system',
    });
    return;
  }
  // Small delay so the toast is visible before the page tears down
  setTimeout(() => {
    window.location.reload();
  }, 600);
};

/**
 * User-facing factory reset. Unlike `confirmClearConfig` (dev-only,
 * gated) this is the production entry point exposed in Settings →
 * Advanced. It deletes the on-disk settings.json so the next launch
 * falls back to defaults, then reloads the page so the freshly-empty
 * state takes effect and OOBE fires on the next launch (or now, if
 * the user reopens before that).
 */
const confirmFactoryReset = async () => {
  factoryResetBusy.value = true;
  const deep = factoryResetDeep.value;
  factoryResetDeep.value = false; // reset for next open
  console.log('[factory_reset] starting, deep=', deep);

  try {
    if (deep) {
      const report = await invoke<{ config_path: string; logs_deleted: number; overlay_history_cleared: boolean }>(
        'factory_reset_deep',
      );
      console.log('[factory_reset] deep result:', report);
    } else {
      const path = await invoke<string>('factory_reset');
      console.log('[factory_reset] config deleted:', path);
    }
    // Also wipe localStorage keys we set ourselves so the next launch
    // starts truly fresh. We don't clear everything (that would also
    // nuke unrelated keys if the user has any), only the 2pyr-owned
    // ones.
    localStorage.removeItem('sourceHandling');
    localStorage.removeItem('openOutputAfterConvert');
    localStorage.removeItem('animationSpeed');
    localStorage.removeItem('themeColor');
    localStorage.removeItem('language');
    showFactoryResetDialog.value = false;
    factoryResetBusy.value = false;

    // Tell App.vue to show the OOBE immediately. We do NOT try to
    // exit or reload the app — in the destroy/recreate-window
    // architecture the exit path is fragile (ExitRequested
    // interception) and reload doesn't reset in-memory state. Showing
    // OOBE right here is instant, reliable, and the user sees the
    // effect immediately.
    emit('reset-to-oobe');
  } catch (e) {
    console.error('[factory_reset] failed:', e);
    await notify({
      title: t('settings.factoryReset.failedTitle'),
      body: String(e),
      type: 'error',
      source: 'system',
    });
    // Keep the dialog open so the user can retry without losing the
    // “deep” checkbox state.
    factoryResetBusy.value = false;
  }
};

const closeLogWindow = () => {
  showLogWindow.value = false;
  if (logTimer) {
    clearInterval(logTimer);
    logTimer = null;
  }
};

const pickOutputFolder = async () => {
  try {
    const dir = await open({ directory: true, multiple: false });
    if (dir && typeof dir === 'string') {
      outputPath.value = dir;
    }
  } catch (e) {
    console.error('pickOutputFolder failed', e);
  }
};

const saveOutputPath = async () => {
  if (outputMode.value === 'fixed' && outputPath.value.trim().length > 0) {
    try {
      await invoke('create_dir', { path: outputPath.value });
    } catch (e) {
      console.error('create_dir failed', e);
    }
  }
  try {
    await invoke('update_config', {
      patch: {
        outputMode: outputMode.value,
        outputPath: outputPath.value,
      }
    });
  } catch (e) {
    console.error('update_config failed', e);
  }
  showOutputDialog.value = false;
};

onMounted(() => {
  const savedMode = localStorage.getItem('outputMode');
  const savedPath = localStorage.getItem('outputPath');
  if (savedMode === 'follow' || savedMode === 'fixed') {
    outputMode.value = savedMode;
  }
  if (savedPath) {
    outputPath.value = savedPath;
  }
  const savedThemeColor = localStorage.getItem('themeColor');
  if (savedThemeColor) {
    themeColor.value = savedThemeColor;
    tempThemeColor.value = savedThemeColor;
  }

  const savedNotificationEnabled = localStorage.getItem('notificationEnabled');
  if (savedNotificationEnabled === 'false') {
    notificationEnabled.value = false;
  }

  const savedNotificationMode = localStorage.getItem('notificationMode');
  if (savedNotificationMode === 'system' || savedNotificationMode === 'app' || savedNotificationMode === 'both') {
    notificationMode.value = savedNotificationMode;
  }

  const savedAnimationSpeed = localStorage.getItem('animationSpeed');
  if (savedAnimationSpeed === 'slow' || savedAnimationSpeed === 'normal' || savedAnimationSpeed === 'fast') {
    animationSpeed.value = savedAnimationSpeed;
  }
  emit('update:animation-speed', animationSpeed.value);

  invoke<any>('get_config')
    .then((cfg) => {
      if (cfg?.output_mode === 'follow' || cfg?.output_mode === 'fixed') {
        outputMode.value = cfg.output_mode;
      }
      if (typeof cfg?.output_path === 'string' && cfg.output_path.length > 0) {
        outputPath.value = cfg.output_path;
      }
      if (cfg?.palette?.theme_color) {
        themeColor.value = cfg.palette.theme_color;
        tempThemeColor.value = cfg.palette.theme_color;
      }
      if (typeof cfg?.notification_enabled === 'boolean') {
        notificationEnabled.value = cfg.notification_enabled;
      }
      if (cfg?.notification_mode === 'system' || cfg?.notification_mode === 'app' || cfg?.notification_mode === 'both') {
        notificationMode.value = cfg.notification_mode;
      }
      if (typeof cfg?.toast_duration_ms === 'number' && cfg.toast_duration_ms >= 4000 && cfg.toast_duration_ms <= 15000) {
        toastDuration.value = cfg.toast_duration_ms;
        setToastDuration(cfg.toast_duration_ms);
      }
      if (cfg?.toast_position === 'top-left' || cfg?.toast_position === 'top-right' || cfg?.toast_position === 'bottom-left' || cfg?.toast_position === 'bottom-right') {
        toastPosition.value = cfg.toast_position;
      }
      if (typeof cfg?.background_image === 'string' && cfg.background_image.length > 0) {
        currentBackgroundPath.value = cfg.background_image;
      }
      if (cfg?.background_fit === 'cover' || cfg?.background_fit === 'contain' || cfg?.background_fit === 'stretch' || cfg?.background_fit === 'tile') {
        currentBackgroundFit.value = cfg.background_fit;
      }
      if (typeof cfg?.background_opacity === 'number') {
        currentBackgroundOpacity.value = cfg.background_opacity;
      }
      if (cfg?.ui_style === 'glass' || cfg?.ui_style === 'frosted') {
        uiStyle.value = cfg.ui_style;
      }
      if (typeof cfg?.conversion_threads === 'number' && [1, 2, 4].includes(cfg.conversion_threads)) {
        conversionThreads.value = cfg.conversion_threads;
      }
      if (typeof cfg?.output_naming === 'string' && cfg.output_naming.length > 0) {
        // 迁移旧值（default/timestamp/overwrite）到模板语义
        const legacy: Record<string, string> = {
          default: '[Ver][Name]',
          timestamp: '[Ver][Time]',
          overwrite: '[Name]',
        };
        namingTemplate.value = legacy[cfg.output_naming] ?? cfg.output_naming;
      }
    })
    .catch(() => {});

  const savedSourceHandling = localStorage.getItem('sourceHandling');
  if (savedSourceHandling === 'ask' || savedSourceHandling === 'delete' || savedSourceHandling === 'keep') {
    sourceHandling.value = savedSourceHandling;
  }
  const savedOpenOutput = localStorage.getItem('openOutputAfterConvert');
  if (savedOpenOutput === 'true' || savedOpenOutput === 'false') {
    openOutputAfterConvert.value = savedOpenOutput === 'true';
  }

  currentVersionFromConfig();
  loadUpdateChannel();
  loadConversionHistory();

  // Load version from Tauri app metadata (matches Python script's version bump)
  getVersion().then(v => { currentVersion.value = v; }).catch(() => {});
  invoke<boolean>('get_dev_mode')
    .then((enabled) => {
      devModeEnabled.value = !!enabled;
    })
    .catch(() => {});
  invoke<boolean>('is_action_monitor')
    .then((enabled) => {
      actionMonitorEnabled.value = !!enabled;
    })
    .catch(() => {});
});

watch(actionMonitorEnabled, (val) => {
  invoke('set_action_monitor', { enabled: val }).catch(() => {});
  emit('update:action-monitor', val);
});

watch(showVersionInfo, (open) => {
  if (open) return;
  versionTapCount.value = 0;
  devHint.value = '';
  cancelDevUnlock();
});

watch(showLogWindow, async (open) => {
  if (!open) return;
  await refreshLogs();
  if (logTimer) clearInterval(logTimer);
  logTimer = setInterval(refreshLogs, 1200);
});

watch(() => props.devMode, (v) => {
  if (typeof v === 'boolean') devModeEnabled.value = v;
});

onUnmounted(() => {
  if (logTimer) clearInterval(logTimer);
});

watch(outputMode, (val) => {
  localStorage.setItem('outputMode', val);
  invoke('update_config', { patch: { outputMode: val } }).catch(() => {});
});

watch(outputPath, (val) => {
  localStorage.setItem('outputPath', val);
  invoke('update_config', { patch: { outputPath: val } }).catch(() => {});
});

watch(notificationEnabled, (val) => {
  localStorage.setItem('notificationEnabled', String(val));
  invoke('update_config', { patch: { notificationEnabled: val } }).catch(() => {});
  setNotificationEnabled(val);
});

watch(notificationMode, (val) => {
  localStorage.setItem('notificationMode', val);
  invoke('update_config', { patch: { notificationMode: val } }).catch(() => {});
  setNotificationMode(val);
});

watch(animationSpeed, (val) => {
  localStorage.setItem('animationSpeed', val);
  emit('update:animation-speed', val);
});

watch(sourceHandling, (val) => {
  localStorage.setItem('sourceHandling', val);
  invoke('update_config', { patch: { sourceHandling: val } }).catch(() => {});
  emit('update:source-handling', val);
});

watch(openOutputAfterConvert, (val) => {
  localStorage.setItem('openOutputAfterConvert', String(val));
  invoke('update_config', { patch: { openOutputAfterConvert: val } }).catch(() => {});
  emit('update:open-output-after-convert', val);
});

watch(toastDuration, (val) => {
  setToastDuration(val);
  invoke('update_config', { patch: { toastDurationMs: val } }).catch(() => {});
});

watch(toastPosition, (val) => {
  invoke('update_config', { patch: { toastPosition: val } }).catch(() => {});
});

watch(conversionThreads, (val) => {
  invoke('update_config', { patch: { conversionThreads: val } }).catch(() => {});
});

watch(uiStyle, (val) => {
  invoke('update_config', { patch: { uiStyle: val } }).catch(() => {});
  emit('update:ui-style', val);
});

watch(namingTemplate, (val) => {
  invoke('update_config', { patch: { outputNaming: val } }).catch(() => {});
});

watch(() => props.userName, (v) => {
  if (v !== undefined) localUserName.value = v;
});

watch(localUserName, (val) => {
  emit('update:user-name', val);
  invoke('update_config', { patch: { userName: val } }).catch(() => {});
});

const openThemeDialog = () => {
  tempThemeColor.value = themeColor.value;
  const hsv = hexToHsv(tempThemeColor.value);
  hue.value = hsv.h;
  sat.value = hsv.s;
  val.value = hsv.v;
  showThemeDialog.value = true;
};

const confirmThemeColor = async () => {
  themeColor.value = tempThemeColor.value;
  document.documentElement.style.setProperty('--theme-color', themeColor.value);
  localStorage.setItem('themeColor', themeColor.value);
  try {
    await invoke('update_config', {
      patch: { palette: { theme_color: themeColor.value } }
    });
  } catch (e) {
    console.error('update_config failed', e);
  }
  showThemeDialog.value = false;
};

const resetThemeColor = async () => {
  themeColor.value = defaultThemeColor;
  tempThemeColor.value = defaultThemeColor;
  document.documentElement.style.setProperty('--theme-color', themeColor.value);
  localStorage.setItem('themeColor', themeColor.value);
  try {
    await invoke('update_config', {
      patch: { palette: { theme_color: themeColor.value } }
    });
  } catch (e) {
    console.error('update_config failed', e);
  }
  showResetDialog.value = false;
};

const updateTempFromHsv = () => {
  tempThemeColor.value = hsvToHex(hue.value, sat.value, val.value);
};

const onHueChange = (e: Event) => {
  const v = Number((e.target as HTMLInputElement).value);
  hue.value = v;
  updateTempFromHsv();
};

const startPick = (event: MouseEvent) => {
  isPicking.value = true;
  handlePick(event);
  window.addEventListener('mousemove', handlePick);
  window.addEventListener('mouseup', endPick);
};

const endPick = () => {
  isPicking.value = false;
  window.removeEventListener('mousemove', handlePick);
  window.removeEventListener('mouseup', endPick);
};

const handlePick = (event: MouseEvent) => {
  if (!svRef.value) return;
  const rect = svRef.value.getBoundingClientRect();
  const x = Math.min(Math.max(event.clientX - rect.left, 0), rect.width);
  const y = Math.min(Math.max(event.clientY - rect.top, 0), rect.height);
  sat.value = Math.round((x / rect.width) * 100);
  val.value = Math.round(100 - (y / rect.height) * 100);
  updateTempFromHsv();
};

const hsvToHex = (h: number, s: number, v: number) => {
  const sat = s / 100;
  const val = v / 100;
  const c = val * sat;
  const x = c * (1 - Math.abs(((h / 60) % 2) - 1));
  const m = val - c;
  let r = 0, g = 0, b = 0;
  if (h < 60) { r = c; g = x; b = 0; }
  else if (h < 120) { r = x; g = c; b = 0; }
  else if (h < 180) { r = 0; g = c; b = x; }
  else if (h < 240) { r = 0; g = x; b = c; }
  else if (h < 300) { r = x; g = 0; b = c; }
  else { r = c; g = 0; b = x; }
  const toHex = (n: number) => {
    const v = Math.round((n + m) * 255);
    return v.toString(16).padStart(2, '0');
  };
  return `#${toHex(r)}${toHex(g)}${toHex(b)}`;
};

const hexToHsv = (hex: string) => {
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
};
</script>

<style scoped>
.fanhua-settings {
  width: 100%; height: 100%; min-height: 0;
  /* Background gradient + aurora ::before are provided by App.vue's
     `.page-shell > *` rule so all three pages render identically. */
  color: #1d1d1f;
  display: flex; flex-direction: column; overflow: hidden;
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", "PingFang SC", "Microsoft YaHei", sans-serif;
  position: relative;
}

.settings-header {
  width: 100%;
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(220px, 520px) minmax(0, 1fr);
  align-items: center;
  gap: 24px;
  padding: 24px 48px 16px;
  position: relative;
  z-index: 1;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 20px;
  justify-self: start;
  min-width: 0;
}

/* Title + subtitle stack vertically (matches ConversionPage). */
.header-left > .title-group {
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.header-search {
  width: 100%;
  max-width: 420px;
  position: relative;
  justify-self: center;
}

.header-right {
  justify-self: end;
}

.search-icon {
  position: absolute;
  left: 14px;
  top: 50%;
  transform: translateY(-50%);
  color: #94a3b8;
  font-size: 18px;
}

.search-input {
  width: 100%;
  padding: 10px 16px 10px 44px;
  border-radius: 12px;
  border: 2px solid rgba(0, 0, 0, 0.05);
  background: rgba(255, 255, 255, 0.8);
  backdrop-filter: blur(10px);
  font-size: 14px;
  transition: all 0.3s;
  outline: none;
}

.search-input:focus {
  border-color: var(--theme-color);
  background: #fff;
  box-shadow: 0 4px 12px rgba(var(--theme-color-rgb), 0.1);
}

.page-title {
  font-size: clamp(22px, 3.2vw, 32px);
  font-weight: 800;
  color: #0f172a;
  margin: 0;
  min-width: 0;
  max-width: 16ch;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.page-subtitle { margin: 6px 0 0; color: #86868b; font-size: 13px; }

.settings-scroll-area {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding-right: 10px;
}
.settings-scroll-area::-webkit-scrollbar { width: 6px; }
.settings-scroll-area::-webkit-scrollbar-thumb {
  background: rgba(0,0,0,0.12); border-radius: 3px;
}
.settings-scroll-area::-webkit-scrollbar-thumb:hover {
  background: rgba(0,0,0,0.22);
}
.version-dialog {
  max-width: 500px;
}

.version-hero {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 16px;
  padding: 24px 0;
  border-bottom: 1px solid #f1f5f9;
  margin-bottom: 24px;
}

.version-logo {
  width: 80px;
  height: 80px;
}

.version-tag {
  border: none;
  background: transparent;
  font-size: 20px;
  font-weight: 800;
  color: #0f172a;
  cursor: pointer;
}

.version-tap-target:hover {
  color: var(--theme-color);
}

.version-build {
  margin-top: 4px;
  font-size: 12px;
  font-weight: 600;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  color: #64748b;
  letter-spacing: 0.4px;
  padding: 3px 10px;
  border-radius: 6px;
  background: color-mix(in srgb, var(--theme-color) 8%, transparent);
}

.version-build-mode {
  margin-top: 4px;
  font-size: 10px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.8px;
  color: #f59e0b;
}

.version-build-mode.beta {
  display: inline-block;
  text-transform: none;
  letter-spacing: 0.4px;
  padding: 3px 10px;
  border-radius: 999px;
  color: #ea580c;
  background: rgba(249, 115, 22, 0.12);
}

.dev-hint {
  font-size: 12px;
  font-weight: 700;
  color: var(--theme-color);
}

.dev-dialog {
  width: min(520px, 92vw);
}

.dev-code {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
  font-size: 14px;
  padding: 10px 12px;
  border-radius: 12px;
  background: rgba(0,0,0,0.04);
  border: 1px solid rgba(0,0,0,0.06);
  color: #0f172a;
  margin: 10px 0 12px;
  user-select: text;
}

.dev-error {
  margin-top: 10px;
  font-size: 12px;
  font-weight: 700;
  color: #ef4444;
}

.changelog-area h4 {
  font-size: 14px;
  font-weight: 700;
  color: #64748b;
  margin-bottom: 12px;
}

.changelog-area ul {
  list-style: none;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.changelog-area li {
  font-size: 14px;
  color: #475569;
  line-height: 1.5;
  display: flex;
  gap: 8px;
}

.changelog-area li::before {
  content: "•";
  color: var(--theme-color);
  font-weight: bold;
}

.back-btn {
  background: rgba(0, 0, 0, 0.05);
  border: none;
  padding: 10px 18px;
  border-radius: 14px;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  gap: 8px;
  font-weight: 600;
  color: #111827;
  transition: 0.3s;
  flex: 0 0 auto;
}
.back-icon { font-size: 16px; line-height: 1; color: #111827; }
.back-btn:hover { background: rgba(0, 0, 0, 0.1); transform: translateX(-4px); }
.settings-scroll-area::-webkit-scrollbar-track { background: transparent; }

@media (max-width: 720px) {
  .settings-header {
    grid-template-columns: 1fr;
    gap: 14px;
    padding: 20px 16px 12px;
  }
  .header-left {
    justify-self: start;
  }
  .header-search {
    justify-self: stretch;
    max-width: none;
  }
  .page-title {
    max-width: 22ch;
  }
  .settings-group { padding: 0 16px; }
  .settings-scroll-area { padding-right: 4px; }
}

@media (max-width: 520px) {
  .back-btn span {
    display: none;
  }
}

.settings-group { margin-top: 30px; width: 100%; padding: 0 48px; }
.group-title {
  font-size: 13px; font-weight: 700; color: #86868b;
  margin-left: 15px; margin-bottom: 10px; text-transform: uppercase; letter-spacing: 1px;
}

.group-card {
  background: rgba(255, 255, 255, 0.6);
  backdrop-filter: blur(20px);
  border: 1px solid rgba(0, 0, 0, 0.05);
  border-radius: 24px;
  overflow: hidden;
  box-shadow: 0 4px 20px rgba(0,0,0,0.02);
}

.setting-item {
  display: flex; align-items: center; padding: 18px 20px;
  border-bottom: 1px solid rgba(0, 0, 0, 0.03);
  transition: 0.2s;
}
.setting-item:last-child { border-bottom: none; }
.setting-item.clickable { cursor: pointer; }
.setting-item.clickable:hover { background: rgba(0, 0, 0, 0.02); }

.item-icon { width: 40px; color: #111827; display: inline-flex; align-items: center; justify-content: center; }
.item-icon i { font-size: 20px; line-height: 1; }
.item-info { flex: 1; display: flex; flex-direction: column; gap: 2px; }
.item-info .label { font-size: 15px; font-weight: 600; color: #1d1d1f; }
.item-info .desc { font-size: 12px; color: #86868b; }

.item-arrow { color: #c6c6c8; font-weight: 800; }

.fanhua-select {
  background: rgba(0, 0, 0, 0.05); border: none; padding: 6px 10px;
  border-radius: 8px; font-size: 13px; outline: none;
}

.segmented {
  display: inline-flex; background: rgba(0,0,0,0.05); padding: 4px; border-radius: 999px; gap: 4px;
}
.seg-btn {
  border: none; background: transparent; padding: 6px 12px; border-radius: 999px;
  font-size: 12px; font-weight: 600; color: #6b7280; cursor: pointer; transition: 0.2s;
}
.seg-btn.active { background: #fff; color: #1d1d1f; box-shadow: 0 6px 12px rgba(0,0,0,0.08); }

.btn-text {
  background: color-mix(in srgb, var(--theme-color) 12%, transparent); color: var(--theme-color);
  border: none; padding: 6px 14px; border-radius: 10px;
  font-size: 13px; font-weight: 600; cursor: pointer;
}
.btn-text.secondary { background: rgba(0,0,0,0.06); color: #334155; }
.btn-text:disabled { opacity: 0.5; cursor: not-allowed; }

/* 转换历史列表（对话框内） */
.history-dialog { width: 520px; max-width: 92vw; }

/* 背景更换对话框 */
.background-dialog {
  width: 520px;
  max-width: 92vw;
  position: relative;
}
.background-dialog .dialog-body {
  gap: 14px;
}
.bg-preview {
  width: 100%;
  height: 180px;
  border-radius: 14px;
  border: 1px solid rgba(0, 0, 0, 0.08);
  background: rgba(0, 0, 0, 0.04);
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
}
.bg-preview-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  color: #94a3b8;
  font-size: 13px;
}
.bg-preview-empty i { font-size: 30px; }
.bg-row {
  display: flex;
  align-items: center;
  gap: 12px;
}
.bg-row-label {
  flex: 0 0 76px;
  font-size: 13px;
  font-weight: 600;
  color: #1a1a2e;
}
.bg-range {
  flex: 1 1 auto;
  accent-color: var(--theme-color);
}
.bg-opacity-val {
  flex: 0 0 42px;
  text-align: right;
  font-size: 12px;
  color: #64748b;
}

/* Syncing 遮罩：中央旋转圆圈 + Syncing... */
.bg-syncing {
  position: absolute;
  inset: 0;
  z-index: 10;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
  background: rgba(255, 255, 255, 0.82);
  backdrop-filter: blur(4px);
  -webkit-backdrop-filter: blur(4px);
  border-radius: inherit;
  font-size: 15px;
  font-weight: 700;
  color: #1a1a2e;
}
.bg-syncing i {
  font-size: 26px;
  color: var(--theme-color);
}
.history-list {
  display: flex;
  flex-direction: column;
  max-height: 420px;
  overflow-y: auto;
  border-top: 1px solid rgba(0, 0, 0, 0.06);
}
.history-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 4px;
  border-bottom: 1px solid rgba(0, 0, 0, 0.05);
}
.history-status {
  width: 8px; height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}
.history-status.ok { background: #10b981; }
.history-status.fail { background: #ef4444; }
.history-status.cancelled { background: #94a3b8; }
.history-info { flex: 1; min-width: 0; }
.history-name {
  font-size: 13px; font-weight: 600; color: #1e293b;
  white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
}
.history-meta { font-size: 12px; color: #94a3b8; margin-top: 2px; }
.history-empty { padding: 20px 0; text-align: center; color: #94a3b8; font-size: 13px; }

/* 输出命名模板编辑器（对话框内） */
.naming-dialog { width: 480px; max-width: 90vw; }
.naming-dialog .dialog-body {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.naming-input {
  width: 100%;
  padding: 9px 12px;
  font-size: 13px;
  font-family: inherit;
  border: 1.5px solid rgba(0, 0, 0, 0.12);
  border-radius: 10px;
  background: rgba(0, 0, 0, 0.04);
  color: #1a1a2e;
  outline: none;
}
.naming-input:focus { border-color: var(--theme-color); background: #fff; }
.naming-row {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}
.naming-hint-label { font-size: 12px; color: #94a3b8; min-width: 76px; }
.naming-tag-btn {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
  padding: 6px 12px;
  border-radius: 8px;
  border: 1px dashed rgba(0, 0, 0, 0.18);
  background: transparent;
  cursor: pointer;
  transition: all 0.15s;
}
.naming-tag-btn:hover { border-color: var(--theme-color); background: color-mix(in srgb, var(--theme-color) 6%, transparent); }
.naming-tag-token {
  font-size: 12px;
  font-weight: 600;
  font-family: ui-monospace, Consolas, monospace;
  color: #475569;
}
.naming-tag-btn:hover .naming-tag-token { color: var(--theme-color); }
.naming-tag-desc {
  font-size: 10.5px;
  color: #94a3b8;
  line-height: 1.2;
}
.naming-preview {
  font-size: 12.5px;
  color: #64748b;
  padding: 8px 12px;
  border-radius: 10px;
  background: rgba(0, 0, 0, 0.03);
  word-break: break-all;
}
.naming-preview b { color: #0f172a; }

.inline-input {
  padding: 7px 14px;
  font-size: 13px;
  font-family: inherit;
  font-weight: 500;
  border: 1.5px solid rgba(0, 0, 0, 0.12);
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.8);
  color: #1a1a2e;
  outline: none;
  transition: all 0.2s ease;
  width: 160px;
  text-align: right;
}
.inline-input:focus {
  border-color: var(--theme-color);
  box-shadow: 0 0 0 2px color-mix(in srgb, var(--theme-color) 12%, transparent);
}
.inline-input::placeholder {
  color: #9ca3af;
}

.switch { position: relative; display: inline-block; width: 42px; height: 24px; }
.switch input { opacity: 0; width: 0; height: 0; }
.slider {
  position: absolute; inset: 0; cursor: pointer; background-color: #e9e9eb; transition: .4s; border-radius: 34px;
}
.slider:before {
  position: absolute; content: ""; height: 18px; width: 18px; left: 3px; bottom: 3px; background-color: white; transition: .4s; border-radius: 50%;
  box-shadow: 0 2px 4px rgba(0,0,0,0.1);
}
.switch input:checked + .slider { background-color: var(--theme-color); }
.switch input:checked + .slider:before { transform: translateX(18px); }
.switch input:disabled + .slider { opacity: 0.5; cursor: not-allowed; }
.switch.disabled { cursor: not-allowed; }

/* Dev-only destructive action */
.setting-item.danger .item-icon { color: #b91c1c; }
.setting-item.danger:hover { background: rgba(239, 68, 68, 0.06); }
.btn-text.danger { color: #b91c1c; }
.btn-text.danger:hover { background: rgba(239, 68, 68, 0.08); }

.status-badge {
  display: inline-block;
  margin-left: 8px;
  padding: 2px 8px;
  border-radius: 10px;
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.3px;
  vertical-align: middle;
}
.status-loading      { background: #f1f5f9; color: #64748b; }
.status-busy         { background: #dbeafe; color: #1d4ed8; }
.status-registered   { background: #dcfce7; color: #15803d; }
.status-unregistered { background: #fef3c7; color: #b45309; }
.status-partial      { background: #fee2e2; color: #b91c1c; }

.dialog-overlay {
  position: fixed; inset: 0; background: rgba(15, 23, 42, 0.3);
  display: flex; align-items: center; justify-content: center; z-index: 200;
  backdrop-filter: blur(6px);
}
.dialog-content {
  width: 420px; max-width: 90vw;
  background: rgba(255,255,255,0.95); border: 1px solid rgba(0,0,0,0.06);
  border-radius: 20px; padding: 20px; box-shadow: 0 20px 50px rgba(0,0,0,0.2);
}
.log-dialog {
  width: min(900px, 90vw);
}
.log-toolbar {
  display: flex;
  justify-content: flex-end;
  margin-bottom: 8px;
}
.log-output {
  height: 360px;
  overflow: auto;
  background: #0b1020;
  color: #a8ffb0;
  border-radius: 10px;
  padding: 12px;
  font-size: 12px;
  line-height: 1.45;
}
.dialog-header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 12px; }
.dialog-header h3 { font-size: 16px; margin: 0; }
.dialog-close { border: none; background: transparent; font-size: 20px; cursor: pointer; color: #64748b; }
.dialog-body { display: flex; flex-direction: column; gap: 8px; }

/* Inline checkbox row inside a dialog (e.g. the factory-reset “deep
   clean” toggle). The whole row is the hit target so the user can
   click anywhere on the label, not just the small box. */
.dialog-checkbox-row {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  padding: 10px 12px;
  margin-top: 8px;
  border-radius: 10px;
  background: rgba(0, 0, 0, 0.04);
  cursor: pointer;
  transition: background 0.15s;
}
.dialog-checkbox-row:hover { background: rgba(0, 0, 0, 0.07); }
.dialog-checkbox-row input[type="checkbox"] {
  margin-top: 2px;
  width: 16px; height: 16px;
  cursor: pointer;
  accent-color: var(--theme-color, #007bff);
}
.dialog-checkbox-text {
  font-weight: 700; font-size: 13px; color: #0f172a;
}
.dialog-checkbox-hint {
  display: block;
  margin-top: 2px;
  font-size: 11.5px; color: #64748b; line-height: 1.4;
}
.dialog-label { font-size: 12px; color: #64748b; }
.dialog-input {
  border: 1px solid rgba(0,0,0,0.08);
  border-radius: 10px; padding: 10px 12px; font-size: 13px;
}
.dialog-input-row {
  display: flex;
  align-items: center;
  gap: 10px;
}
.dialog-input-row .dialog-input {
  flex: 1;
}

.theme-swatches {
  display: flex;
  align-items: center;
  gap: 16px;
}

.swatch-block {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
}

.swatch-label {
  font-size: 11px;
  color: #94a3b8;
}

.color-swatch {
  width: 22px;
  height: 22px;
  border-radius: 6px;
  border: 1px solid rgba(0,0,0,0.1);
  box-shadow: inset 0 0 0 1px rgba(255,255,255,0.6);
}

.reset-swatch {
  cursor: pointer;
  border: 1px solid rgba(0,0,0,0.12);
  background: transparent;
  padding: 0;
}

.theme-dialog {
  width: 460px;
}

.theme-preview {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 12px;
  border-radius: 14px;
  background: rgba(0,0,0,0.04);
  border: 1px solid rgba(0,0,0,0.06);
}

.preview-chip {
  width: 52px;
  height: 52px;
  border-radius: 14px;
  box-shadow: 0 10px 20px rgba(0,0,0,0.12);
}

.preview-title { font-weight: 700; font-size: 14px; }
.preview-sub { font-size: 12px; color: #64748b; }

.picker-area {
  margin-top: 12px;
  display: flex;
  align-items: center;
  gap: 16px;
}

.sv-panel {
  position: relative;
  width: 220px;
  height: 140px;
  border-radius: 12px;
  background: linear-gradient(90deg, #fff, hsl(var(--hue), 100%, 50%));
  overflow: hidden;
  border: 1px solid #111;
  box-shadow: 0 0 0 1px rgba(0,0,0,0.35), 0 8px 20px rgba(0,0,0,0.18), 0 0 18px color-mix(in srgb, var(--picker-color) 30%, transparent);
  cursor: crosshair;
}

.sv-white {
  position: absolute;
  inset: 0;
  background: linear-gradient(90deg, #fff, rgba(255,255,255,0));
}

.sv-black {
  position: absolute;
  inset: 0;
  background: linear-gradient(0deg, #000, rgba(0,0,0,0));
}

.sv-cursor {
  position: absolute;
  width: 12px;
  height: 12px;
  border: 2px solid #fff;
  border-radius: 50%;
  box-shadow: 0 0 0 2px rgba(0,0,0,0.4), 0 0 12px color-mix(in srgb, var(--picker-color) 60%, transparent);
  transform: translate(-6px, -6px);
}

.picker-side {
  display: flex;
  flex-direction: column;
  gap: 10px;
  align-items: flex-start;
}

.hue-slider {
  width: 160px;
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
  box-shadow: 0 0 10px color-mix(in srgb, var(--picker-color) 70%, transparent), 0 2px 8px rgba(0,0,0,0.25);
  cursor: pointer;
  margin-top: -4px;
}

.hue-slider::-moz-range-thumb {
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: var(--picker-color);
  border: 2px solid #111;
  box-shadow: 0 0 10px color-mix(in srgb, var(--picker-color) 70%, transparent), 0 2px 8px rgba(0,0,0,0.25);
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
  font-size: 13px;
  color: #64748b;
}
.dialog-hint { font-size: 12px; color: #94a3b8; margin: 0; }
.dialog-footer { display: flex; justify-content: flex-end; gap: 10px; margin-top: 16px; }

/* .dialog-pop-* 规则已删,SettingsPage 的 dialog 走 App.vue 全局规则。
   原因:scoped 0.25s transition: all 期间 dialog-overlay 整个在 transform,
   backdrop-filter 跟着缩(「shader 收缩」),box-shadow 也跟着缩。
   App.vue 全局用 overlay 只动 opacity、content 动 opacity+transform 的分层方案,
   overlay 永远不 transform,backdrop-filter 区域稳定。 */

.ri-spin { animation: ri-spin 1s linear infinite; }
@keyframes ri-spin { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }
</style>
