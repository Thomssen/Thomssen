"""Thomsen Software advertisement: synthesised score and sound design."""
import sys
import wave
import numpy as np

SR = 48000
sys.path.insert(0, __file__.rsplit("/", 1)[0])
from render import DURATION, SCENES, SPEED  # noqa: E402

N = int(DURATION * SR)
rng = np.random.default_rng(7)
T = np.arange(N) / SR
SC = {n: s for n, s, _ in SCENES}
END = {n: s + l for n, s, l in SCENES}
BPM = 118
BEAT = 60 / BPM
L0 = SC["logo"]
CK = L0 + 2.95 / SPEED["logo"]      # the click, in real seconds
H0 = SC["hook"]


def env(a, b, c, d):
    x = np.clip((T - a) / max(b - a, 1e-6), 0, 1) * (1 - np.clip((T - c) / max(d - c, 1e-6), 0, 1))
    return x * x * (3 - 2 * x)


def midi(m):
    return 440 * 2 ** ((m - 69) / 12)


def tone(f, harm=(1, .5, .25, .12), det=0.0015):
    out = np.zeros(N)
    for k, h in enumerate(harm, 1):
        out += h * (np.sin(2 * np.pi * f * k * (1 - det) * T) + np.sin(2 * np.pi * f * k * (1 + det) * T + 1.3)) / 2
    return out


def at(t0, fn_decay, f, amp, harm=(1,), sweep=0.0):
    """Percussive decaying tone starting at t0."""
    x = np.clip(T - t0, 0, None)
    out = np.zeros(N)
    for k, h in enumerate(harm, 1):
        out += h * np.sin(2 * np.pi * (f * k + sweep * np.exp(-x * 30)) * x)
    return out * np.exp(-x * fn_decay) * (T >= t0) * amp


# filtered noise used for risers and air
noise = rng.standard_normal(N)
spec = np.fft.rfft(noise)
fr = np.fft.rfftfreq(N, 1 / SR)
air = np.fft.irfft(spec / (1 + (np.abs(fr - 3000) / 2200) ** 2), N)
hiss = np.fft.irfft(spec * (fr > 6000), N)

# ---------------------------------------------------------------- pad + bass
chords = [[38, 45, 50, 57], [34, 41, 50, 57], [31, 38, 46, 53], [33, 40, 45, 52],
          [38, 45, 50, 57], [38, 45, 53, 57]]
names = ["shield", "vpn", "osint", "vm", "unified", "end"]
bounds = []
for i, n in enumerate(names):
    s = SC[n]
    e = SC[names[i + 1]] if i + 1 < len(names) else DURATION + 1
    bounds.append((s, e))
pad = np.zeros(N)
bass = np.zeros(N)
for ch, (a, b) in zip(chords, bounds):
    e = env(a - 1.0, a + 0.8, b - 0.8, b + 0.8)
    for m in ch:
        pad += tone(midi(m)) * e * 0.06
    bass += np.sin(2 * np.pi * midi(ch[0] - 12) * T) * e * 0.20
pad *= 0.7 + 0.3 * env(CK, CK + 3, DURATION - 3, DURATION)
intro_pad = tone(midi(38)) * env(CK - 1, CK + 0.6, SC["hero"] + 1, SC["hero"] + 2.5) * 0.05

# ---------------------------------------------------------------- opening
click = np.zeros(N)
for tc, g in ((CK - 0.02, 1.0), (CK + 0.07, 0.6)):
    x = np.clip(T - tc, 0, None)
    click += (np.sin(2 * np.pi * 2100 * x) * 0.6 + rng.standard_normal(N) * 0.4) * np.exp(-x * 240) * (T >= tc) * 0.07 * g
riser = air * env(H0 + 2.9, CK, CK, CK + 0.1) * 0.12 + hiss * env(H0 + 3.3, CK, CK, CK + 0.05) * 0.04
hook_hits = sum(at(H0 + b, 3.5, 52, 0.45, sweep=45) + at(H0 + b, 14, 180, 0.04, (1, .5)) for b in (0.25, 1.0, 1.8, 3.1, 3.7))
hook_drone = np.sin(2 * np.pi * midi(26) * T) * env(0.0, 1.0, CK, CK + 0.3) * 0.12 + tone(midi(37), (1, .5)) * env(0.3, 2.5, CK, CK + 0.3) * 0.04
click_hit = at(CK, 2.2, 46, 0.55, sweep=40) + at(CK, 1.2, 92, 0.12)
open_whoosh = air * env(CK + 0.1, CK + 0.6, CK + 0.6, CK + 1.6) * 0.08
shimmer = (at(CK + 0.5, 1.4, midi(86), 0.012, (1, .3)) + at(CK + 0.6, 1.4, midi(93), 0.008, (1, .3)))
t_chime = at(CK + 1.2, 1.3, midi(81), 0.03, (1, .4, .15)) + at(CK + 1.3, 1.3, midi(86), 0.025, (1, .4, .15))
# T tile fading in
tile = at(L0 + 0.4, 3.0, midi(74), 0.018, (1, .3))

