import { useCallback, useEffect, useState } from 'react'
import { AnimatePresence, motion } from 'framer-motion'
import confetti from 'canvas-confetti'
import Background from './components/Background'
import Particles from './components/Particles'
import Quiz from './components/Quiz'
import Celebration from './components/Celebration'
import { useEasterEggs } from './hooks/useEasterEggs'

// Want to prank someone else? Open the page with ?name=Sam
const NAME = (() => {
  const n = new URLSearchParams(window.location.search).get('name')?.trim().slice(0, 20)
  return n || 'Hamude'
})()

export default function App() {
  const [phase, setPhase] = useState<'quiz' | 'party'>('quiz')
  const [attempts, setAttempts] = useState(0)
  const [round, setRound] = useState(0)
  const [toast, setToast] = useState<{ id: number; text: string } | null>(null)

  const showToast = useCallback((text: string) => setToast({ id: Date.now(), text }), [])
  useEffect(() => {
    if (!toast) return
    const t = setTimeout(() => setToast(null), 3200)
    return () => clearTimeout(t)
  }, [toast])

  useEffect(() => {
    document.title = `${NAME} Is Gay 🌈`
    console.log('%c🌈 Psst, nosy dev tools person!', 'font-size:16px;color:#d946ef;font-weight:bold')
    console.log('Try the Konami code. Or type "pride", "sparkle" or "hamude" anywhere. Or add ?name=Someone to the URL.')
  }, [])

  useEasterEggs({
    onKonami: useCallback(() => {
      const on = document.documentElement.classList.toggle('disco')
      showToast(on ? '🪩 DISCO MODE ACTIVATED' : 'Disco mode off. Party pooper.')
    }, [showToast]),
    onWord: useCallback(
      (w) => {
        if (w === 'hamude') showToast(`👋 Yes, that's ${NAME}!`)
        if (w === 'pride') {
          showToast('🏳️‍🌈 Love is love!')
          confetti({ particleCount: 160, spread: 120, origin: { y: 0.6 }, colors: ['#ff3b5c', '#ff9f3b', '#ffe23b', '#3bff8a', '#3bd1ff', '#8b5cff'] })
        }
        if (w === 'sparkle') {
          showToast('✨ Sparkle overload ✨')
          confetti({ particleCount: 80, spread: 360, startVelocity: 20, gravity: 0.3, ticks: 300, scalar: 1.2, colors: ['#ffffff', '#f0abfc', '#c084fc'], origin: { x: 0.5, y: 0.5 } })
        }
      },
      [showToast],
    ),
  })

  const restart = () => {
    confetti.reset()
    setAttempts(0)
    setRound((r) => r + 1)
    setPhase('quiz')
  }

  return (
    <div className="relative h-full w-full overflow-hidden">
      <Background />
      <Particles />

      {phase === 'quiz' && (
        <Quiz
          key={round}
          name={NAME}
          attempts={attempts}
          onAttempt={() => setAttempts((a) => a + 1)}
          onYes={() => setPhase('party')}
          onEgg={showToast}
        />
      )}

      <AnimatePresence>{phase === 'party' && <Celebration key="party" name={NAME} onRetry={restart} />}</AnimatePresence>

      <AnimatePresence>
        {toast && (
          <motion.div
            key={toast.id}
            initial={{ opacity: 0, y: 30, scale: 0.9 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={{ opacity: 0, y: 20 }}
            className="glass pointer-events-none fixed bottom-14 left-1/2 z-50 -translate-x-1/2 whitespace-nowrap rounded-full px-5 py-3 text-sm font-medium"
          >
            {toast.text}
          </motion.div>
        )}
      </AnimatePresence>

      <footer className="pointer-events-none fixed inset-x-0 bottom-3 z-10 px-4 text-center text-[11px] text-white/30">
        A completely fictional, lighthearted prank. 🌈 Love is love. · psst: ↑↑↓↓←→←→BA
      </footer>
    </div>
  )
}
