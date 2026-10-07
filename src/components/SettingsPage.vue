<template>
  <div class="fanhua-settings">
    <!-- 全应用共用的舞台背景，见 shell/StageBackdrop.vue -->
    <StageBackdrop />

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

    <Workbench :steps="categories" :label="t('settings.title')" @go="selectCategory">
      <div class="stage-panel stage-narrow">
      <!-- 新增语言切换 (置于全局设置最前) -->
      <section class="settings-group" v-if="shouldShowGroup('language') && inCategory('language')">
        <h3 class="group-title">{{ t('settings.groups.language') }}</h3>
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
      </section>

      <!-- 个人信息 -->
      <section class="settings-group" v-if="shouldShowGroup('personal') && inCategory('personal')">
        <h3 class="group-title">{{ t('settings.groups.personal') }}</h3>
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
      </section>

      <!-- 全局设置优先 -->
      <section class="settings-group" v-if="shouldShowGroup('global') && inCategory('global')">
        <h3 class="group-title">{{ t('settings.groups.global') }}</h3>

          <!--
            实验功能：应用内文件浏览器。
            默认关闭 —— 它没有安全性与可用性保证（能指向网络驱动器、超长路径，
            也显示不出系统对话框能显示的全部内容）。关闭时所有选择一律走系统对话框，
            走的是已知可用的那条路。
          -->
          <div class="setting-item" v-if="shouldShowItem('inAppPicker')">
            <div class="item-icon">
              <i class="ri-folder-open-line" aria-hidden="true"></i>
            </div>
            <div class="item-info">
              <div class="label">
                {{ t('settings.inAppPicker.label') }}
                <span class="exp-tag">{{ t('common.experimental') }}</span>
              </div>
              <div class="desc">{{ t('settings.inAppPicker.desc') }}</div>
              <div class="desc warn">{{ t('settings.inAppPicker.disclaimer') }}</div>
            </div>
            <div class="item-action">
              <label class="switch">
                <input type="checkbox" v-model="inAppPickerOn" />
                <span class="slider"></span>
              </label>
            </div>
          </div>
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
                <button class="color-swatch reset-swatch" @click.stop="openThemeReset" :style="{ background: defaultThemeColor }"></button>
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
      </section>

      <!-- 转换设置 -->
      <section class="settings-group" v-if="shouldShowGroup('convert') && inCategory('convert')">
        <h3 class="group-title">{{ t('settings.groups.convert') }}</h3>
          <div class="setting-item">
            <div class="item-icon">
              <i class="ri-code-box-line" aria-hidden="true"></i>
            </div>
            <div class="item-info">
              <div class="label">Editor Mode / Foray</div>
              <div class="desc">{{ t('settings.forayAi.editorModeDesc') }}</div>
            </div>
            <div class="item-action">
              <label class="switch-wrap">
                <span class="switch">
                  <input type="checkbox" :checked="editorMode" @change="onEditorModeChange" />
                  <span class="slider"></span>
                </span>
              </label>
            </div>
          </div>

          <div class="setting-item" v-if="editorMode">
            <div class="item-icon">
              <i class="ri-brain-line" aria-hidden="true"></i>
            </div>
            <div class="item-info">
              <div class="label">{{ t('settings.forayAi.label') }}</div>
              <div class="desc">{{ t('settings.forayAi.desc') }}</div>
            </div>
          </div>
          <div v-if="editorMode" class="foray-ai-form">
            <div class="foray-ai-grid">
              <label>Base URL
                <input v-model="aiBaseUrl" type="text" placeholder="https://api.openai.com/v1" />
              </label>
              <label>API Key
                <input v-model="aiApiKey" type="password" autocomplete="off" />
              </label>
              <label>Model
                <input v-model="aiModel" type="text" placeholder="gpt-4o-mini" />
              </label>
              <label>{{ t('settings.forayAi.defaultTier') }}
                <select v-model.number="aiTier">
                  <option :value="1">{{ t('settings.forayAi.tier1') }}</option>
                  <option :value="2">{{ t('settings.forayAi.tier2') }}</option>
                  <option :value="3">{{ t('settings.forayAi.tier3') }}</option>
                  <option :value="4">{{ t('settings.forayAi.tier4') }}</option>
                  <option :value="5">{{ t('settings.forayAi.tier5') }}</option>
                </select>
              </label>
            </div>
            <label class="wide">{{ t('settings.forayAi.systemPrompt') }}
              <textarea v-model="aiPrompt" rows="3" :placeholder="t('settings.forayAi.promptPlaceholder')"></textarea>
            </label>
            <div class="foray-ai-actions">
              <button class="ghost-btn" type="button" @click="saveAiToBackend">{{ t('settings.forayAi.save') }}</button>
              <button class="ghost-btn" type="button" @click="restoreAiPrompt">{{ t('settings.forayAi.restorePrompt') }}</button>
              <button class="btn-text" type="button" @click="testAiConnection">{{ t('settings.forayAi.testConnection') }}</button>
            </div>
            <p v-if="aiMsg" class="foray-ai-msg">{{ aiMsg }}</p>
          </div>
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
              <div class="label">{{ t('settings.performanceMode.label') }}</div>
              <div class="desc">
                {{ performanceMode === 'performance'
                  ? t('settings.performanceMode.performanceDesc')
                  : t('settings.performanceMode.balancedDesc') }}
              </div>
              <div v-if="perfPlan" class="desc perf-detail">
                {{ t('settings.performanceMode.detail', {
                  cores: perfPlan.cores,
                  threads: perfPlan.threads,
                  packs: perfPlan.packs,
                }) }}
                <span v-if="perfPlan.note" class="perf-note">· {{ perfPlan.note }}</span>
              </div>
            </div>
            <div class="item-action">
              <div class="segmented">
                <button
                  class="seg-btn"
                  :class="{ active: performanceMode === 'balanced' }"
                  @click="performanceMode = 'balanced'"
                >{{ t('settings.performanceMode.balanced') }}</button>
                <button
                  class="seg-btn"
                  :class="{ active: performanceMode === 'performance' }"
                  @click="performanceMode = 'performance'"
                >{{ t('settings.performanceMode.performance') }}</button>
              </div>
            </div>
          </div>

          <div class="setting-item clickable" @click="showNamingDialog = true" v-if="shouldShowItem('outputNaming')">
            <div class="item-icon">
              <i class="ri-file-text-line" aria-hidden="true"></i>
            </div>
            <div class="item-info">
              <div class="label">{{ t('settings.outputNaming.label') }}</div>
              <div class="desc">{{ t('settings.outputNaming.descInline', { name: namingTemplate }) }}</div>
            </div>
            <div class="item-arrow">→</div>
          </div>
      </section>

      <!-- 转换历史 -->
      <section class="settings-group" v-if="shouldShowGroup('conversionHistory') && inCategory('conversionHistory')">
        <h3 class="group-title">{{ t('settings.conversionHistory.groupTitle') }}</h3>
          <div class="setting-item clickable" @click="showHistoryDialog = true" v-if="shouldShowItem('conversionHistory')">
            <div class="item-icon">
              <i class="ri-history-line" aria-hidden="true"></i>
            </div>
            <div class="item-info">
              <div class="label">{{ t('settings.conversionHistory.label') }}</div>
              <div class="desc">{{ t('settings.conversionHistory.desc') }}</div>
            </div>
            <div class="item-arrow">→</div>
          </div>
      </section>

      <!-- 通知设置 -->
      <section class="settings-group" v-if="shouldShowGroup('notification') && inCategory('notification')">
        <h3 class="group-title">{{ t('settings.groups.notification') }}</h3>
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
      </section>

      <!-- 动画设置：总开关 + 速率 -->
      <section class="settings-group" v-if="shouldShowGroup('animationSpeed') && inCategory('animationSpeed')">
        <h3 class="group-title">{{ t('settings.groups.animation') }}</h3>
          <div class="setting-item" v-if="shouldShowItem('animationEnabled')">
            <div class="item-icon">
              <i class="ri-magic-line" aria-hidden="true"></i>
            </div>
            <div class="item-info">
              <div class="label">{{ t('settings.animationEnabled.label') }}</div>
              <div class="desc">{{ t('settings.animationEnabled.desc') }}</div>
            </div>
            <div class="item-action">
              <div class="segmented">
                <button
                  v-for="opt in animationEnabledOptions"
                  :key="opt.value"
                  class="seg-btn"
                  :class="{ active: animationEnabled === opt.value }"
                  @click="animationEnabled = opt.value"
                >{{ t(opt.labelKey) }}</button>
              </div>
            </div>
          </div>
          <div
            class="setting-item"
            v-if="shouldShowItem('animationSpeed')"
            :class="{ 'is-disabled': animationEnabled === 'off' }"
          >
            <div class="item-icon">
              <i class="ri-speed-up-line" aria-hidden="true"></i>
            </div>
            <div class="item-info">
              <div class="label">{{ t('settings.animationSpeed.label') }}</div>
              <div class="desc">{{ t('settings.animationSpeed.desc') }}</div>
            </div>
            <div class="item-action">
              <div class="segmented" :class="{ disabled: animationEnabled === 'off' }">
                <button
                  v-for="opt in animationSpeedOptions"
                  :key="opt.value"
                  class="seg-btn"
                  :class="{ active: animationSpeed === opt.value }"
                  :disabled="animationEnabled === 'off'"
                  @click="animationSpeed = opt.value"
                >{{ t(opt.labelKey) }}</button>
              </div>
            </div>
          </div>
      </section>

      <!-- 关闭行为设置已移除：关闭窗口现在始终直接退出应用 -->

      <!-- 高级设置 (工厂重置) -->
      <section class="settings-group" v-if="shouldShowGroup('factoryReset') && inCategory('factoryReset')">
        <h3 class="group-title">{{ t('settings.factoryReset.groupTitle') }}</h3>
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
      </section>

      <!-- 版本设置 (新增) -->
      <section class="settings-group" v-if="shouldShowGroup('version') && inCategory('version')">
        <h3 class="group-title">{{ t('settings.groups.version') }}</h3>
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
                <button class="seg-btn" :class="{ active: updateChannel === 'both' }" @click="changeChannel('both')">{{ t('settings.updateChannel.both') }}</button>
              </div>
            </div>
          </div>
          <div class="setting-item" v-if="shouldShowItem('autoCheckUpdate')">
            <div class="item-icon">
              <i class="ri-refresh-line" aria-hidden="true"></i>
            </div>
            <div class="item-info">
              <div class="label">{{ t('settings.autoCheckUpdate.label') }}</div>
              <div class="desc">{{ t('settings.autoCheckUpdate.desc') }}</div>
            </div>
            <div class="item-action">
              <label class="switch">
                <input type="checkbox" v-model="autoCheckUpdate" @change="saveAutoCheckUpdate" />
                <span class="slider round"></span>
              </label>
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
            <div class="item-meta">
              <span class="meta-value">v{{ displayVersion }}</span>
              <span v-if="appBuildNumber && appBuildNumber !== '?'" class="meta-sub">+{{ appBuildNumber }}</span>
            </div>
            <div class="item-arrow">→</div>
          </div>
          <div class="setting-item clickable" @click="showAuthors = true" v-if="shouldShowItem('authors')">
            <div class="item-icon">
              <i class="ri-team-line" aria-hidden="true"></i>
            </div>
            <div class="item-info">
              <div class="label">{{ t('settings.versionInfo.authorsLabel') }}</div>
              <div class="desc">{{ t('settings.versionInfo.authorsDesc') }}</div>
            </div>
            <div class="item-meta">
              <span class="meta-value">{{ t('settings.versionInfo.author0Name') }}</span>
              <span class="meta-sub">{{ t('settings.versionInfo.authorsCount', { count: 4 }) }}</span>
            </div>
            <div class="item-arrow">→</div>
          </div>
          <div class="setting-item clickable" @click="openLegalSidebar" v-if="shouldShowItem('legal')">
            <div class="item-icon">
              <i class="ri-book-open-line" aria-hidden="true"></i>
            </div>
            <div class="item-info">
              <div class="label">{{ t('settings.legal.label') }}</div>
              <div class="desc">{{ t('settings.legal.desc') }}</div>
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
                <template v-else>{{ t('settings.checkUpdate.desc', { version: displayVersion }) }}</template>
              </div>
            </div>
            <div class="item-arrow">→</div>
          </div>
      </section>


    <!-- 版本信息 / 作者 / 法律 / 设置弹窗：独立组件 -->
    <VersionInfoDialog
      v-model="showVersionInfo"
      :version="displayVersion"
      :build="appBuildNumber"
      :is-beta="appIsBeta"
      :dev-hint="devHint"
      @tap-version="onVersionTap"
    />
    <AuthorsDialog v-model="showAuthors" />
    <LegalSidebar v-model="showLegalSidebar" />
    <DevUnlockDialog v-model="showDevUnlockDialog" @unlocked="onDevUnlocked" />
    <OutputPathDialog
      v-model="showOutputDialog"
      :path="outputPath"
      :mode="outputMode"
      @save="onSaveOutputPath"
    />
    <NamingDialog v-model="showNamingDialog" :template="namingTemplate" @save="onSaveNaming" />
    <HistoryDialog v-model="showHistoryDialog" />
    <BackgroundDialog
      v-model="showBackgroundDialog"
      :path="currentBackgroundPath"
      :fit="currentBackgroundFit"
      :opacity="currentBackgroundOpacity"
      @applied="onBackgroundApplied"
      @removed="onBackgroundRemoved"
    />
    <ThemeDialog
      v-model="showThemeDialog"
      :color="themeColor"
      :default-color="defaultThemeColor"
      show-reset
      :start-reset="themeStartReset"
      @confirm="onThemeConfirm"
      @reset="onThemeReset"
    />
    <transition name="dev-group">
      <section class="settings-group dev-group" v-if="devModeEnabled && shouldShowGroup('dev') && inCategory('dev')">
        <div class="dev-group-head">
          <h3 class="group-title dev-title">
            <i class="ri-code-box-line" aria-hidden="true"></i>
            {{ t('settings.groups.dev') }}
          </h3>
          <span class="dev-badge">DEV</span>
        </div>
        <p class="dev-sub">{{ t('settings.devMode.groupDesc') }}</p>

        <div class="dev-live-strip" v-if="shouldShowItem('devActionMonitor')">
          <div class="live-left">
            <span class="live-dot" :class="{ on: actionMonitorEnabled }" aria-hidden="true"></span>
            <div class="live-text">
              <b>{{ t('settings.devMode.actionMonitor') }}</b>
              <span>{{ actionMonitorEnabled ? t('settings.devMode.monitorOn') : t('settings.devMode.monitorOff') }}</span>
            </div>
          </div>
          <div class="live-stats">
            <span class="stat">
              <i class="ri-stack-line" aria-hidden="true"></i>
              {{ actionFrameCount }}
            </span>
            <span class="stat port" :title="t('settings.devMode.livePortHint')">
              <i class="ri-wifi-line" aria-hidden="true"></i>
              :{{ actionLivePort }}
            </span>
            <label class="switch">
              <input type="checkbox" v-model="actionMonitorEnabled" />
              <span class="slider"></span>
            </label>
          </div>
        </div>

          <div class="setting-item clickable" v-if="shouldShowItem('devAnalyzePack')" @click="analyzePack">
            <div class="item-icon">
              <i class="ri-survey-line" aria-hidden="true"></i>
            </div>
            <div class="item-info">
              <div class="label">{{ t('settings.devMode.analyzePack') }}</div>
              <div class="desc">{{ t('settings.devMode.analyzePackDesc') }}</div>
            </div>
            <div class="item-arrow">→</div>
          </div>
          <div class="setting-item clickable" v-if="shouldShowItem('devLog')" @click="showLogWindow = true">
            <div class="item-icon">
              <i class="ri-terminal-box-line" aria-hidden="true"></i>
            </div>
            <div class="item-info">
              <div class="label">{{ t('settings.devMode.logWindowTitle') }}</div>
              <div class="desc">{{ t('settings.devMode.viewLog') }}</div>
            </div>
            <div class="item-arrow">→</div>
          </div>
          <div class="setting-item clickable" v-if="shouldShowItem('devExportAmr')" @click="exportActionRecords">
            <div class="item-icon">
              <i class="ri-download-2-line" aria-hidden="true"></i>
            </div>
            <div class="item-info">
              <div class="label">{{ t('settings.devMode.exportActions') }}</div>
              <div class="desc">{{ t('settings.devMode.exportActionsDesc') }}</div>
            </div>
            <div class="item-meta">
              <span v-if="actionFrameCount > 0" class="meta-value">{{ actionFrameCount }}</span>
            </div>
            <div class="item-arrow">→</div>
          </div>
          <div class="setting-item clickable" v-if="shouldShowItem('devClearActions')" @click="clearActionRecords" :class="{ 'is-disabled': actionFrameCount === 0 }">
            <div class="item-icon">
              <i class="ri-eraser-line" aria-hidden="true"></i>
            </div>
            <div class="item-info">
              <div class="label">{{ t('settings.devMode.clearActions') }}</div>
              <div class="desc">{{ t('settings.devMode.clearActionsDesc') }}</div>
            </div>
            <div class="item-arrow">→</div>
          </div>
          <div class="setting-item" v-if="shouldShowItem('devLogRedaction')">
            <div class="item-icon">
              <i class="ri-shield-keyhole-line" aria-hidden="true"></i>
            </div>
            <div class="item-info">
              <div class="label">{{ t('settings.devMode.logRedaction') }}</div>
              <div class="desc">
                {{ logRedaction ? t('settings.devMode.logRedactionOn') : t('settings.devMode.logRedactionOff') }}
              </div>
            </div>
            <div class="item-action">
              <label class="switch">
                <input type="checkbox" v-model="logRedaction" />
                <span class="slider"></span>
              </label>
            </div>
          </div>
          <div class="setting-item clickable danger" v-if="shouldShowItem('devClearConfig')" @click="showClearConfigDialog = true">
            <div class="item-icon">
              <i class="ri-delete-bin-line" aria-hidden="true"></i>
            </div>
            <div class="item-info">
              <div class="label">{{ t('settings.devMode.clearConfig') }}</div>
              <div class="desc">{{ t('settings.devMode.clearConfigDesc') }}</div>
            </div>
            <div class="item-arrow">→</div>
          </div>
      </section>
    </transition>

      </div>
    </Workbench>

    <ClearConfigDialog v-model="showClearConfigDialog" />
    <FactoryResetDialog v-model="showFactoryResetDialog" @reset="emit('reset-to-oobe')" />
    <LogWindowDialog v-model="showLogWindow" />
    <PackAnalysisDialog v-model="showAnalysisDialog" :analysis="packAnalysis" />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch, onUnmounted } from 'vue';
