import { useCallback, useEffect, useLayoutEffect, useRef, useState } from 'react'
import { AnimatePresence, motion } from 'framer-motion'
import confetti from 'canvas-confetti'
import { clickMessages, dodgeMessages, fill, milestones, noLabels, ranks, tapMessages } from '../messages'

const NO_W = 144 // keep in sync with the w-36 / h-14 classes on the placeholder
const NO_H = 56
const PAD = 14

type Pt = { x: number; y: number }
type Box = { left: number; top: number; right: number; bottom: number }
type NoState = { x: number; y: number; rotate: number; scale: number; spring: { stiffness: number; damping: number; mass: number } }
type Via = 'move' | 'tap' | 'click'

const rand = (a: number, b: number) => a + Math.random() * (b - a)
const clamp = (v: number, a: number, b: number) => Math.min(b, Math.max(a, v))
const inflate = (b: Box, n: number): Box => ({ left: b.left - n, top: b.top - n, right: b.right + n, bottom: b.bottom + n })
const overlaps = (a: Box, b: Box) => a.left < b.right && a.right > b.left && a.top < b.bottom && a.bottom > b.top
const boxOf = (p: Pt, scale = 1): Box => {
  const w = NO_W * scale, h = NO_H * scale
  return { left: p.x + (NO_W - w) / 2, top: p.y + (NO_H - h) / 2, right: p.x + (NO_W + w) / 2, bottom: p.y + (NO_H + h) / 2 }
}
const distToBox = (c: Pt, b: Box) => {
  const dx = Math.max(b.left - c.x, 0, c.x - b.right)
  const dy = Math.max(b.top - c.y, 0, c.y - b.bottom)
  return Math.hypot(dx, dy)
}

type Props = {
  name: string
  attempts: number
  onAttempt: () => void
  onYes: () => void
  onEgg: (text: string) => void
}

