"""Thomsen Software advertisement: synthesised score and sound design."""
import sys
import wave
import numpy as np

SR = 48000
sys.path.insert(0, __file__.rsplit("/", 1)[0])
from render import DURATION, SCENES  # noqa: E402

N = int(DURATION * SR)
rng = np.random.default_rng(7)
T = np.arange(N) / SR


def env(a, b, c, d):
    """Piecewise-smooth envelope over time: rise a..b, hold, fall c..d."""
    x = np.clip((T - a) / max(b - a, 1e-6), 0, 1) * (1 - np.clip((T - c) / max(d - c, 1e-6), 0, 1))
    return x * x * (3 - 2 * x)


def tone(f, harm=(1, .5, .25, .12), det=0.0015):
    out = np.zeros(N)
    for k, h in enumerate(harm, 1):
        out += h * (np.sin(2 * np.pi * f * k * (1 - det) * T) + np.sin(2 * np.pi * f * k * (1 + det) * T + 1.3)) / 2
    return out


def midi(m):
    return 440 * 2 ** ((m - 69) / 12)


# pad: slow chord changes, one per scene
chords = [[38, 45, 50, 57], [34, 41, 50, 57], [31, 38, 46, 53], [33, 40, 45, 52],
          [38, 45, 50, 57], [38, 45, 53, 57]]
SC = {n: s for n, s, _ in SCENES}
bounds = [(0, SC["shield"]), (SC["shield"], SC["vpn"]), (SC["vpn"], SC["osint"]), (SC["osint"], SC["vm"]), (SC["vm"], SC["unified"]), (SC["unified"], DURATION + 1)]
pad = np.zeros(N)
for ch, (a, b) in zip(chords, bounds):
    e = env(a - 1.5, a + 1.5, b - 1.5, b + 1.5) if a > 0 else env(0.5, 4.5, b - 1.5, b + 1.5)
    for m in ch:
        pad += tone(midi(m)) * e * 0.045
sub = np.sin(2 * np.pi * midi(26) * T) * env(0.8, 5, DURATION - 6, DURATION - 0.5) * 0.16

# opening: swell into a soft low hit as the logo resolves
swell = rng.standard_normal(N)
sp = np.fft.rfft(swell); fr = np.fft.rfftfreq(N, 1 / SR)
swell = np.fft.irfft(sp * (1 / (1 + (np.abs(fr - 2500) / 1800) ** 2)), N)
open_fx = swell * env(0.6, 2.9, 2.9, 3.2) * 0.07
hit_t = np.clip(T - 2.97, 0, None)
hit = np.sin(2 * np.pi * (46 + 30 * np.exp(-hit_t * 6)) * hit_t) * np.exp(-hit_t * 1.1) * (T > 2.97) * 0.5
shimmer = tone(midi(86), (1, .3)) * np.exp(-hit_t * 0.8) * (T > 2.97) * 0.012

# mouse click: a tiny down/up pair, then the wordmark opens with a soft whoosh
click = np.zeros(N)
for tc, g in ((2.93, 1.0), (3.03, 0.6)):
    x = np.clip(T - tc, 0, None)
    click += (np.sin(2 * np.pi * 2100 * x) * 0.6 + rng.standard_normal(N) * 0.4) * np.exp(-x * 220) * (T >= tc) * 0.05 * g
open_fx += swell * env(3.1, 3.9, 3.9, 5.0) * 0.05
# soft whooshes at scene changes
wh = np.zeros(N)
for _, s, _ in SCENES[1:]:
    wh += swell * env(s - 1.4, s - 0.1, s, s + 0.9) * 0.06
# gentle tick as each scene's text arrives
ticks = np.zeros(N)
for _, s, _ in SCENES[1:]:
    for off, m in ((0.7, 86), (1.0, 81)):
        t0 = s + off
        x = np.clip(T - t0, 0, None)
        ticks += np.sin(2 * np.pi * midi(m) * x) * np.exp(-x * 5) * (T >= t0) * 0.02

# sparse pentatonic pluck pattern from the first product on
notes = [62, 65, 69, 72, 74, 69, 65, 72]
pluck = np.zeros(N)
beat = 60 / 84
t0 = SC["hero"]
i = 0
while t0 < DURATION - 6:
    m = notes[(i * 3) % len(notes)] + (12 if i % 7 == 6 else 0)
    x = np.clip(T - t0, 0, None)
    amp = 0.022 * (0.6 + 0.4 * np.sin(i * 0.7) ** 2)
    pluck += (np.sin(2 * np.pi * midi(m) * x) + 0.3 * np.sin(2 * np.pi * midi(m) * 2 * x)) * np.exp(-x * 3.2) * (T >= t0) * amp
    t0 += beat * 2
    i += 1
pluck *= env(SC["hero"] - 0.5, SC["hero"] + 2, DURATION - 9, DURATION - 3)

# closing: one lifted chord as the logo lands
lift = np.zeros(N)
for m in (62, 69, 74, 81):
    lift += tone(midi(m)) * env(SC["end"] + 0.5, SC["end"] + 4, SC["end"] + 5, DURATION - 0.1) * 0.03

dry = click + pad + sub + open_fx + hit + shimmer + wh + ticks + pluck + lift
# echo + reverb
ir_n = int(3.2 * SR)
ir = rng.standard_normal(ir_n) * np.exp(-np.arange(ir_n) / SR * 2.0)
ir[: int(0.03 * SR)] = 0
wet = np.fft.irfft(np.fft.rfft(dry, N + ir_n) * np.fft.rfft(ir, N + ir_n), N + ir_n)[:N]
wet *= np.abs(dry).max() / (np.abs(wet).max() + 1e-9) * 0.9
L = dry + wet * 0.9
# stereo width from a tiny delay and detuned wet
R = dry + np.roll(wet, int(0.011 * SR)) * 0.9
for ch in (L, R):
    ch *= 1 - np.clip((T - (DURATION - 2.5)) / 2.5, 0, 1)
st = np.stack([L, R], 1)
st = st / np.abs(st).max() * 0.8
pcm = (st * 32767).astype("<i2")
with wave.open(sys.argv[1], "wb") as w:
    w.setnchannels(2); w.setsampwidth(2); w.setframerate(SR); w.writeframes(pcm.tobytes())
print("audio ok", DURATION)