import { useI18n } from 'vue-i18n'
import { pickFiles, pickSavePath, isInAppPickerEnabled, setInAppPickerEnabled } from '../composables/useFilePicker';
import { invoke } from '@tauri-apps/api/core';
import { getVersion } from '@tauri-apps/api/app';
import { useUpdater } from '../composables/useUpdater';
import { useNotification, type NotificationMode } from '../composables/useNotification';
import { useLanguage } from '../composables/useLanguage';
import { useAppInfo } from '../composables/useAppInfo';
import AuthorsDialog from './AuthorsDialog.vue';
import LegalSidebar from './LegalSidebar.vue';
import VersionInfoDialog from './VersionInfoDialog.vue';
import NamingDialog from './settings/NamingDialog.vue';
import HistoryDialog from './settings/HistoryDialog.vue';
import ThemeDialog from './settings/ThemeDialog.vue';
import BackgroundDialog from './settings/BackgroundDialog.vue';
import LogWindowDialog from './settings/LogWindowDialog.vue';
import PackAnalysisDialog, { type PackAnalysis } from './settings/PackAnalysisDialog.vue';
import FactoryResetDialog from './settings/FactoryResetDialog.vue';
import ClearConfigDialog from './settings/ClearConfigDialog.vue';
import OutputPathDialog from './settings/OutputPathDialog.vue';
import DevUnlockDialog from './settings/DevUnlockDialog.vue';
import Workbench, { type WorkbenchStep } from './shell/Workbench.vue';
import StageBackdrop from './shell/StageBackdrop.vue';
const { t } = useI18n()
const { locale, setLanguage } = useLanguage()