# ---------------------------------------------------------------- scene changes
whoosh = np.zeros(N)
impacts = np.zeros(N)
for name, s, _ in SCENES[1:]:
    whoosh += air * env(s - 0.55, s - 0.02, s, s + 0.35) * 0.10
    whoosh += hiss * env(s - 0.45, s - 0.02, s, s + 0.15) * 0.025
    impacts += at(s, 3.0, 50, 0.35, sweep=30) + at(s, 2.0, 100, 0.08)
ticks = np.zeros(N)
for name, s, _ in SCENES[1:]:
    for off, m in ((0.30, 86), (0.50, 81), (0.75, 93)):
        ticks += at(s + off, 6, midi(m), 0.025, (1, .3))

# ---------------------------------------------------------------- rhythm
rhythm_on = env(CK - 0.05, CK + 0.6, END["unified"] + 0.4, END["unified"] + 1.4)
kick = np.zeros(N)
hat = np.zeros(N)
pluck = np.zeros(N)
arp = [62, 65, 69, 72, 74, 72, 69, 65]
t0 = CK
i = 0
while t0 < DURATION - 3:
    on = rhythm_on[min(int(t0 * SR), N - 1)]
    if on > 0.01:
        # soft kick on every beat, hat on the off-beat
        kick += at(t0, 9, 55, 0.30 * on, sweep=60)
        x = np.clip(T - (t0 + BEAT / 2), 0, None)
        hat += hiss * np.exp(-x * 60) * (T >= t0 + BEAT / 2) * 0.05 * on
        for j in range(2):
            m = arp[(i * 2 + j) % len(arp)] + (12 if (i % 8) == 7 and j == 1 else 0)
            pluck += at(t0 + j * BEAT / 2, 4.0, midi(m), 0.03 * on, (1, .3, .1))
    t0 += BEAT
    i += 1

# closing lift
lift = np.zeros(N)
for m in (62, 69, 74, 81):
    lift += tone(midi(m)) * env(SC["end"] + 0.2, SC["end"] + 2.4, SC["end"] + 3.2, DURATION - 0.1) * 0.04
end_hit = at(SC["end"] + 0.6, 2.4, 49, 0.4, sweep=35)
logo_hit = at(SC["end"] + 3.2, 1.4, 49, 0.3, sweep=35) + at(SC["end"] + 3.2, 1.0, midi(86), 0.02, (1, .3))

dry = (hook_hits + hook_drone + click + click_hit + riser + open_whoosh + shimmer + t_chime + tile + pad + intro_pad + bass
       + whoosh + impacts + ticks + kick + hat + pluck + lift + end_hit + logo_hit)

# reverb tail
ir_n = int(2.8 * SR)
ir = rng.standard_normal(ir_n) * np.exp(-np.arange(ir_n) / SR * 2.2)
ir[: int(0.03 * SR)] = 0
wet = np.fft.irfft(np.fft.rfft(dry, N + ir_n) * np.fft.rfft(ir, N + ir_n), N + ir_n)[:N]
wet *= np.abs(dry).max() / (np.abs(wet).max() + 1e-9) * 0.85
L = dry + wet * 0.8
R = dry + np.roll(wet, int(0.011 * SR)) * 0.8
if len(sys.argv) > 2:
    import soundfile as sf
    vo, vsr = sf.read(sys.argv[2], dtype="float32")
    vo = np.pad(vo, (0, max(0, N - len(vo))))[:N]
    # music ducks under the voice
    w = int(0.25 * SR)
    cs = np.cumsum(np.abs(vo))
    ma = np.concatenate([np.zeros(w // 2), (cs[w:] - cs[:-w]) / w, np.zeros(N - (N - w) - w // 2)])[:N]
    duck = 1 - 0.55 * np.clip(ma / 0.04, 0, 1)
    k = int(0.15 * SR)
    duck = np.convolve(duck, np.ones(k) / k, mode="same")
    L = L * duck * 0.8
    R = R * duck * 0.8
    vir = rng.standard_normal(int(0.9 * SR)) * np.exp(-np.arange(int(0.9 * SR)) / SR * 5)
    vwet = np.fft.irfft(np.fft.rfft(vo, N + len(vir)) * np.fft.rfft(vir, N + len(vir)), N + len(vir))[:N]
    vwet *= np.abs(vo).max() / (np.abs(vwet).max() + 1e-9) * 0.12
    L = L + vo + vwet
    R = R + vo + vwet
for ch in (L, R):
    ch *= 1 - np.clip((T - (DURATION - 2.0)) / 2.0, 0, 1)
st = np.stack([L, R], 1)
st = np.tanh(st * 1.1 / np.abs(st).max() * 1.3) / np.tanh(1.3)   # gentle glue
st = st / np.abs(st).max() * 0.9
pcm = (st * 32767).astype("<i2")
with wave.open(sys.argv[1], "wb") as w:
    w.setnchannels(2)
    w.setsampwidth(2)
    w.setframerate(SR)
    w.writeframes(pcm.tobytes())
print("audio ok", DURATION)
