import { useEffect, useRef } from 'react'

type P = { x: number; y: number; r: number; vx: number; vy: number; hue: number; a: number; tw: number }

/** Subtle drifting particles that gently shy away from the pointer. */
export default function Particles({ count = 70 }: { count?: number }) {
  const ref = useRef<HTMLCanvasElement>(null)

  useEffect(() => {
    const canvas = ref.current!
    const ctx = canvas.getContext('2d')!
    const reduce = window.matchMedia('(prefers-reduced-motion: reduce)').matches
    let w = 0, h = 0, raf = 0
    const dpr = Math.min(window.devicePixelRatio || 1, 2)
    const mouse = { x: -9999, y: -9999 }

    const resize = () => {
      w = window.innerWidth
      h = window.innerHeight
      canvas.width = w * dpr
      canvas.height = h * dpr
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0)
    }
    resize()

    const n = window.innerWidth < 640 ? Math.round(count * 0.6) : count
    const ps: P[] = Array.from({ length: n }, () => ({
      x: Math.random() * w,
      y: Math.random() * h,
      r: Math.random() * 1.8 + 0.4,
      vx: (Math.random() - 0.5) * 0.18,
      vy: -(Math.random() * 0.25 + 0.05),
      hue: 260 + Math.random() * 100, // purple -> pink
      a: Math.random() * 0.5 + 0.2,
      tw: Math.random() * Math.PI * 2,
    }))

    const draw = () => {
      ctx.clearRect(0, 0, w, h)
      for (const p of ps) {
        if (!reduce) {
          const dx = p.x - mouse.x, dy = p.y - mouse.y
          const d2 = dx * dx + dy * dy
          if (d2 < 140 * 140) {
            const d = Math.sqrt(d2) || 1
            p.x += (dx / d) * 0.9
            p.y += (dy / d) * 0.9
          }
          p.x += p.vx
          p.y += p.vy
          p.tw += 0.02
        }
        if (p.y < -10) { p.y = h + 10; p.x = Math.random() * w }
        if (p.x < -10) p.x = w + 10
        if (p.x > w + 10) p.x = -10
        const alpha = p.a * (0.6 + 0.4 * Math.sin(p.tw))
        ctx.beginPath()
        ctx.fillStyle = `hsla(${p.hue}, 95%, 70%, ${alpha})`
        ctx.shadowColor = `hsla(${p.hue}, 95%, 65%, ${alpha})`
        ctx.shadowBlur = 8
        ctx.arc(p.x, p.y, p.r, 0, Math.PI * 2)
        ctx.fill()
      }
      raf = requestAnimationFrame(draw)
    }
    draw()

    const move = (e: PointerEvent) => { mouse.x = e.clientX; mouse.y = e.clientY }
    window.addEventListener('resize', resize)
    window.addEventListener('pointermove', move)
    return () => {
      cancelAnimationFrame(raf)
      window.removeEventListener('resize', resize)
      window.removeEventListener('pointermove', move)
    }
  }, [count])

  return <canvas ref={ref} className="pointer-events-none fixed inset-0 z-0" aria-hidden />
}