const props = defineProps<{
  devMode?: boolean;
  userName?: string;
  sourceHandling?: 'ask' | 'delete' | 'keep';
  openOutputAfterConvert?: boolean;
}>();
const emit = defineEmits([
    'update:editorMode',
  'switch-page',
  'update:dev-mode',
  'update:user-name',
  'update:animation-speed',
  'update:animation-enabled',
  'update:source-handling',
  'update:open-output-after-convert',
  'update:action-monitor',
  'update:background',
  'update:ui-style',
  'show-update-dialog',
  'reset-to-oobe',
]);

const { notify, setNotificationEnabled, setNotificationMode, setToastDuration } = useNotification();
const { version: appInfoVersion, build: appBuildNumber, isBeta: appIsBeta } = useAppInfo();

const outputMode = ref<'follow' | 'fixed'>('follow');
const outputPath = ref('C:/Users/Admin/Documents/2-Pyramid/Output');
const showOutputDialog = ref(false);
const onSaveOutputPath = async (path: string) => {
  outputPath.value = path;
  try {
    await invoke('update_config', {
      patch: { outputMode: outputMode.value, outputPath: path },
    });
  } catch (e) {
    console.error('update_config failed', e);
  }
};
const notificationEnabled = ref(true);
const notificationMode = ref<NotificationMode>('app');
type AnimationSpeed = 'slow' | 'normal' | 'fast';
type AnimationEnabled = 'on' | 'off' | 'system';
const animationSpeed = ref<AnimationSpeed>('normal');
const animationEnabled = ref<AnimationEnabled>('system');
const animationSpeedOptions: { value: AnimationSpeed; labelKey: string }[] = [
  { value: 'slow', labelKey: 'settings.animationSpeed.slow' },
  { value: 'normal', labelKey: 'settings.animationSpeed.normal' },
  { value: 'fast', labelKey: 'settings.animationSpeed.fast' },
];
const animationEnabledOptions: { value: AnimationEnabled; labelKey: string }[] = [
  { value: 'on', labelKey: 'settings.animationEnabled.on' },
  { value: 'system', labelKey: 'settings.animationEnabled.system' },
  { value: 'off', labelKey: 'settings.animationEnabled.off' },
];
const sourceHandling = ref<'ask' | 'delete' | 'keep'>(props.sourceHandling ?? 'ask');
const openOutputAfterConvert = ref<boolean>(props.openOutputAfterConvert ?? true);
const toastDuration = ref(8000);
const toastDurationOptions = [4000, 6000, 8000, 10000, 12000];
const toastPosition = ref<'top-left' | 'top-right' | 'bottom-left' | 'bottom-right'>('top-right');
const conversionThreads = ref(2);
// 性能档位：平衡 / 性能（后端解析成线程预算与并发包数）
const performanceMode = ref<'balanced' | 'performance'>('balanced');
interface PerfPlan {
  mode: string;
  cores: number;
  threads: number;
  packs: number;
  availableMemoryMb: number | null;
  note: string | null;
}
const perfPlan = ref<PerfPlan | null>(null);

