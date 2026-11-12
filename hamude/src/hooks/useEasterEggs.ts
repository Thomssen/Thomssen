import { useEffect } from 'react'

const KONAMI = ['arrowup', 'arrowup', 'arrowdown', 'arrowdown', 'arrowleft', 'arrowright', 'arrowleft', 'arrowright', 'b', 'a']

type Handlers = {
  onKonami: () => void
  onWord: (word: 'hamude' | 'pride' | 'sparkle') => void
}

/** Listens for secret key sequences anywhere on the page. */
export function useEasterEggs({ onKonami, onWord }: Handlers) {
  useEffect(() => {
    let konami = 0
    let typed = ''
    const down = (e: KeyboardEvent) => {
      const k = e.key.toLowerCase()
      konami = k === KONAMI[konami] ? konami + 1 : k === KONAMI[0] ? 1 : 0
      if (konami === KONAMI.length) {
        konami = 0
        onKonami()
      }
      if (k.length === 1) {
        typed = (typed + k).slice(-8)
        for (const w of ['hamude', 'pride', 'sparkle'] as const) {
          if (typed.endsWith(w)) {
            typed = ''
            onWord(w)
          }
        }
      }
    }
    window.addEventListener('keydown', down)
    return () => window.removeEventListener('keydown', down)
  }, [onKonami, onWord])
}
