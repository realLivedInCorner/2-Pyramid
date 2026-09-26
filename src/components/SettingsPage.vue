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
        </div>
      </section>

      <!-- 转换设置 -->
      <section class="settings-group" v-if="shouldShowGroup('convert')">
        <div class="group-card">
        <h3 class="group-title">{{ t('settings.groups.convert') }}</h3>
          <div class="setting-item">
            <div class="item-icon">
              <i class="ri-code-box-line" aria-hidden="true"></i>
            </div>
            <div class="item-info">
              <div class="label">Editor Mode / Foray</div>
              <div class="desc">开启后主页拖入 zip 进入 Foray；关闭为普通转换。</div>
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
              <div class="label">Foray AI（OpenAI 兼容）</div>
              <div class="desc">Key 仅存本机 ~/.2pyr/foray-ai.json；外部 API 与作者无关。提示词可改可还原。</div>
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
              <label>默认档位
                <select v-model.number="aiTier">
                  <option :value="1">1 目录树</option>
                  <option :value="2">2 +mcmeta</option>
                  <option :value="3">3 +JSON</option>
                  <option :value="4">4 +着色器</option>
                  <option :value="5">5 +贴图概括</option>
                </select>
              </label>
            </div>
            <label class="wide">系统提示词
              <textarea v-model="aiPrompt" rows="3" placeholder="留空使用内置默认"></textarea>
            </label>
            <div class="foray-ai-actions">
              <button class="ghost-btn" type="button" @click="saveAiToBackend">保存 AI 配置</button>
              <button class="ghost-btn" type="button" @click="restoreAiPrompt">还原默认提示词</button>
              <button class="btn-text" type="button" @click="testAiConnection">测试连接</button>
            </div>
            <p v-if="aiMsg" class="foray-ai-msg">{{ aiMsg }}</p>
          </div>
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

          <div class="setting-item clickable" @click="showNamingDialog = true" v-if="shouldShowItem('outputNaming')">
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
      </div>
      </section>

      <!-- 转换历史 -->
      <section class="settings-group" v-if="shouldShowGroup('conversionHistory')">
        <h3 class="group-title">{{ t('settings.conversionHistory.groupTitle') }}</h3>
        <div class="group-card">
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

      <!-- 动画设置：总开关 + 速率 -->
      <section class="settings-group" v-if="shouldShowGroup('animationSpeed')">
        <h3 class="group-title">{{ t('settings.groups.animation') }}</h3>
        <div class="group-card">
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
          <div class="setting-item" v-if="shouldShowItem('updateSource')">
            <div class="item-icon">
              <i class="ri-server-line" aria-hidden="true"></i>
            </div>
            <div class="item-info">
              <div class="label">{{ t('settings.updateSource.label') }}</div>
              <div class="desc">{{ t('settings.updateSource.desc') }}</div>
              <!-- 测速结果 -->
              <div v-if="speedResults" class="source-speed">
                <div
                  v-for="r in speedResults"
                  :key="r.source"
                  class="speed-row"
                  :class="{ unreachable: !r.reachable }"
                >
                  <span class="speed-name">{{ r.source === 'mirror' ? t('settings.updateSource.mirror') : t('settings.updateSource.github') }}</span>
                  <span v-if="r.reachable" class="speed-val">
                    {{ r.speedKbps >= 1024 ? (r.speedKbps / 1024).toFixed(1) + ' MB/s' : r.speedKbps + ' KB/s' }} · {{ r.latencyMs }}ms
                  </span>
                  <span v-else class="speed-val">{{ r.error || t('settings.updateSource.unreachable') }}</span>
                </div>
              </div>
            </div>
            <div class="item-action source-actions">
              <div class="segmented">
                <button class="seg-btn" :class="{ active: updateSource === 'mirror' }" @click="changeSource('mirror')">{{ t('settings.updateSource.mirror') }}</button>
                <button class="seg-btn" :class="{ active: updateSource === 'github' }" @click="changeSource('github')">{{ t('settings.updateSource.github') }}</button>
              </div>
              <div class="speed-btns">
                <button class="btn-text secondary speed-btn" :disabled="sourceMeasuring" @click="measureSources">
                  <i :class="sourceMeasuring ? 'ri-loader-4-line ri-spin' : 'ri-speed-up-line'" aria-hidden="true"></i>
                  {{ sourceMeasuring ? t('settings.updateSource.measuring') : t('settings.updateSource.speedTest') }}
                </button>
                <button
                  v-if="fastestSource"
                  class="btn-text speed-btn"
                  @click="changeSource(fastestSource)"
                >{{ t('settings.updateSource.useFastest') }}（{{ fastestSource === 'mirror' ? t('settings.updateSource.mirror') : t('settings.updateSource.github') }}）</button>
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
        </div>
      </section>
    </main>

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
      <section class="settings-group dev-group" v-if="devModeEnabled && shouldShowGroup('dev')">
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

        <div class="group-card">
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
        </div>
      </section>
    </transition>

    <ClearConfigDialog v-model="showClearConfigDialog" />
    <FactoryResetDialog v-model="showFactoryResetDialog" @reset="emit('reset-to-oobe')" />
    <LogWindowDialog v-model="showLogWindow" />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch, onUnmounted } from 'vue';