async function refreshPerfPlan() {
  try {
    perfPlan.value = await invoke<PerfPlan>('get_perf_plan');
  } catch {
    perfPlan.value = null;
  }
}
watch(performanceMode, async (val) => {
  invoke('update_config', { patch: { performanceMode: val } }).catch(() => {});
  await refreshPerfPlan();
});
const namingTemplate = ref('[Ver][Name]');
const showNamingDialog = ref(false);
const onSaveNaming = (value: string) => {
  namingTemplate.value = value;
};
const showHistoryDialog = ref(false);
const showThemeDialog = ref(false);
const themeStartReset = ref(false);
const showClearConfigDialog = ref(false);
const showFactoryResetDialog = ref(false);
const actionMonitorEnabled = ref(false);
const actionFrameCount = ref(0);
const actionLivePort = ref(24159);
let actionStatusTimer: ReturnType<typeof setInterval> | null = null;

async function refreshActionStatus() {
  try {
    const st = await invoke<{ enabled: boolean; frames: number; livePort?: number }>('action_monitor_status');
    actionFrameCount.value = st.frames ?? 0;
    if (typeof st.livePort === 'number') actionLivePort.value = st.livePort;
    if (typeof st.enabled === 'boolean' && st.enabled !== actionMonitorEnabled.value) {
      actionMonitorEnabled.value = st.enabled;
    }
  } catch { /* ignore */ }
}

const clearActionRecords = async () => {
  try {
    const dropped = await invoke<number>('clear_action_records');
    actionFrameCount.value = 0;
    await notify({
      title: t('settings.devMode.clearActions'),
      body: t('settings.devMode.clearActionsDone', { count: dropped }),
      type: 'success',
      source: 'system',
    });
  } catch { /* ignore */ }
};

const showBackgroundDialog = ref(false);
const currentBackgroundPath = ref<string | null>(null);
const currentBackgroundFit = ref<'cover' | 'contain' | 'stretch' | 'tile'>('cover');
const currentBackgroundOpacity = ref(1);
const uiStyle = ref<'glass' | 'frosted'>('glass');

const openBackgroundDialog = () => {
  showBackgroundDialog.value = true;
};

const onBackgroundApplied = (payload: {
  path: string | null;
  fit: 'cover' | 'contain' | 'stretch' | 'tile';
  opacity: number;
  themeColor: string | null;
}) => {
  currentBackgroundPath.value = payload.path;
  currentBackgroundFit.value = payload.fit;
  currentBackgroundOpacity.value = payload.opacity;
  if (payload.themeColor) {
    themeColor.value = payload.themeColor;
  }
  emit('update:background', {
    path: payload.path,
    fit: payload.fit,
    opacity: payload.opacity,
    themeColor: payload.themeColor,
  });
};

const onBackgroundRemoved = () => {
  currentBackgroundPath.value = null;
  emit('update:background', { path: null, fit: 'cover', opacity: 1, themeColor: null });
};

const defaultThemeColor = 'var(--ink)';
const themeColor = ref('var(--ink)');

/**
 * 实验开关：应用内文件浏览器。
 *
 * 状态存在 composable 里而不是这里 —— useFilePicker 每次调用都要读它，
 * 而设置页可能压根没挂载。这里只是一个双向绑定的视图。
 */
const inAppPickerOn = computed({
  get: () => isInAppPickerEnabled().value,
  set: (v: boolean) => setInAppPickerEnabled(v),
});

const localUserName = ref(props.userName || '');
const searchQuery = ref('');
const showVersionInfo = ref(false);
const showAuthors = ref(false);
const showLegalSidebar = ref(false);

function openLegalSidebar() {
  showLegalSidebar.value = true;
}

const devModeEnabled = ref(!!props.devMode);
const editorMode = ref(localStorage.getItem('editorMode') === 'true');
const aiBaseUrl = ref('https://api.openai.com/v1');
const aiApiKey = ref('');
const aiModel = ref('gpt-4o-mini');
const aiTier = ref(1);
const aiPrompt = ref('');
const aiMsg = ref('');

async function loadAiFromBackend() {
  try {
    const c = await invoke<any>('foray_ai_config_get');
    if (c?.base_url) aiBaseUrl.value = c.base_url;
    if (c?.api_key) aiApiKey.value = c.api_key;
    if (c?.model) aiModel.value = c.model;
    if (typeof c?.default_tier === 'number') aiTier.value = c.default_tier;
    if (typeof c?.system_prompt === 'string') aiPrompt.value = c.system_prompt;
  } catch { /* ignore */ }
}

async function saveAiToBackend() {
  try {
    await invoke('foray_ai_config_set', {
      config: {
        base_url: aiBaseUrl.value,
        api_key: aiApiKey.value,
        model: aiModel.value,
        default_tier: aiTier.value,
        system_prompt: aiPrompt.value,
      },
    });
    aiMsg.value = t('settings.forayAi.saved');
  } catch (e: any) {
    aiMsg.value = String(e);
  }
}

function restoreAiPrompt() {
  aiPrompt.value = '';
  aiMsg.value = t('settings.forayAi.promptRestored');
}

async function testAiConnection() {
  await saveAiToBackend();
  try {
    const r = await invoke<string>('foray_ai_test');
    aiMsg.value = t('settings.forayAi.connOk', { msg: String(r).slice(0, 80) });
  } catch (e: any) {
    aiMsg.value = t('settings.forayAi.connFailed', { msg: String(e).slice(0, 160) });
  }
}

function onEditorModeChange(e: Event) {
  const on = (e.target as HTMLInputElement).checked;
  editorMode.value = on;
  localStorage.setItem('editorMode', String(on));
  emit('update:editorMode', on);
  if (on) void loadAiFromBackend();
}
const versionTapCount = ref(0);
const devHint = ref('');
const showDevUnlockDialog = ref(false);
const showLogWindow = ref(false);
// 资源包结构分析（多版本包调研用）
const showAnalysisDialog = ref(false);
const packAnalysis = ref<PackAnalysis | null>(null);

async function analyzePack() {
  try {
    const picked = await pickFiles({ extensions: ['zip', 'mcpack'] });
    if (!picked || typeof picked !== 'string') return;
    packAnalysis.value = await invoke<PackAnalysis>('analyze_pack', { path: picked });
    showAnalysisDialog.value = true;
  } catch (e) {
    notify({
      title: t('common.error'),
      body: t('settings.devMode.analyzePackFailed', { error: String(e) }),
      type: 'error',
      source: 'system',
    });
  }
}
// 导出日志脱敏（默认开启；仅影响导出，磁盘与界面日志保持原文）
const logRedaction = ref(true);
watch(logRedaction, (val) => {
  invoke('update_config', { patch: { logRedaction: val } }).catch(() => {});
});

const onDevUnlocked = () => {
  devModeEnabled.value = true;
  emit('update:dev-mode', true);
};

// ── Updater ──────────────────────────────────────
const { checkForUpdate, getChannel } = useUpdater();
const updateChannel = ref('master');
const autoCheckUpdate = ref(true);
const updateChecking = ref(false);
const updateError = ref('');
const currentVersion = ref('');
const displayVersion = computed(() => {
  const v = appInfoVersion.value && appInfoVersion.value !== '0.0.0'
    ? appInfoVersion.value
    : currentVersion.value;
  return v || '—';
});

async function loadUpdateChannel() {
  try {
    updateChannel.value = await getChannel();
  } catch { /* use default */ }
  try {
    const cfg = await invoke<any>('get_config');
    if (typeof cfg?.auto_check_update === 'boolean') autoCheckUpdate.value = cfg.auto_check_update;
    if (typeof cfg?.log_redaction === 'boolean') logRedaction.value = cfg.log_redaction;
  } catch { /* default true */ }
}

