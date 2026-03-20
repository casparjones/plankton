// Globale Toast-Instanz für Nicht-Component-Code (Services, etc.)
import { useToast, POSITION } from 'vue-toastification'
import { h } from 'vue'

export const toast = useToast()

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
              }, 'Ja'),
              h('button', {
                class: 'toast-confirm-no',
                onClick: () => { resolved = true; toast.dismiss(id); resolve(false) },
              }, 'Nein'),
            ]),
          ])
        },
      },
      {
        timeout: 10000,
        closeOnClick: false,
        draggable: false,
        closeButton: false,
        onClose: () => { if (!resolved) resolve(false) },
      },
    )
  })
}
