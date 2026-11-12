import { useEffect, useMemo, useState } from 'react'
import { motion } from 'framer-motion'
import confetti from 'canvas-confetti'

const COLORS = ['#ff3b5c', '#ff9f3b', '#ffe23b', '#3bff8a', '#3bd1ff', '#8b5cff', '#ff3bd4']
const DANCERS = ['🕺', '💃', '🪩', '🦄', '🌈', '🎉', '✨', '🥳']
const RISERS = ['🌈', '🦄', '✨', '💖', '🪩', '🎉', '💜', '🧁']

export default function Celebration({ name, onRetry }: { name: string; onRetry: () => void }) {
  const [showRetry, setShowRetry] = useState(false)
  const title = `WE KNEW IT, ${name.toUpperCase()}!`
  const chars = useMemo(() => title.split(''), [title])
  const risers = useMemo(
    () =>
      Array.from({ length: 22 }, (_, i) => ({
        e: RISERS[i % RISERS.length],
        left: Math.random() * 100,
        dur: 6 + Math.random() * 6,
        delay: Math.random() * 6,
        size: 1.4 + Math.random() * 2,
      })),
    [],
  )

  // Confetti: a big center explosion, then rainbow cannons from both sides.
  useEffect(() => {
    const opts = { colors: COLORS, disableForReducedMotion: true }
    confetti({ ...opts, particleCount: 220, spread: 360, startVelocity: 55, origin: { x: 0.5, y: 0.45 }, ticks: 220 })
    const t1 = setTimeout(() => confetti({ ...opts, particleCount: 140, spread: 100, angle: 60, origin: { x: 0, y: 0.8 }, startVelocity: 70 }), 250)
    const t2 = setTimeout(() => confetti({ ...opts, particleCount: 140, spread: 100, angle: 120, origin: { x: 1, y: 0.8 }, startVelocity: 70 }), 250)

    let shapes: confetti.Shape[] | undefined
    try {
      shapes = [confetti.shapeFromText({ text: '🌈', scalar: 3 }), confetti.shapeFromText({ text: '✨', scalar: 3 }), confetti.shapeFromText({ text: '🦄', scalar: 3 })]
    } catch { /* emoji shapes are optional */ }
    const t3 = setTimeout(() => shapes && confetti({ ...opts, shapes, scalar: 3, particleCount: 40, spread: 160, startVelocity: 45, origin: { x: 0.5, y: 0.6 } }), 700)

    const end = Date.now() + 6000
    const loop = setInterval(() => {
      if (Date.now() > end) return clearInterval(loop)
      confetti({ ...opts, particleCount: 6, angle: 60, spread: 70, origin: { x: 0, y: 0.7 } })
      confetti({ ...opts, particleCount: 6, angle: 120, spread: 70, origin: { x: 1, y: 0.7 } })
    }, 120)
    const retry = setTimeout(() => setShowRetry(true), 2200)
    return () => { clearTimeout(t1); clearTimeout(t2); clearTimeout(t3); clearTimeout(retry); clearInterval(loop); confetti.reset() }
  }, [])

  return (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      className="fixed inset-0 z-40 flex flex-col items-center justify-center overflow-hidden px-4 text-center"
    >
      {/* rainbow backdrop */}
      <motion.div
        className="absolute inset-0 opacity-45"
        style={{
          background:
            'repeating-linear-gradient(115deg, #ff3b5c 0 8%, #ff9f3b 8% 16%, #ffe23b 16% 24%, #3bff8a 24% 32%, #3bd1ff 32% 40%, #8b5cff 40% 48%, #ff3bd4 48% 56%)',
          backgroundSize: '200% 200%',
        }}
        animate={{ backgroundPosition: ['0% 0%', '100% 100%'] }}
        transition={{ duration: 6, repeat: Infinity, ease: 'linear' }}
      />
      <div className="absolute inset-0 bg-[radial-gradient(ellipse_at_center,rgba(5,5,7,0.55)_0%,rgba(5,5,7,0.9)_80%)]" />

      {/* pulsing rainbow rings */}
      {COLORS.map((c, i) => (
        <motion.div
          key={c}
          className="absolute left-1/2 top-1/2 rounded-full border-[10px]"
          style={{ borderColor: c, width: 200, height: 200, x: '-50%', y: '-50%', opacity: 0.5 }}
          animate={{ scale: [0.4, 7], opacity: [0.55, 0] }}
          transition={{ duration: 4, repeat: Infinity, delay: i * 0.5, ease: 'easeOut' }}
        />
      ))}

      {/* rising emoji */}
      {risers.map((r, i) => (
        <motion.span
          key={i}
          className="pointer-events-none absolute bottom-[-10%]"
          style={{ left: `${r.left}%`, fontSize: `${r.size}rem` }}
          animate={{ y: ['0vh', '-120vh'], rotate: [0, 360], opacity: [0, 1, 1, 0] }}
          transition={{ duration: r.dur, delay: r.delay, repeat: Infinity, ease: 'linear' }}
        >
          {r.e}
        </motion.span>
      ))}

      {/* disco ball */}
      <motion.div
        className="absolute left-1/2 top-0 z-10 flex origin-top flex-col items-center"
        initial={{ y: -300 }}
        animate={{ y: 0, rotate: [-8, 8, -8] }}
        transition={{ y: { type: 'spring', stiffness: 60, damping: 10, delay: 0.3 }, rotate: { duration: 3, repeat: Infinity, ease: 'easeInOut' } }}
      >
        <div className="h-16 w-px bg-white/50 sm:h-24" />
        <motion.span className="text-6xl drop-shadow-[0_0_30px_rgba(255,255,255,0.8)] sm:text-8xl" animate={{ rotateY: 360 }} transition={{ duration: 4, repeat: Infinity, ease: 'linear' }}>
          🪩
        </motion.span>
      </motion.div>

      {/* headline */}
      <h1 className="relative z-10 mt-24 font-display text-4xl font-bold leading-tight sm:text-7xl md:text-8xl" aria-label={title}>
        {chars.map((c, i) => (
          <motion.span
            key={i}
            aria-hidden
            className="rainbow-text inline-block"
            initial={{ opacity: 0, y: 80, scale: 0, rotate: -40 }}
            animate={{ opacity: 1, y: [0, -14, 0], scale: 1, rotate: 0 }}
            transition={{
              opacity: { delay: 0.2 + i * 0.045 },
              scale: { delay: 0.2 + i * 0.045, type: 'spring', stiffness: 260, damping: 12 },
              rotate: { delay: 0.2 + i * 0.045, type: 'spring', stiffness: 200, damping: 12 },
              y: { delay: 1.4 + i * 0.07, duration: 1.2, repeat: Infinity, ease: 'easeInOut' },
            }}
            style={{ whiteSpace: 'pre', filter: 'drop-shadow(0 0 18px rgba(217,70,239,0.55))' }}
          >
            {c}
          </motion.span>
        ))}
      </h1>

      {/* dancing crew */}
      <div className="relative z-10 mt-8 flex flex-wrap justify-center gap-2 text-4xl sm:gap-4 sm:text-6xl">
        {DANCERS.map((d, i) => (
          <motion.span
            key={i}
            animate={{ y: [0, -28, 0], rotate: [0, i % 2 ? 18 : -18, 0], scale: [1, 1.2, 1] }}
            transition={{ duration: 0.8, repeat: Infinity, delay: i * 0.1, ease: 'easeInOut' }}
          >
            {d}
          </motion.span>
        ))}
      </div>

      <motion.p
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ delay: 1.2 }}
        className="relative z-10 mt-6 max-w-md text-sm text-white/70 sm:text-base"
      >
        Officially certified <span className="font-semibold text-fuchsia-300">100% fabulous</span>. 🏅
        <br />
        <span className="text-white/45">(It&apos;s a joke, {name}. Love you, mate. Everyone&apos;s welcome here. 🌈)</span>
      </motion.p>

      <div className="relative z-10 mt-8 h-16">
        {showRetry && (
          <motion.button
            type="button"
            onClick={onRetry}
            initial={{ opacity: 0, scale: 0.5, y: 20 }}
            animate={{ opacity: 1, scale: 1, y: 0 }}
            whileHover={{ scale: 1.08 }}
            whileTap={{ scale: 0.93 }}
            transition={{ type: 'spring', stiffness: 300, damping: 15 }}
            className="glass rounded-2xl px-8 py-4 font-display text-lg font-bold text-white shadow-[0_0_40px_-6px_rgba(217,70,239,0.8)] outline-none focus-visible:ring-4 focus-visible:ring-white/60"
          >
            ↺ Try Again
          </motion.button>
        )}
      </div>
    </motion.div>
  )
}