export default function Quiz({ name, attempts, onAttempt, onYes, onEgg }: Props) {
  const noHome = useRef<HTMLDivElement>(null)
  const yesHome = useRef<HTMLDivElement>(null)

  const [no, setNo] = useState<NoState | null>(null)
  const noPos = useRef<Pt>({ x: 0, y: 0 })
  const noScale = useRef(1)
  const noRot = useRef(0)
  const dodged = useRef(false)
  const lastFlee = useRef(0)
  const swapped = useRef(false)
  const [yesOff, setYesOff] = useState<Pt>({ x: 0, y: 0 })
  const yesOffRef = useRef<Pt>({ x: 0, y: 0 })
  const [label, setLabel] = useState(noLabels[0])
  const [puffs, setPuffs] = useState<{ id: number; x: number; y: number }[]>([])
  const [message, setMessage] = useState<{ id: number; text: string } | null>(null)
  const lastMsg = useRef(-1)
  const msgId = useRef(0)
  const attemptsRef = useRef(attempts)
  attemptsRef.current = attempts
  const qClicks = useRef(0)
  const badgeClicks = useRef(0)

  const homePos = useCallback((): Pt => {
    const r = noHome.current!.getBoundingClientRect()
    return { x: r.left + (r.width - NO_W) / 2, y: r.top + (r.height - NO_H) / 2 }
  }, [])

  // Place NO in its slot on first paint, and keep it sane on resize.
  useLayoutEffect(() => {
    const place = () => {
      swapped.current = false
      yesOffRef.current = { x: 0, y: 0 }
      setYesOff({ x: 0, y: 0 })
      if (!dodged.current) {
        noPos.current = homePos()
      } else {
        noPos.current = {
          x: clamp(noPos.current.x, PAD, Math.max(PAD, window.innerWidth - NO_W - PAD)),
          y: clamp(noPos.current.y, PAD, Math.max(PAD, window.innerHeight - NO_H - PAD)),
        }
      }
      setNo((prev) =>
        prev
          ? { ...prev, ...noPos.current }
          : { ...noPos.current, rotate: 0, scale: 1, spring: { stiffness: 420, damping: 22, mass: 0.7 } },
      )
    }
    place()
    window.addEventListener('resize', place)
    return () => window.removeEventListener('resize', place)
  }, [homePos])

  const say = useCallback(
    (via: Via, n: number) => {
      const pool = milestones[n] ? null : via === 'tap' ? tapMessages : via === 'click' ? clickMessages : dodgeMessages
      let text: string
      if (!pool) text = milestones[n]
      else {
        let i = Math.floor(Math.random() * pool.length)
        if (pool.length > 1 && i === lastMsg.current) i = (i + 1) % pool.length
        lastMsg.current = i
        text = fill(pool[i], name)
      }
      setMessage({ id: ++msgId.current, text })
    },
    [name],
  )

  const flee = useCallback(
    (cursor: Pt, via: Via, force = false) => {
      const now = performance.now()
      if (!force && now - lastFlee.current < 240) return
      lastFlee.current = now
      dodged.current = true

      const W = window.innerWidth
      const H = window.innerHeight
      const cur = noPos.current
      const curC: Pt = { x: cur.x + NO_W / 2, y: cur.y + NO_H / 2 }
      const n = attemptsRef.current + 1

      const yesBox = yesHome.current!.getBoundingClientRect()
      const yesHomeBox: Box = { left: yesBox.left, top: yesBox.top, right: yesBox.right, bottom: yesBox.bottom }
      const yesC: Pt = { x: (yesBox.left + yesBox.right) / 2, y: (yesBox.top + yesBox.bottom) / 2 }

      // 1) Sometimes swap places with YES.
      let target: Pt | null = null
      let off: Pt = { x: 0, y: 0 }
      let didSwap = false
      if (!swapped.current && n >= 2 && Math.random() < 0.22) {
        const t: Pt = { x: yesC.x - NO_W / 2, y: yesC.y - NO_H / 2 }
        if (Math.hypot(t.x + NO_W / 2 - cursor.x, t.y + NO_H / 2 - cursor.y) > 90) {
          target = t
          off = { x: curC.x - yesC.x, y: curC.y - yesC.y }
          didSwap = true
        }
      }

      // 2) Otherwise pick an unpredictable spot, away from the cursor and from YES.
      if (!target) {
        const avoid: Box[] = [inflate(yesHomeBox, 22)]
        if (swapped.current) {
          const o = yesOffRef.current
          avoid.push(inflate({ left: yesBox.left + o.x, top: yesBox.top + o.y, right: yesBox.right + o.x, bottom: yesBox.bottom + o.y }, 22))
        }
        const minDist = Math.min(300, Math.min(W, H) * 0.5)
        let best: Pt | null = null
        let bestScore = -1
        for (let i = 0; i < 80; i++) {
          const p: Pt = {
            x: rand(PAD, Math.max(PAD, W - NO_W - PAD)),
            y: rand(PAD, Math.max(PAD, H - NO_H - PAD)),
          }
          if (avoid.some((b) => overlaps(boxOf(p), b))) continue
          const c: Pt = { x: p.x + NO_W / 2, y: p.y + NO_H / 2 }
          const dc = Math.hypot(c.x - cursor.x, c.y - cursor.y)
          const dp = Math.hypot(c.x - curC.x, c.y - curC.y)
          if (dc >= minDist && dp >= 140) { best = p; break }
          const score = Math.min(dc, 400) + Math.min(dp, 200) * 0.5
          if (score > bestScore) { bestScore = score; best = p }
        }
        target = best ?? { x: PAD, y: PAD }
      }

      // Funny physics: random spin, size and spring.
      const baseRot = Math.round(noRot.current / 360) * 360
      const spins = [-2, -1, 1, 2][Math.floor(Math.random() * 4)] * 360
      const tilt = Math.random() < 0.3 ? (Math.random() < 0.5 ? -1 : 1) * rand(8, 24) : 0
      const rotate = baseRot + spins + tilt
      const scale = Math.random() < 0.4 ? rand(0.58, 0.82) : rand(0.88, 1.1)
      noRot.current = rotate
      noScale.current = scale
      noPos.current = target

      // keep the target fully on-screen no matter what
      noPos.current = {
        x: clamp(target.x, PAD, Math.max(PAD, W - NO_W - PAD)),
        y: clamp(target.y, PAD, Math.max(PAD, H - NO_H - PAD)),
      }

      swapped.current = didSwap
      yesOffRef.current = off
      setYesOff(off)
      setNo({
        ...noPos.current,
        rotate,
        scale,
        spring: { stiffness: rand(260, 560), damping: rand(14, 26), mass: rand(0.55, 1) },
      })
      setLabel(noLabels[Math.floor(Math.random() * noLabels.length)])

      const id = ++msgId.current
      setPuffs((p) => [...p.slice(-4), { id, x: curC.x, y: curC.y }])
      window.setTimeout(() => setPuffs((p) => p.filter((q) => q.id !== id)), 700)

      onAttempt()
      say(via, n)
    },
    [onAttempt, say],
  )

  // Proximity detection: the heart of the dodge.
  useEffect(() => {
    const move = (e: PointerEvent) => {
      const coarse = e.pointerType === 'touch' || e.pointerType === 'pen'
      const radius = coarse ? 70 : 105
      const box = boxOf(noPos.current, noScale.current)
      if (distToBox({ x: e.clientX, y: e.clientY }, box) < radius) flee({ x: e.clientX, y: e.clientY }, 'move')
    }
    window.addEventListener('pointermove', move, { passive: true })
    return () => window.removeEventListener('pointermove', move)
  }, [flee])

  const lastDown = useRef(0)
  const onNoDown = (e: React.PointerEvent) => {
    e.preventDefault()
    lastDown.current = performance.now()
    flee({ x: e.clientX, y: e.clientY }, e.pointerType === 'mouse' ? 'click' : 'tap', true)
  }
  const onNoClick = (e: React.MouseEvent) => {
    e.preventDefault()
    // A click straight after a pointerdown was already handled (and counted).
    if (performance.now() - lastDown.current < 700) return
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect()
    const c = e.clientX || e.clientY ? { x: e.clientX, y: e.clientY } : { x: r.left + r.width / 2, y: r.top + r.height / 2 }
    flee(c, 'click', true)
  }

  const rank = [...ranks].reverse().find(([min]) => attempts >= min)![1]
  const yesScale = 1 + Math.min(attempts, 30) * 0.008

  const poke = () => {
    qClicks.current++
    if (qClicks.current === 5) { qClicks.current = 0; onEgg('Ow! Stop poking the question. 😳') }
  }
  const badge = () => {
    badgeClicks.current++
    if (badgeClicks.current === 5) {
      badgeClicks.current = 0
      confetti({ particleCount: 90, spread: 90, origin: { y: 0.2 }, colors: ['#ff3b5c', '#ffd23b', '#3bff8a', '#3bd1ff', '#a855f7', '#ff3bd4'] })
      onEgg('🌈 You found the rainbow badge secret!')
    }
  }

  const words = [`${name},`, 'are', 'you', 'gay?']

  return (
    <main className="relative z-10 flex min-h-full items-center justify-center px-4 py-10">
      {/* counter */}
      <motion.div
        initial={{ y: -30, opacity: 0 }}
        animate={{ y: 0, opacity: 1 }}
        transition={{ delay: 0.5, type: 'spring', stiffness: 200, damping: 20 }}
        className="glass fixed left-1/2 top-4 z-20 flex -translate-x-1/2 items-center gap-3 rounded-full px-4 py-2 text-xs sm:text-sm"
      >
        <span className="text-white/60">Escape attempts</span>
        <motion.span
          key={attempts}
          initial={{ scale: 1.8, color: '#f472b6' }}
          animate={{ scale: 1, color: '#ffffff' }}
          transition={{ type: 'spring', stiffness: 400, damping: 15 }}
          className="min-w-[1.5ch] text-center font-display text-base font-bold tabular-nums sm:text-lg"
          aria-live="polite"
        >
          {attempts}
        </motion.span>
        <span className="hidden h-4 w-px bg-white/15 sm:block" />
        <span className="hidden text-fuchsia-300 sm:block">{rank}</span>
      </motion.div>

      <motion.section
        initial={{ opacity: 0, y: 40, scale: 0.96 }}
        animate={{ opacity: 1, y: 0, scale: 1 }}
        transition={{ type: 'spring', stiffness: 120, damping: 18 }}
        className="glass relative w-full max-w-3xl rounded-[2rem] px-5 py-10 text-center sm:px-12 sm:py-16"
      >
        <motion.button
          type="button"
          onClick={badge}
          whileHover={{ rotate: 12, scale: 1.1 }}
          whileTap={{ scale: 0.85 }}
          animate={{ y: [0, -6, 0] }}
          transition={{ y: { duration: 3, repeat: Infinity, ease: 'easeInOut' } }}
          className="mx-auto mb-5 flex h-14 w-14 items-center justify-center rounded-2xl border border-white/10 bg-gradient-to-br from-fuchsia-500/30 to-purple-600/30 text-3xl shadow-[0_0_40px_-5px_rgba(217,70,239,0.6)]"
          aria-label="rainbow"
        >
          🌈
        </motion.button>

        <h1
          onClick={poke}
          className="font-display text-[2.4rem] font-bold leading-[1.05] tracking-tight sm:text-7xl"
        >
          {words.map((w, i) => (
            <motion.span
              key={w}
              initial={{ opacity: 0, y: 30, filter: 'blur(10px)' }}
              animate={{ opacity: 1, y: 0, filter: 'blur(0px)' }}
              transition={{ delay: 0.25 + i * 0.12, type: 'spring', stiffness: 140, damping: 16 }}
              className={`mr-[0.25em] inline-block last:mr-0 ${i === words.length - 1 ? 'rainbow-text' : ''}`}
            >
              {w}
            </motion.span>
          ))}
        </h1>

        <motion.p
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ delay: 1 }}
          className="mt-4 text-sm text-white/45"
        >
          Choose wisely. Or don&apos;t. There is only one good choice.
        </motion.p>

        <div className="mt-10 flex flex-col items-center justify-center gap-6 sm:mt-12 sm:flex-row sm:gap-10">
          <div ref={yesHome} className="h-16 w-56 sm:h-[4.5rem] sm:w-64">
            <motion.button
              type="button"
              onClick={onYes}
              initial={{ opacity: 0, scale: 0.6 }}
              animate={{ opacity: 1, x: yesOff.x, y: yesOff.y, scale: yesScale }}
              transition={{ type: 'spring', stiffness: 260, damping: 20, opacity: { delay: 0.8 } }}
              whileHover={{ scale: yesScale * 1.08 }}
              whileTap={{ scale: yesScale * 0.94 }}
              className="group relative z-20 h-full w-full overflow-hidden rounded-2xl bg-gradient-to-r from-fuchsia-500 via-purple-500 to-pink-500 font-display text-2xl font-bold tracking-wide text-white shadow-[0_0_50px_-4px_rgba(217,70,239,0.85),inset_0_1px_0_rgba(255,255,255,0.4)] outline-none focus-visible:ring-4 focus-visible:ring-white/60"
            >
              <span className="absolute inset-0 -translate-x-full bg-gradient-to-r from-transparent via-white/40 to-transparent transition-transform duration-700 group-hover:translate-x-full" />
              <span className="relative">YES 🌈</span>
            </motion.button>
          </div>

          {/* In-flow slot that keeps the layout stable while NO flies around */}
          <div ref={noHome} className="h-14 w-36" aria-hidden />
        </div>

        <div className="mt-8 flex min-h-[3.5rem] items-center justify-center">
          <AnimatePresence mode="wait">
            {message && (
              <motion.p
                key={message.id}
                initial={{ opacity: 0, y: 14, scale: 0.9, rotate: -2 }}
                animate={{ opacity: 1, y: 0, scale: 1, rotate: 0 }}
                exit={{ opacity: 0, y: -10, scale: 0.95 }}
                transition={{ type: 'spring', stiffness: 300, damping: 20 }}
                className="rounded-full border border-fuchsia-400/30 bg-fuchsia-500/10 px-5 py-2 text-sm font-medium text-fuchsia-100 shadow-[0_0_30px_-8px_rgba(217,70,239,0.8)] sm:text-base"
              >
                {message.text}
              </motion.p>
            )}
          </AnimatePresence>
        </div>
      </motion.section>

      {/* poofs */}
      {puffs.map((p) => (
        <motion.span
          key={p.id}
          initial={{ opacity: 0.9, scale: 0.4, y: 0 }}
          animate={{ opacity: 0, scale: 1.8, y: -30 }}
          transition={{ duration: 0.6, ease: 'easeOut' }}
          className="pointer-events-none fixed z-30 text-3xl"
          style={{ left: p.x - 16, top: p.y - 16 }}
        >
          💨
        </motion.span>
      ))}

      {/* the escaping NO button */}
      {no && (
        <motion.button
          type="button"
          initial={false}
          animate={{ x: no.x, y: no.y, rotate: no.rotate, scale: no.scale }}
          transition={{
            x: { type: 'spring', ...no.spring },
            y: { type: 'spring', ...no.spring },
            rotate: { type: 'spring', stiffness: 120, damping: 12 },
            scale: { type: 'spring', stiffness: 300, damping: 14 },
          }}
          onPointerDown={onNoDown}
          onClick={onNoClick}
          style={{ width: NO_W, height: NO_H, left: 0, top: 0 }}
          className="glass fixed z-30 select-none whitespace-nowrap rounded-2xl text-lg font-semibold text-white/85 outline-none hover:text-white focus-visible:ring-4 focus-visible:ring-white/40"
        >
          {label}
        </motion.button>
      )}
    </main>
  )
}