async function saveAutoCheckUpdate() {
  try {
    await invoke('update_config', { patch: { autoCheckUpdate: autoCheckUpdate.value } });
  } catch (e) {
    console.error('save auto_check_update failed', e);
  }
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
  { id: 'inAppPicker', group: 'global', label: t('settings.inAppPicker.label'), desc: t('settings.inAppPicker.desc') },
  { id: 'outputMode', group: 'convert', label: t('settings.outputMode.label'), desc: t('settings.outputMode.desc') },
  { id: 'notification', group: 'notification', label: t('settings.notification.label'), desc: t('settings.notification.desc') },
  { id: 'notificationMode', group: 'notification', label: t('settings.notificationMode.label'), desc: t('settings.notificationMode.desc') },
  { id: 'testNotification', group: 'notification', label: t('settings.testNotification.label'), desc: t('settings.testNotification.desc') },
  { id: 'toastDuration', group: 'notification', label: t('settings.toastDuration.label'), desc: t('settings.toastDuration.desc') },
  { id: 'toastPosition', group: 'notification', label: t('settings.toastPosition.label'), desc: t('settings.toastPosition.desc') },
  { id: 'conversionThreads', group: 'convert', label: t('settings.performanceMode.label'), desc: t('settings.performanceMode.balancedDesc') },
  { id: 'outputNaming', group: 'convert', label: t('settings.outputNaming.label'), desc: t('settings.outputNaming.desc') },
  { id: 'conversionHistory', group: 'conversionHistory', label: t('settings.conversionHistory.label'), desc: t('settings.conversionHistory.desc') },
  { id: 'channel', group: 'version', label: t('settings.updateChannel.label'), desc: t('settings.updateChannel.desc') },
  { id: 'autoCheckUpdate', group: 'version', label: t('settings.autoCheckUpdate.label'), desc: t('settings.autoCheckUpdate.desc') },
  { id: 'animationEnabled', group: 'animationSpeed', label: t('settings.animationEnabled.label'), desc: t('settings.animationEnabled.desc') },
  { id: 'animationSpeed', group: 'animationSpeed', label: t('settings.animationSpeed.label'), desc: t('settings.animationSpeed.desc') },
  { id: 'versionInfo', group: 'version', label: t('settings.versionInfo.label'), desc: t('settings.versionInfo.desc') },
  { id: 'authors', group: 'version', label: t('settings.versionInfo.authorsLabel'), desc: t('settings.versionInfo.authorsDesc') },
  { id: 'legal', group: 'version', label: t('settings.legal.label'), desc: t('settings.legal.desc') },
  { id: 'update', group: 'version', label: t('settings.checkUpdate.label'), desc: t('settings.checkUpdate.searchDesc') },
  { id: 'devLog', group: 'dev', label: t('settings.devMode.logWindowTitle'), desc: t('settings.devMode.viewLog') },
  { id: 'devAnalyzePack', group: 'dev', label: t('settings.devMode.analyzePack'), desc: t('settings.devMode.analyzePackDesc') },
  { id: 'devLogRedaction', group: 'dev', label: t('settings.devMode.logRedaction'), desc: t('settings.devMode.logRedactionSearchDesc') },
  { id: 'devExportAmr', group: 'dev', label: t('settings.devMode.exportActions'), desc: t('settings.devMode.exportActionsDesc') },
  { id: 'devClearActions', group: 'dev', label: t('settings.devMode.clearActions'), desc: t('settings.devMode.clearActionsDesc') },
  { id: 'devClearConfig', group: 'dev', label: t('settings.devMode.clearConfig'), desc: t('settings.devMode.clearConfigDesc') },
  { id: 'devActionMonitor', group: 'dev', label: t('settings.devMode.actionMonitor'), desc: t('settings.devMode.actionMonitorDesc') }
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

/**
 * ── 设置分类（常驻导航 + 内容井）────────────────────────────────────────
 *
 * 原来是 27 行平铺在一个长滚动里，只能靠小标题分段、靠滚动找。
 * 现在左侧常驻分类轨、右侧只呈现当前分类 —— 与转换页共用同一套外壳
 * （<Workbench> 的 rail 支持 icon，所以分类不会显示成「第 1 步 / 第 2 步」）。
 *
 * ⚠️ 分类只是**浏览**维度，不参与搜索：
 * 一旦搜索框有内容，inCategory 放行全部分组 —— 否则跨分类的命中会被
 * 当前分类过滤掉，用户搜到了东西却看不见。
 */
const CATEGORIES = [
  { id: 'general',    icon: 'ri-user-settings-line',  groups: ['language', 'personal'] },
  { id: 'appearance', icon: 'ri-palette-line',        groups: ['global'] },
  { id: 'convert',    icon: 'ri-swap-box-line',       groups: ['convert'] },
  { id: 'notify',     icon: 'ri-notification-3-line', groups: ['notification'] },
  // remixicon 里既没有 ri-animation-line 也没有 ri-motion-line —— 写错不会报错，
  // 只会渲染出 0×0、content:none 的空 <i>，不实测量就永远发现不了。
  // 这个类名是对着 remixicon.css 里的 ::before 定义逐个核过的。
  { id: 'motion',     icon: 'ri-shining-line',        groups: ['animationSpeed'] },
  { id: 'update',     icon: 'ri-information-line',    groups: ['version'] },
  { id: 'records',    icon: 'ri-history-line',        groups: ['conversionHistory', 'factoryReset'] },
  { id: 'dev',        icon: 'ri-terminal-box-line',   groups: ['dev'] },
] as const;

const activeCategory = ref<string>('general');
const selectCategory = (id: string) => { activeCategory.value = id; };

/** 该分组是否属于当前分类。搜索中一律放行。 */
const inCategory = (groupId: string) => {
  if (searchQuery.value) return true;
  const cat = CATEGORIES.find((c) => c.id === activeCategory.value);
  return !!cat && (cat.groups as readonly string[]).includes(groupId);
};

const isSearching = computed(() => searchQuery.value.trim().length > 0);

const categories = computed<WorkbenchStep[]>(() =>
  CATEGORIES
    // 开发者分类只在开发者模式下出现，避免普通用户看到一个空分类
    .filter((c) => c.id !== 'dev' || devModeEnabled.value)
    .map((c) => ({
      id: c.id,
      label: t(`settings.categories.${c.id}`),
      icon: c.icon,
      // 搜索中不标任何分类为当前 —— 此时右侧显示的是跨分类的搜索结果
      state: c.id === activeCategory.value && !isSearching.value ? 'current' : 'todo',
    }))
);

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
    source: 'system',
    ignoreDisabled: true,
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

const exportActionRecords = async () => {
  try {
    const stamp = new Date()
      .toISOString()
      .replace(/[-:T]/g, '')
      .slice(0, 14);
    const dest = await pickSavePath({
      extensions: ['2amr'],
      defaultName: `2pyramid-actions-${stamp}.2amr`,
    });
    if (!dest) return;
    const count = await invoke<number>('export_action_records', { dest });
    await notify({
      title: t('settings.devMode.exportActions'),
      body: t('settings.devMode.exportActionsSuccess', { count, path: dest }),
      type: 'success',
      source: 'system',
    });
  } catch (e) {
    await notify({
      title: t('settings.devMode.exportActions'),
      body: t('settings.devMode.exportActionsFailed', { error: String(e) }),
      type: 'error',
      source: 'system',
    });
  }
};

onMounted(() => {
  if (editorMode.value) void loadAiFromBackend();
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

  const savedAnimationEnabled = localStorage.getItem('animationEnabled');
  if (savedAnimationEnabled === 'on' || savedAnimationEnabled === 'off' || savedAnimationEnabled === 'system') {
    animationEnabled.value = savedAnimationEnabled;
  }
  emit('update:animation-enabled', animationEnabled.value);

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
      }
      if (typeof cfg?.notification_enabled === 'boolean') {
        notificationEnabled.value = cfg.notification_enabled;
      }
      if (cfg?.notification_mode === 'system' || cfg?.notification_mode === 'app' || cfg?.notification_mode === 'both') {
        notificationMode.value = cfg.notification_mode;
      }
      setNotificationEnabled(notificationEnabled.value);
      setNotificationMode(notificationMode.value);
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
      if (cfg?.performance_mode === 'balanced' || cfg?.performance_mode === 'performance') {
        performanceMode.value = cfg.performance_mode;
      } else if (typeof cfg?.conversion_threads === 'number') {
        // 旧配置迁移：≤2 → 平衡，≥3 → 性能
        conversionThreads.value = cfg.conversion_threads;
        performanceMode.value = cfg.conversion_threads >= 3 ? 'performance' : 'balanced';
      }
      void refreshPerfPlan();
      if (typeof cfg?.output_naming === 'string' && cfg.output_naming.length > 0) {
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
  if (actionStatusTimer) {
    clearInterval(actionStatusTimer);
    actionStatusTimer = null;
  }
  if (val) {
    void refreshActionStatus();
    actionStatusTimer = setInterval(() => { void refreshActionStatus(); }, 1500);
  }
});

watch(showVersionInfo, (open) => {
  if (open) return;
  versionTapCount.value = 0;
  devHint.value = '';
});

onUnmounted(() => {
  if (actionStatusTimer) {
    clearInterval(actionStatusTimer);
    actionStatusTimer = null;
  }
});

watch(() => props.devMode, (v) => {
  if (typeof v === 'boolean') devModeEnabled.value = v;
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

watch(animationEnabled, (val) => {
  localStorage.setItem('animationEnabled', val);
  emit('update:animation-enabled', val);
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
  themeStartReset.value = false;
  showThemeDialog.value = true;
};

const openThemeReset = () => {
  themeStartReset.value = true;
  showThemeDialog.value = true;
};

const onThemeConfirm = async (color: string) => {
  themeColor.value = color;
  document.documentElement.style.setProperty('--theme-color', color);
  localStorage.setItem('themeColor', color);
  try {
    await invoke('update_config', { patch: { palette: { theme_color: color } } });
  } catch (e) {
    console.error('update_config failed', e);
  }
};

const onThemeReset = async () => {
  themeColor.value = defaultThemeColor;
  document.documentElement.style.setProperty('--theme-color', themeColor.value);
  localStorage.setItem('themeColor', themeColor.value);
  try {
    await invoke('update_config', { patch: { palette: { theme_color: themeColor.value } } });
  } catch (e) {
    console.error('update_config failed', e);
  }
};
</script>

<style scoped>
.fanhua-settings {
  width: 100%; height: 100%; min-height: 0;
  /* Background gradient + aurora ::before are provided by App.vue's
     `.page-shell > *` rule so all three pages render identically. */
  color: var(--ink);
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
  color: var(--ink-4);
  font-size: 18px;
}

.search-input {
  width: 100%;
  padding: 10px 16px 10px 44px;
  border-radius: 8px;
  border: 1px solid rgba(var(--warm-rgb), 0.2);
  background: rgba(255, 255, 255, 0.72);
  font-size: 14px;
  transition: all 0.3s;
  outline: none;
}

.search-input:focus {
  border-color: var(--theme-color);
  background: var(--surface);
  box-shadow: 0 4px 12px rgba(var(--theme-color-rgb), 0.1);
}

.page-title {
  font-size: clamp(22px, 3.2vw, 30px);
  font-weight: 700;
  color: var(--ink);
  margin: 0;
  min-width: 0;
  max-width: 16ch;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.page-subtitle { margin: 6px 0 0; color: var(--ink-3); font-size: 13px; }
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
  background: rgba(var(--ink-rgb), 0.05);
  border: 1px solid rgba(var(--ink-rgb), 0.05);
  color: var(--ink);
  margin: 10px 0 12px;
  user-select: text;
}

.dev-error {
  margin-top: 10px;
  font-size: 12px;
  font-weight: 700;
  color: var(--danger);
}
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
  
}

@media (max-width: 520px) {
  .back-btn span {
    display: none;
  }
}

/* 原来这里是 padding: 0 48px —— 那是「整宽滚动区」时代给内容留的呼吸边距。\n   现在左侧已有轨道 + 20px 间距，再内缩 48px 会让卡片右缘离窗口 92px、\n   而轨道左缘只有 40px，左右明显不对称。改为贴齐舞台边缘。 */\n.settings-group { margin-top: 30px; width: 100%; }
.group-title {
  /* 首页的层级靠字号与字距，不靠字重 —— 这里也照做 */
  font-size: 10.5px; font-weight: 400; color: var(--ink-3);
  margin-bottom: 10px; text-transform: uppercase; letter-spacing: 0.18em;
}

/* 开发者选项分组：启用时与其他分组一样平滑淡入（不突兀出现） */
.dev-group-enter-active {
  transition: opacity 0.35s ease, transform 0.35s cubic-bezier(0.22, 1, 0.36, 1);
}
.dev-group-leave-active {
  transition: opacity 0.2s ease;
}
.dev-group-enter-from {
  opacity: 0;
  transform: translateY(12px);
}
.dev-group-leave-to {
  opacity: 0;
}

/* 开发者区块：深色玻璃 + 绿点缀，与主 UI 区分 */
.dev-group {
  position: relative;
}

.dev-group-head {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-left: 15px;
  margin-bottom: 4px;
}

.dev-title {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  margin: 0;
  color: var(--ink);
  text-transform: none;
  letter-spacing: 0;
  font-size: 14px;
}

.dev-title i {
  font-size: 18px;
  color: var(--ok);
}

.dev-badge {
  display: inline-flex;
  align-items: center;
  padding: 2px 8px;
  border-radius: 999px;
  font-size: 10px;
  font-weight: 800;
  letter-spacing: 0.06em;
  background: rgba(5, 150, 105, 0.12);
  color: var(--ok);
  border: 1px solid rgba(5, 150, 105, 0.22);
}

.dev-sub {
  margin: 0 0 12px 15px;
  font-size: 12px;
  color: var(--ink-3);
  line-height: 1.5;
}

.dev-live-strip {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 12px;
  padding: 12px 16px;
  border-radius: 16px;
  background: linear-gradient(135deg, rgba(6, 78, 59, 0.08), rgba(5, 150, 105, 0.06));
  border: 1px solid rgba(5, 150, 105, 0.14);
  box-shadow: 0 2px 12px rgba(6, 78, 59, 0.04);
}

.live-left {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
}

.live-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: var(--surface-6);
  flex-shrink: 0;
  box-shadow: 0 0 0 3px rgba(148, 163, 184, 0.15);
}

.live-dot.on {
  background: var(--ok);
  box-shadow: 0 0 0 3px rgba(16, 185, 129, 0.18), 0 0 10px rgba(16, 185, 129, 0.45);
  animation: dev-live-pulse 1.6s ease-in-out infinite;
}

@keyframes dev-live-pulse {
  50% { opacity: 0.55; }
}

.live-text {
  display: flex;
  flex-direction: column;
  gap: 1px;
  min-width: 0;
}

.live-text b {
  font-size: 13.5px;
  font-weight: 700;
  color: var(--ink);
}

.live-text span {
  font-size: 11.5px;
  color: var(--ink-3);
}

.live-stats {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-shrink: 0;
}

.live-stats .stat {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 4px 10px;
  border-radius: 999px;
  background: rgba(255, 255, 255, 0.72);
  border: 1px solid rgba(var(--ink-rgb), 0.05);
  font-size: 12px;
  font-weight: 700;
  color: var(--ink-2);
  font-variant-numeric: tabular-nums;
}

.live-stats .stat i {
  font-size: 13px;
  color: var(--ok);
}

.live-stats .stat.port {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  color: var(--ok);
}

/* 开发者卡片：略深、边缘绿描边 */
.dev-group .group-card {
  border-color: rgba(5, 150, 105, 0.12);
  background: rgba(255, 255, 255, 0.68);
}

.dev-group .setting-item.clickable.danger:hover {
  background: rgba(239, 68, 68, 0.05);
}

.group-card {
    /* 这个类已经在模板里去掉了 —— 内容层不再用卡片包裹。
     保留一个空规则会让后来人以为卡片还存在，所以整条删除。 */
}

.setting-item {
  display: flex; align-items: center; padding: 14px 4px;
  /* 只在下方画线：相邻两行共用一条线，不会出现双线 */
  border-bottom: 1px solid rgba(var(--warm-rgb), 0.14);
  transition: background-color 0.2s ease;
}
.setting-item:last-child { border-bottom: none; }
.setting-item.clickable { cursor: pointer; }
.setting-item.clickable:hover { background: rgba(var(--ink-rgb), 0.03); }
.setting-item.is-disabled { opacity: 0.45; }
.segmented.disabled { pointer-events: none; }

.item-icon { width: 40px; color: var(--ink); display: inline-flex; align-items: center; justify-content: center; }
.item-icon i { font-size: 20px; line-height: 1; }
.item-info { flex: 1; display: flex; flex-direction: column; gap: 2px; }
.item-info .label { font-size: 15px; font-weight: 600; color: var(--ink); }
.item-info .desc { font-size: 12px; color: var(--ink-3); }

.item-meta {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 2px;
  margin-right: 10px;
  flex-shrink: 0;
}
.meta-value {
  font-size: 13px;
  font-weight: 700;
  color: var(--ink);
  font-variant-numeric: tabular-nums;
}
.meta-sub {
  font-size: 11px;
  font-weight: 600;
  color: var(--ink-4);
  font-variant-numeric: tabular-nums;
}

.item-arrow { color: var(--surface-6); font-weight: 800; }

/* 性能档位：档位说明与解析后的线程/包数 */
.perf-detail {
  margin-top: 3px;
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 11.5px;
  color: var(--ink);
}
.perf-note { color: var(--warn); }

.fanhua-select {
  background: rgba(var(--ink-rgb), 0.05); border: none; padding: 6px 10px;
  border-radius: 8px; font-size: 13px; outline: none;
}

/* 设置页局部覆盖：主题色文字按钮（比全局 shared 的灰字更醒目） */
.btn-text {
  background: color-mix(in srgb, var(--theme-color) 12%, transparent);
  color: var(--theme-color);
}
.btn-text.secondary { background: rgba(var(--ink-rgb), 0.05); color: var(--ink-2); }
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
  border: 1px solid rgba(var(--ink-rgb), 0.08);
  background: rgba(var(--ink-rgb), 0.05);
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
  color: var(--ink-4);
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
  color: var(--ink);
}
.bg-range {
  flex: 1 1 auto;
  accent-color: var(--theme-color);
}
.bg-opacity-val {
  flex: 0 0 42px;
  text-align: right;
  font-size: 12px;
  color: var(--ink-3);
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
  color: var(--ink);
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
  border-top: 1px solid rgba(var(--ink-rgb), 0.05);
}
.history-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 4px;
  border-bottom: 1px solid rgba(var(--ink-rgb), 0.05);
}
.history-status {
  width: 8px; height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}
