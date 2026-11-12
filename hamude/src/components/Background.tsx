import { motion } from 'framer-motion'

/** Dark aurora blobs + faint grid. Pure decoration. */
export default function Background() {
  return (
    <div className="pointer-events-none fixed inset-0 z-0 overflow-hidden bg-ink" aria-hidden>
      <motion.div
        className="absolute -left-40 -top-40 h-[34rem] w-[34rem] rounded-full bg-purple-600/30 blur-[120px]"
        animate={{ x: [0, 80, 0], y: [0, 60, 0] }}
        transition={{ duration: 18, repeat: Infinity, ease: 'easeInOut' }}
      />
      <motion.div
        className="absolute -bottom-48 -right-32 h-[36rem] w-[36rem] rounded-full bg-pink-600/25 blur-[130px]"
        animate={{ x: [0, -90, 0], y: [0, -50, 0] }}
        transition={{ duration: 22, repeat: Infinity, ease: 'easeInOut' }}
      />
      <motion.div
        className="absolute left-1/2 top-1/3 h-[26rem] w-[26rem] -translate-x-1/2 rounded-full bg-indigo-600/20 blur-[120px]"
        animate={{ scale: [1, 1.25, 1] }}
        transition={{ duration: 14, repeat: Infinity, ease: 'easeInOut' }}
      />
      <div
        className="absolute inset-0 opacity-[0.07]"
        style={{
          backgroundImage:
            'linear-gradient(rgba(255,255,255,.6) 1px, transparent 1px), linear-gradient(90deg, rgba(255,255,255,.6) 1px, transparent 1px)',
          backgroundSize: '56px 56px',
          maskImage: 'radial-gradient(ellipse at center, black 30%, transparent 75%)',
          WebkitMaskImage: 'radial-gradient(ellipse at center, black 30%, transparent 75%)',
        }}
      />
    </div>
  )
}
