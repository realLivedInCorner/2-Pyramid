import { useI18n } from 'vue-i18n'
import { watch } from 'vue'

export type SupportedLocale = 'zh-CN' | 'en-US'

export function useLanguage() {
  const { locale } = useI18n<{ message: Record<string, string> }, SupportedLocale>()

  function setLanguage(lang: SupportedLocale) {
    locale.value = lang
    localStorage.setItem('language', lang)
    document.documentElement.lang = lang
  }

  watch(locale, (val) => {
    document.documentElement.lang = val
  }, { immediate: true })

  return {
    locale,
    setLanguage,
  }
}