.history-status.ok { background: var(--ok); }
.history-status.fail { background: var(--danger); }
.history-status.cancelled { background: var(--ink-4); }
.history-info { flex: 1; min-width: 0; }
.history-name {
  font-size: 13px; font-weight: 600; color: var(--ink);
  white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
}
.history-meta { font-size: 12px; color: var(--ink-4); margin-top: 2px; }
.history-empty { padding: 20px 0; text-align: center; color: var(--ink-4); font-size: 13px; }

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
  border: 1.5px solid rgba(var(--ink-rgb), 0.12);
  border-radius: 10px;
  background: rgba(var(--ink-rgb), 0.05);
  color: var(--ink);
  outline: none;
}
.naming-input:focus { border-color: var(--theme-color); background: var(--surface); }
.naming-row {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}
.naming-hint-label { font-size: 12px; color: var(--ink-4); min-width: 76px; }
.naming-tag-btn {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
  padding: 6px 12px;
  border-radius: 8px;
  border: 1px dashed rgba(var(--ink-rgb), 0.18);
  background: transparent;
  cursor: pointer;
  transition: all 0.15s;
}
.naming-tag-btn:hover { border-color: var(--theme-color); background: color-mix(in srgb, var(--theme-color) 6%, transparent); }
.naming-tag-token {
  font-size: 12px;
  font-weight: 600;
  font-family: ui-monospace, Consolas, monospace;
  color: var(--ink-2);
}
.naming-tag-btn:hover .naming-tag-token { color: var(--theme-color); }
.naming-tag-desc {
  font-size: 10.5px;
  color: var(--ink-4);
  line-height: 1.2;
}
.naming-preview {
  font-size: 12.5px;
  color: var(--ink-3);
  padding: 8px 12px;
  border-radius: 10px;
  background: rgba(var(--ink-rgb), 0.03);
  word-break: break-all;
}
.naming-preview b { color: var(--ink); }

