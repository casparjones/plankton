// Globale Toast-Instanz für Nicht-Component-Code (Services, etc.)
import { useToast, POSITION } from 'vue-toastification'
import { h } from 'vue'
import { t } from './i18n'

export const toast = useToast()

/**
 * Toast-basierter Prompt-Dialog. Zeigt einen Toast mit Eingabefeld und OK/Abbrechen.
 * Resolved mit dem eingegebenen Text (OK/Enter) oder null (Abbrechen/ESC/Timeout).
 */
export function toastPrompt(message: string, placeholder?: string): Promise<string | null> {
  return new Promise((resolve) => {
    let resolved = false
    const id = toast.info(
      {
        render() {
          const input = h('input', {
            type: 'text',
            placeholder: placeholder || '',
            class: 'toast-prompt-input',
            autocomplete: 'off',
            onKeydown: (e: KeyboardEvent) => {
              if (e.key === 'Enter') {
                const val = (e.target as HTMLInputElement).value.trim()
                resolved = true
                toast.dismiss(id)
                resolve(val || null)
              } else if (e.key === 'Escape') {
                resolved = true
                toast.dismiss(id)
                resolve(null)
              }
            },
          })
          return h('div', { class: 'toast-confirm' }, [
            h('div', { class: 'toast-confirm-msg' }, message),
            h('div', { class: 'toast-prompt-field' }, [input]),
            h('div', { class: 'toast-confirm-actions' }, [
              h('button', {
                class: 'toast-prompt-ok',
                onClick: (e: Event) => {
                  const inputEl = (e.target as HTMLElement)
                    .closest('.toast-confirm')
                    ?.querySelector<HTMLInputElement>('.toast-prompt-input')
                  const val = inputEl?.value.trim() || null
                  resolved = true
                  toast.dismiss(id)
                  resolve(val)
                },
              }, t('ok') || 'OK'),
              h('button', {
                class: 'toast-confirm-no',
                onClick: () => { resolved = true; toast.dismiss(id); resolve(null) },
              }, t('cancel') || 'Abbrechen'),
            ]),
          ])
        },
      },
      {
        position: POSITION.TOP_CENTER,
        timeout: 30000,
        closeOnClick: false,
        draggable: false,
        closeButton: false,
        onClose: () => { if (!resolved) resolve(null) },
      },
    )
    // Nach dem Rendern: Input fokussieren
    setTimeout(() => {
      const input = document.querySelector<HTMLInputElement>('.toast-prompt-input')
      if (input) input.focus()
    }, 50)
  })
}

/**
 * Toast-basierter Confirm-Dialog. Zeigt einen Toast mit Ja/Nein Buttons.
 * Resolved mit true bei Bestätigung, false bei Abbruch oder Timeout.
 */
export function toastConfirm(message: string): Promise<boolean> {
  return new Promise((resolve) => {
    let resolved = false
    const id = toast.warning(
      {
        render() {
          return h('div', { class: 'toast-confirm' }, [
            h('div', { class: 'toast-confirm-msg' }, message),
            h('div', { class: 'toast-confirm-actions' }, [
              h('button', {
                class: 'toast-confirm-yes',
                onClick: () => { resolved = true; toast.dismiss(id); resolve(true) },
              }, t('yes')),
              h('button', {
                class: 'toast-confirm-no',
                onClick: () => { resolved = true; toast.dismiss(id); resolve(false) },
              }, t('no')),
            ]),
          ])
        },
      },
      {
        position: POSITION.TOP_CENTER,
        timeout: 10000,
        closeOnClick: false,
        draggable: false,
        closeButton: false,
        onClose: () => { if (!resolved) resolve(false) },
      },
    )
  })
}
