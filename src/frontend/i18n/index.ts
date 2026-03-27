import { ref, computed } from 'vue'
import en from './en'
import de from './de'
import fr from './fr'
import es from './es'

export type Locale = 'en' | 'de' | 'fr' | 'es'
export type TranslationDict = typeof en

const messages: Record<Locale, TranslationDict> = { en, de: de as any, fr: fr as any, es: es as any }

export const LOCALES: { code: Locale; label: string }[] = [
  { code: 'en', label: 'English' },
  { code: 'de', label: 'Deutsch' },
  { code: 'fr', label: 'Français' },
  { code: 'es', label: 'Español' },
]

/** Detect initial locale from localStorage or browser */
function detectLocale(): Locale {
  const stored = localStorage.getItem('plankton-locale')
  if (stored && stored in messages) return stored as Locale
  const browserLang = navigator.language.split('-')[0]
  if (browserLang in messages) return browserLang as Locale
  return 'en'
}

/** Reactive current locale */
export const currentLocale = ref<Locale>(detectLocale())

/** Set locale and persist */
export function setLocale(locale: Locale): void {
  currentLocale.value = locale
  localStorage.setItem('plankton-locale', locale)
  document.documentElement.lang = locale
}

/** Resolve a dotted key path from a translation object */
function resolve(obj: any, path: string): string | undefined {
  return path.split('.').reduce((acc, key) => acc?.[key], obj)
}

/**
 * Translate a key with optional interpolation.
 * Usage: t('bulk.selected', { count: 5 }) → "5 task(s) selected"
 */
export function t(key: string, params?: Record<string, string | number>): string {
  let text = resolve(messages[currentLocale.value], key)
    ?? resolve(messages.en, key)  // fallback to English
    ?? key  // fallback to key itself

  if (params) {
    for (const [k, v] of Object.entries(params)) {
      text = text.replace(new RegExp(`\\{${k}\\}`, 'g'), String(v))
    }
  }
  return text
}

/** Vue composable for i18n */
export function useI18n() {
  return {
    t,
    locale: currentLocale,
    setLocale,
    locales: LOCALES,
  }
}