.inline-input {
  padding: 7px 14px;
  font-size: 13px;
  font-family: inherit;
  font-weight: 500;
  border: 1.5px solid rgba(var(--ink-rgb), 0.12);
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.8);
  color: var(--ink);
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
  color: var(--ink-4);
}

.item-action { display: flex; align-items: center; justify-content: flex-end; flex-shrink: 0; }
.switch-wrap { cursor: pointer; }
.foray-ai-form {
  width: 100%;
  box-sizing: border-box;
  padding: 4px 20px 18px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.foray-ai-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px 16px;
}
.foray-ai-grid label,
.foray-ai-form > label.wide {
  display: flex;
  flex-direction: column;
  gap: 6px;
  font-size: 12px;
  color: var(--ink-3);
  font-weight: 600;
}
.foray-ai-grid input,
.foray-ai-grid select,
.foray-ai-form textarea {
  width: 100%;
  box-sizing: border-box;
  padding: 10px 12px;
  border-radius: var(--ui-radius-btn, 10px);
  border: 1px solid rgba(var(--ink-rgb), 0.08);
  background: rgba(255, 255, 255, 0.8);
  color: var(--ink);
  font-size: 13px;
  font-family: inherit;
}
.foray-ai-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  align-items: center;
}
.foray-ai-form { display: flex; flex-direction: column; gap: 10px; padding: 12px 16px 16px; width: 100%; }
.foray-ai-form label { display: flex; flex-direction: column; gap: 6px; font-size: 12px; color: var(--ink-3); font-weight: 600; }
.foray-ai-form input, .foray-ai-form select, .foray-ai-form textarea { width: 100%; box-sizing: border-box; padding: 10px 12px; border-radius: var(--ui-radius-btn, 10px); border: 1px solid rgba(var(--ink-rgb), 0.08); background: rgba(255,255,255,0.8); color: var(--ink); font: inherit; font-weight: 500; }
.foray-ai-msg { margin: 8px 0 0; font-size: 12px; color: var(--theme-color, var(--ink)); font-weight: 600; }
.switch { position: relative; display: inline-block; width: 42px; height: 24px; }
.switch input { opacity: 0; width: 0; height: 0; }
.slider {
  position: absolute; inset: 0; cursor: pointer; background-color: var(--surface-5); transition: background-color 200ms cubic-bezier(0.4, 0, 0.2, 1); border-radius: 34px;
}
.slider:before {
  position: absolute; content: ""; height: 18px; width: 18px; left: 3px; bottom: 3px; background-color: white; transition: transform 200ms cubic-bezier(0.4, 0, 0.2, 1); border-radius: 50%;
  box-shadow: 0 2px 4px rgba(var(--ink-rgb), 0.12);
}
.switch input:checked + .slider { background-color: var(--ink); }
.switch input:checked + .slider:before { transform: translateX(18px); }
.switch input:disabled + .slider { opacity: 0.5; cursor: not-allowed; }
.switch.disabled { cursor: not-allowed; }

/*
 * 实验标签与免责说明。
 * 形状沿用 ConversionPage 的 .exp-tag（琥珀底），但字重按规格收到 600；
 * 免责那句用同一个琥珀色系的文字色，和普通说明区分开。
 */
.exp-tag {
  display: inline-block;
  margin-left: 7px;
  padding: 1px 6px;
  border-radius: 999px;
  font-size: 10px;
  font-weight: 600;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: var(--warn);
  background: var(--warn-tint);
  border: 1px solid var(--warn-line);
  vertical-align: 1px;
}
.setting-item .desc.warn {
  margin-top: 2px;
  color: var(--warn);
  line-height: 1.5;
}