import { useI18n } from 'vue-i18n'
import { save } from '@tauri-apps/plugin-dialog';
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
import FactoryResetDialog from './settings/FactoryResetDialog.vue';
import ClearConfigDialog from './settings/ClearConfigDialog.vue';
import OutputPathDialog from './settings/OutputPathDialog.vue';
import DevUnlockDialog from './settings/DevUnlockDialog.vue';
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
const conversionThreadsOptions = [1, 2, 4];
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

const defaultThemeColor = '#007bff';
const themeColor = ref('#007bff');

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
    aiMsg.value = '已保存到本机配置';
  } catch (e: any) {
    aiMsg.value = String(e);
  }
}

function restoreAiPrompt() {
  aiPrompt.value = '';
  aiMsg.value = '已还原内置默认提示词（保存后生效）';
}

async function testAiConnection() {
  await saveAiToBackend();
  try {
    const r = await invoke<string>('foray_ai_test');
    aiMsg.value = '连接 OK：' + String(r).slice(0, 80);
  } catch (e: any) {
    aiMsg.value = '连接失败：' + String(e).slice(0, 160);
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

const onDevUnlocked = () => {
  devModeEnabled.value = true;
  emit('update:dev-mode', true);
};

// ── Updater ──────────────────────────────────────
const { checkForUpdate, getChannel } = useUpdater();
const updateChannel = ref('master');
const autoCheckUpdate = ref(true);
const updateSource = ref('mirror');
const updateChecking = ref(false);
const updateError = ref('');
const currentVersion = ref('');
const displayVersion = computed(() => {
  const v = appInfoVersion.value && appInfoVersion.value !== '0.0.0'
    ? appInfoVersion.value
    : currentVersion.value;
  return v || '—';
});

interface SourceSpeedResult {
  source: string;
  reachable: boolean;
  latencyMs: number;
  speedKbps: number;
  error: string | null;
}
const speedResults = ref<SourceSpeedResult[] | null>(null);
const sourceMeasuring = ref(false);

const fastestSource = computed(() => {
  const list = speedResults.value;
  if (!list) return null;
  const reachable = list.filter((r) => r.reachable);
  if (reachable.length < 2) return null;
  const a = reachable[0];
  const b = reachable[1];
  if (a.speedKbps !== b.speedKbps) {
    return a.speedKbps > b.speedKbps ? a.source : b.source;
  }
  return a.latencyMs <= b.latencyMs ? a.source : b.source;
});

async function measureSources() {
  if (sourceMeasuring.value) return;
  sourceMeasuring.value = true;
  speedResults.value = null;
  try {
    const results = await invoke<SourceSpeedResult[]>('measure_update_sources');
    speedResults.value = results;
  } catch (e) {
    speedResults.value = null;
    console.error('measure_update_sources failed', e);
  } finally {
    sourceMeasuring.value = false;
  }
}

async function loadUpdateChannel() {
  try {
    updateChannel.value = await getChannel();
  } catch { /* use default */ }
  try {
    updateSource.value = await invoke<string>('get_update_source');
  } catch { /* use default */ }
  try {
    const cfg = await invoke<any>('get_config');
    if (typeof cfg?.auto_check_update === 'boolean') autoCheckUpdate.value = cfg.auto_check_update;
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

async function changeSource(src: string) {
  updateSource.value = src;
  try {
    await invoke('set_update_source', { source: src });
  } catch (e) {
    console.error('set_update_source failed', e);
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
  { id: 'autoCheckUpdate', group: 'version', label: t('settings.autoCheckUpdate.label'), desc: t('settings.autoCheckUpdate.desc') },
  { id: 'updateSource', group: 'version', label: t('settings.updateSource.label'), desc: t('settings.updateSource.desc') },
  { id: 'animationEnabled', group: 'animationSpeed', label: t('settings.animationEnabled.label'), desc: t('settings.animationEnabled.desc') },
  { id: 'animationSpeed', group: 'animationSpeed', label: t('settings.animationSpeed.label'), desc: t('settings.animationSpeed.desc') },
  { id: 'versionInfo', group: 'version', label: t('settings.versionInfo.label'), desc: t('settings.versionInfo.desc') },
  { id: 'authors', group: 'version', label: t('settings.versionInfo.authorsLabel'), desc: t('settings.versionInfo.authorsDesc') },
  { id: 'legal', group: 'version', label: t('settings.legal.label'), desc: t('settings.legal.desc') },
  { id: 'update', group: 'version', label: t('settings.checkUpdate.label'), desc: t('settings.checkUpdate.searchDesc') },
  { id: 'devLog', group: 'dev', label: t('settings.devMode.logWindowTitle'), desc: t('settings.devMode.viewLog') },
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
    const dest = await save({
      defaultPath: `2pyramid-actions-${stamp}.2amr`,
      filters: [{ name: '2amr', extensions: ['2amr'] }],
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
      if (typeof cfg?.conversion_threads === 'number' && [1, 2, 4].includes(cfg.conversion_threads)) {
        conversionThreads.value = cfg.conversion_threads;
      }
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
  padding-bottom: 28px;
}
.settings-scroll-area::-webkit-scrollbar { width: 6px; }
.settings-scroll-area::-webkit-scrollbar-thumb {
  background: rgba(0,0,0,0.12); border-radius: 3px;
}
.settings-scroll-area::-webkit-scrollbar-thumb:hover {
  background: rgba(0,0,0,0.22);
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
  color: #0f172a;
  text-transform: none;
  letter-spacing: 0;
  font-size: 14px;
}

.dev-title i {
  font-size: 18px;
  color: #059669;
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
  color: #047857;
  border: 1px solid rgba(5, 150, 105, 0.22);
}

.dev-sub {
  margin: 0 0 12px 15px;
  font-size: 12px;
  color: #86868b;
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
  background: #cbd5e1;
  flex-shrink: 0;
  box-shadow: 0 0 0 3px rgba(148, 163, 184, 0.15);
}

.live-dot.on {
  background: #10b981;
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
  color: #0f172a;
}

.live-text span {
  font-size: 11.5px;
  color: #64748b;
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
  border: 1px solid rgba(0, 0, 0, 0.05);
  font-size: 12px;
  font-weight: 700;
  color: #334155;
  font-variant-numeric: tabular-nums;
}

.live-stats .stat i {
  font-size: 13px;
  color: #059669;
}

.live-stats .stat.port {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  color: #047857;
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
.setting-item.is-disabled { opacity: 0.45; }
.segmented.disabled { pointer-events: none; }

.item-icon { width: 40px; color: #111827; display: inline-flex; align-items: center; justify-content: center; }
.item-icon i { font-size: 20px; line-height: 1; }
.item-info { flex: 1; display: flex; flex-direction: column; gap: 2px; }
.item-info .label { font-size: 15px; font-weight: 600; color: #1d1d1f; }
.item-info .desc { font-size: 12px; color: #86868b; }

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
  color: #1d1d1f;
  font-variant-numeric: tabular-nums;
}
.meta-sub {
  font-size: 11px;
  font-weight: 600;
  color: #94a3b8;
  font-variant-numeric: tabular-nums;
}

.item-arrow { color: #c6c6c8; font-weight: 800; }

/* 更新源：测速结果与操作按钮 */
.source-actions { flex-direction: column; align-items: flex-end; gap: 8px; }
.speed-btns { display: flex; gap: 6px; flex-wrap: wrap; justify-content: flex-end; }
.speed-btn { padding: 6px 12px; font-size: 12px; }
.source-speed { margin-top: 6px; display: flex; flex-direction: column; gap: 3px; }
.speed-row { display: flex; gap: 8px; font-size: 11.5px; color: #6b7280; }
.speed-row.unreachable .speed-val { color: #dc2626; }
.speed-name { font-weight: 700; color: #374151; min-width: 56px; }
.speed-val { font-family: ui-monospace, SFMono-Regular, Consolas, monospace; }

.fanhua-select {
  background: rgba(0, 0, 0, 0.05); border: none; padding: 6px 10px;
  border-radius: 8px; font-size: 13px; outline: none;
}

/* 设置页局部覆盖：主题色文字按钮（比全局 shared 的灰字更醒目） */
.btn-text {
  background: color-mix(in srgb, var(--theme-color) 12%, transparent);
  color: var(--theme-color);
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
  color: #6b7280;
  font-weight: 600;
}
.foray-ai-grid input,
.foray-ai-grid select,
.foray-ai-form textarea {
  width: 100%;
  box-sizing: border-box;
  padding: 10px 12px;
  border-radius: var(--ui-radius-btn, 10px);
  border: 1px solid rgba(0, 0, 0, 0.08);
  background: rgba(255, 255, 255, 0.8);
  color: #1d1d1f;
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
.foray-ai-form label { display: flex; flex-direction: column; gap: 6px; font-size: 12px; color: #6b7280; font-weight: 600; }
.foray-ai-form input, .foray-ai-form select, .foray-ai-form textarea { width: 100%; box-sizing: border-box; padding: 10px 12px; border-radius: var(--ui-radius-btn, 10px); border: 1px solid rgba(0,0,0,0.08); background: rgba(255,255,255,0.8); color: #1d1d1f; font: inherit; font-weight: 500; }
.foray-ai-msg { margin: 8px 0 0; font-size: 12px; color: var(--theme-color, #007bff); font-weight: 600; }
.switch { position: relative; display: inline-block; width: 42px; height: 24px; }
.switch input { opacity: 0; width: 0; height: 0; }
.slider {
  position: absolute; inset: 0; cursor: pointer; background-color: #e9e9eb; transition: background-color 200ms cubic-bezier(0.4, 0, 0.2, 1); border-radius: 34px;
}
.slider:before {
  position: absolute; content: ""; height: 18px; width: 18px; left: 3px; bottom: 3px; background-color: white; transition: transform 200ms cubic-bezier(0.4, 0, 0.2, 1); border-radius: 50%;
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

/* 弹窗骨架（overlay/content/header/body/footer）走全局 shared.css */
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