/* Dev-only destructive action */
.setting-item.danger .item-icon { color: var(--danger); }
.setting-item.danger:hover { background: rgba(239, 68, 68, 0.06); }
.btn-text.danger { color: var(--danger); }
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
.status-loading      { background: var(--surface-3); color: var(--ink-3); }
.status-busy         { background: var(--surface-3); color: var(--ink); }
.status-registered   { background: rgba(16, 185, 129, 0.14); color: var(--ok); }
.status-unregistered { background: var(--warn-tint); color: var(--warn); }
.status-partial      { background: rgba(179, 38, 30, 0.16); color: var(--danger); }

/* 弹窗骨架（overlay/content/header/body/footer）走全局 shared.css */
.log-toolbar {
  display: flex;
  justify-content: flex-end;
  margin-bottom: 8px;
}
.log-output {
  height: 360px;
  overflow: auto;
  background: var(--ink);
  color: var(--ok);
  border-radius: 10px;
  padding: 12px;
  font-size: 12px;
  line-height: 1.45;
}

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
  background: rgba(var(--ink-rgb), 0.05);
  cursor: pointer;
  transition: background 0.15s;
}
.dialog-checkbox-row:hover { background: rgba(var(--ink-rgb), 0.08); }
.dialog-checkbox-row input[type="checkbox"] {
  margin-top: 2px;
  width: 16px; height: 16px;
  cursor: pointer;
  accent-color: var(--theme-color, var(--ink));
}
.dialog-checkbox-text {
  font-weight: 700; font-size: 13px; color: var(--ink);
}
.dialog-checkbox-hint {
  display: block;
  margin-top: 2px;
  font-size: 11.5px; color: var(--ink-3); line-height: 1.4;
}
.dialog-label { font-size: 12px; color: var(--ink-3); }
.dialog-input {
  border: 1px solid rgba(var(--ink-rgb), 0.08);
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
  color: var(--ink-4);
}

.color-swatch {
  width: 22px;
  height: 22px;
  border-radius: 6px;
  border: 1px solid rgba(var(--ink-rgb), 0.12);
  box-shadow: inset 0 0 0 1px rgba(255,255,255,0.6);
}

.reset-swatch {
  cursor: pointer;
  border: 1px solid rgba(var(--ink-rgb), 0.12);
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
  background: rgba(var(--ink-rgb), 0.05);
  border: 1px solid rgba(var(--ink-rgb), 0.05);
}

.preview-chip {
  width: 52px;
  height: 52px;
  border-radius: 14px;
  box-shadow: 0 10px 20px rgba(var(--ink-rgb), 0.12);
}

.preview-title { font-weight: 700; font-size: 14px; }
.preview-sub { font-size: 12px; color: var(--ink-3); }

.picker-area {
  margin-top: 12px;
  display: flex;
  align-items: center;
  gap: 16px;
}

.color-value {
  font-size: 13px;
  color: var(--ink-3);
}
.dialog-hint { font-size: 12px; color: var(--ink-4); margin: 0; }
.dialog-footer { display: flex; justify-content: flex-end; gap: 10px; margin-top: 16px; }

/* .dialog-pop-* 规则已删,SettingsPage 的 dialog 走 App.vue 全局规则。
   原因:scoped 0.25s transition: all 期间 dialog-overlay 整个在 transform,
   backdrop-filter 跟着缩(「shader 收缩」),box-shadow 也跟着缩。
   App.vue 全局用 overlay 只动 opacity、content 动 opacity+transform 的分层方案,
   overlay 永远不 transform,backdrop-filter 区域稳定。 */

.ri-spin { animation: ri-spin 1s linear infinite; }
@keyframes ri-spin { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }
/*
 * 内容井填满舞台，与左侧轨道、窗口右缘各留同样的 40px —— 转换页也是这么做的。
 * （之前这里限了 760px 却靠左对齐，右侧空出 260px+，左右明显不对称。）
 */
/*
 * 切换分类时，舞台里的分组依次入场。
 *
 * Workbench 的 stageKey 跟随当前选中项，所以分类一变整个舞台会重挂载，
 * 这里的 animation 就会重播 —— 不需要额外的事件或 JS。
 *
 * 顺序是「先分组容器、后行内容」：容器 520ms 落位，行在容器内再错开 30ms。
 * 用 animation 而不是 transition：transition 需要状态前后都存在，
 * 重挂载的元素没有「之前」，只有 animation 能表达入场。
 */
.stage-panel > section.settings-group {
  /* backwards 而不是 both —— 见 Workbench 的 .stage-swap 说明：
     both 会永久保留 translate: 0 0，使元素成为包含块，舞台里的 fixed 对话框会错位。 */
  animation: group-in 520ms var(--ease-out, cubic-bezier(0.22, 1, 0.36, 1)) backwards;
  animation-delay: calc(var(--gi, 0) * 62ms);
}

/* 只列到 10 个 —— 设置页最多同时出现 8 个分组，多写的规则永远不会命中 */
.stage-panel > section.settings-group:nth-child(1) { --gi: 0; }
.stage-panel > section.settings-group:nth-child(2) { --gi: 1; }
.stage-panel > section.settings-group:nth-child(3) { --gi: 2; }
.stage-panel > section.settings-group:nth-child(4) { --gi: 3; }
.stage-panel > section.settings-group:nth-child(5) { --gi: 4; }
.stage-panel > section.settings-group:nth-child(6) { --gi: 5; }
.stage-panel > section.settings-group:nth-child(7) { --gi: 6; }
.stage-panel > section.settings-group:nth-child(8) { --gi: 7; }
.stage-panel > section.settings-group:nth-child(9) { --gi: 8; }
.stage-panel > section.settings-group:nth-child(10) { --gi: 9; }

/* 行在容器之后入场，错开更小，读起来是「容器先立住、内容再浮现」 */
.stage-panel > section.settings-group .setting-item {
  animation: row-in 420ms var(--ease-out, cubic-bezier(0.22, 1, 0.36, 1)) backwards;
}
.stage-panel > section.settings-group .setting-item:nth-child(1) { animation-delay: 90ms; }
.stage-panel > section.settings-group .setting-item:nth-child(2) { animation-delay: 130ms; }
.stage-panel > section.settings-group .setting-item:nth-child(3) { animation-delay: 170ms; }
.stage-panel > section.settings-group .setting-item:nth-child(4) { animation-delay: 210ms; }
.stage-panel > section.settings-group .setting-item:nth-child(5) { animation-delay: 250ms; }
.stage-panel > section.settings-group .setting-item:nth-child(n + 6) { animation-delay: 290ms; }

@keyframes group-in {
  from { opacity: 0; translate: 0 14px; }
  to   { opacity: 1; translate: 0 0; }
}

@keyframes row-in {
  from { opacity: 0; translate: 0 8px; }
  to   { opacity: 1; translate: 0 0; }
}

@media (prefers-reduced-motion: reduce) {
  .stage-panel > section.settings-group,
  .stage-panel > section.settings-group .setting-item { animation-duration: 1ms; animation-delay: 0ms; }
}

/* ══════════ 设置内容层：发丝线分隔的行 ══════════
 *
 * 结构上已经把 <div class="group-card"> 那层壳去掉了，.setting-item 现在是
 * section 的直接子元素。所以这里只需要：
 *   · 每行下方一条发丝线，最后一行不画（否则会和分组间距叠成双线）
 *   · 悬停给极淡的暖色，不要卡片的整块变色
 * 行内容本身（item-icon / item-info / item-action）沿用原来的样式。
 */
.settings-group > .setting-item:last-of-type {
  border-bottom: none;
}

/* 可点击的行：悬停时只染一层几乎看不出的暖色，并轻微右移 */
.setting-item.clickable {
  cursor: pointer;
  border-radius: 6px;
}
.setting-item.clickable:hover {
  background: rgba(var(--warm-rgb), 0.06);
}

/* 分组之间靠留白分隔，不再靠卡片边界 */
.settings-group + .settings-group {
  margin-top: 34px;
}

/* 危险行（删除 / 重置）用暖红，不用纯红 */
.setting-item.danger .label { color: var(--danger); }
.setting-item.danger .item-icon { color: var(--danger); }

</style>
