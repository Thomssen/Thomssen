#!/usr/bin/env python3
"""Original score + sound design for the Thomsen Shield commercial.

Everything is synthesised from scratch with numpy (no samples, no third-party
music), so there is nothing to license: the track is yours to use commercially.
100 BPM, A minor / open fifths.  Cue sheet is locked to render.py's timeline.

    python3 audio.py out/shield-score.wav
"""
import sys
import wave
import numpy as np

SR = 48000
DUR = 52.8
N = int(SR * DUR)
BEAT = 0.6
BAR = 2.4
rng = np.random.default_rng(5)


def tt(d):
    return np.arange(int(d * SR)) / SR


def db(x):
    return 10 ** (x / 20)


def place(buf, sig, t0, gain=1.0, pan=0.0):
    i = int(t0 * SR)
    if i >= buf.shape[1]:
        return
    n = min(len(sig), buf.shape[1] - i)
    l = gain * np.cos((pan + 1) * np.pi / 4)
    r = gain * np.sin((pan + 1) * np.pi / 4)
    buf[0, i:i + n] += sig[:n] * l
    buf[1, i:i + n] += sig[:n] * r


def env_adsr(n, a, d, s, r):
    e = np.ones(n)
    na, nd, nr = int(a * SR), int(d * SR), int(r * SR)
    e[:na] = np.linspace(0, 1, max(na, 1)) ** 2
    e[na:na + nd] = np.linspace(1, s, max(nd, 1))
    e[na + nd:] = s
    if nr:
        e[-nr:] *= np.linspace(1, 0, nr) ** 1.5
    return e


def fft_filter(x, kind, fc, order=4):
    X = np.fft.rfft(x)
    f = np.fft.rfftfreq(len(x), 1 / SR)
    if kind == "lp":
        H = 1 / np.sqrt(1 + (f / fc) ** (2 * order))
    elif kind == "hp":
        H = 1 / np.sqrt(1 + (fc / np.maximum(f, 1e-3)) ** (2 * order))
    return np.fft.irfft(X * H, len(x))


def swept_bandpass(noise, f0, f1, q=1.2, curve=2.0):
    """STFT-style band-pass whose centre sweeps f0 -> f1 (exponential)."""
    n = len(noise)
    blk = 4096
    hop = blk // 2
    win = np.hanning(blk)
    out = np.zeros(n + blk)
    fr = np.fft.rfftfreq(blk, 1 / SR)
    for s in range(0, n - blk, hop):
        u = (s / n) ** curve
        fc = f0 * (f1 / f0) ** u
        bw = fc / q
        H = np.exp(-0.5 * ((fr - fc) / bw) ** 2)
        seg = noise[s:s + blk] * win
        out[s:s + blk] += np.fft.irfft(np.fft.rfft(seg) * H, blk) * win
    return out[:n] / 0.75


def reverb(stereo, rt=3.2, mix=0.35, pre=0.02, damp=5200):
    n = stereo.shape[1]
    ir_len = int(rt * 1.1 * SR)
    wet = np.zeros_like(stereo)
    nfft = 1 << int(np.ceil(np.log2(n + ir_len)))
    for ch in range(2):
        t = np.arange(ir_len) / SR
        ir = rng.standard_normal(ir_len) * np.exp(-6.9 * t / rt)
        ir = fft_filter(ir, "lp", damp, 2)
        ir[: int(pre * SR)] = 0
        ir /= np.sqrt(np.sum(ir ** 2)) + 1e-9
        c = np.fft.irfft(np.fft.rfft(stereo[ch], nfft) * np.fft.rfft(ir, nfft), nfft)[:n]
        wet[ch] = c
    return stereo * (1 - mix * 0.5) + wet * mix * 1.6


# ---------------------------------------------------------------- voices
def saw_pad(freq, d, detune=(-6, 0, 7), harm=14):
    t = tt(d)
    sig = np.zeros_like(t)
    for c in detune:
        f = freq * 2 ** (c / 1200)
        ph = rng.uniform(0, 2 * np.pi)
        for h in range(1, harm + 1):
            if f * h > 5500:
                break
            sig += np.sin(2 * np.pi * f * h * t + ph * h) / h
    return sig / len(detune)


def pluck(freq, d=0.9):
    t = tt(d)
    e = np.exp(-t / 0.20)
    sig = np.sin(2 * np.pi * freq * t) + 0.35 * np.sin(2 * np.pi * freq * 2 * t) * np.exp(-t / 0.08)
    sig += 0.12 * np.sin(2 * np.pi * freq * 3.01 * t) * np.exp(-t / 0.05)
    return sig * e


def kick(d=0.5, f0=140, f1=44):
    t = tt(d)
    ph = 2 * np.pi * (f1 * t + (f0 - f1) * 0.045 * (1 - np.exp(-t / 0.045)))
    s = np.sin(ph) * np.exp(-t / 0.16)
    s += 0.25 * rng.standard_normal(len(t)) * np.exp(-t / 0.004)
    return fft_filter(s, "lp", 900, 2)


def hat(d=0.12):
    t = tt(d)
    return fft_filter(rng.standard_normal(len(t)), "hp", 7500, 2) * np.exp(-t / 0.025)


def tick(freq=2600, d=0.08):
    t = tt(d)
    return (np.sin(2 * np.pi * freq * t) + 0.4 * np.sin(2 * np.pi * freq * 1.5 * t)) * np.exp(-t / 0.012)


def boom(d=4.5):
    t = tt(d)
    ph = 2 * np.pi * (46 * t + 70 * 0.09 * (1 - np.exp(-t / 0.09)))
    s = np.sin(ph) * np.exp(-t / 1.6)
    s += 0.5 * np.sin(2 * np.pi * 23 * t) * np.exp(-t / 2.2)
    th = fft_filter(rng.standard_normal(len(t)), "lp", 420, 3) * np.exp(-t / 0.35) * 2.0
    return s + th


def riser(d, f0=250, f1=9000):
    n = int(d * SR)
    nz = rng.standard_normal(n)
    s = swept_bandpass(nz, f0, f1, q=1.4, curve=1.6)
    e = np.linspace(0, 1, n) ** 2.4
    return s * e


def whoosh(d, up=True):
    n = int(d * SR)
    nz = rng.standard_normal(n)
    s = swept_bandpass(nz, 300 if up else 4000, 4000 if up else 300, q=0.9, curve=1.0)
    e = np.sin(np.linspace(0, np.pi, n)) ** 2
    return s * e


def midi(n):
    return 440 * 2 ** ((n - 69) / 12)


# ---------------------------------------------------------------- score
def build():
    dry = np.zeros((2, N))     # goes through the reverb
    dir_ = np.zeros((2, N))    # stays dry (kick / sub)

    # room tone + sub drone (A1) fades in under the opening hairline
    t = np.arange(N) / SR
    drone = np.sin(2 * np.pi * 55 * t) + 0.35 * np.sin(2 * np.pi * 110 * t + 0.3)
    drone *= np.clip((t - 0.6) / 4.0, 0, 1) * 0.55
    drone *= np.where(t < 9.6, 1.0, 0.8)
    for ch in range(2):
        dir_[ch] += drone * db(-27)

    # pad progression: A5 | F5 | C5 | G5 (open fifths), 2 bars each from 9.6
    chords = [[45, 52, 59, 64], [41, 48, 55, 62], [48, 55, 62, 69], [43, 50, 57, 64]]
    # prelude pad (A) from 4.8 swelling into the cut
    def pad_chord(notes, t0, d, g, a=1.6, r=2.0):
        for j, n in enumerate(notes):
            s = saw_pad(midi(n), d + r)
            s = fft_filter(s, "lp", 1100 + 500 * (j % 2), 3)
            s *= env_adsr(len(s), a, 0.4, 0.8, r)
            place(dry, s, t0, g * db(-31), pan=(-0.5 + 0.33 * j))
    pad_chord(chords[0], 4.8, 4.8, 0.55, a=3.0, r=0.3)
    # pad drops out on the silence before the reveal (9.2–9.6), then opens
    t0 = 9.6
    for i in range(0, 12):
        c = chords[i % 4]
        if t0 >= 45.6 - 1e-6 and t0 < 45.7:
            break
        pad_chord(c, t0, 4.8, 1.0 if t0 < 40.8 else 0.7)
        t0 += 4.8
        if t0 >= 40.8:
            break
    # tagline pad, then final chord (Asus2 / open A) with long tail
    pad_chord([45, 52, 57, 59, 64], 40.8, 4.8, 0.7, a=2.0, r=1.0)
    pad_chord([33, 45, 52, 59, 64, 69], 45.6, 7.2, 1.15, a=0.5, r=3.0)

    # arpeggio (A minor pentatonic), 8ths, from 4.8, gap before the reveal
    pat = [69, 76, 72, 79, 76, 72, 81, 76]
    def arp(t_start, t_end, g, step=0.3):
        i = 0
        t0 = t_start
        while t0 < t_end:
            n = pat[i % len(pat)]
            place(dry, pluck(midi(n)), t0, g * db(-34), pan=0.4 * np.sin(i * 0.7))
            i += 1
            t0 += step
    arp(4.8, 9.0, 0.7)
    arp(12.0, 40.8, 1.0)
    arp(45.6, 50.4, 0.8)

    # drums: soft 4/4 kick + offbeat hats between 16.8 and 40.8
    t0 = 16.8
    while t0 < 40.8 - 1e-6:
        place(dir_, kick(), t0, db(-17), 0.0)
        place(dry, hat(), t0 + BEAT / 2, db(-38), 0.3)
        t0 += BEAT
    # sub heartbeat before the first reveal
    for tb in (7.8, 8.4, 9.0):
        place(dir_, kick(0.6, 90, 40), tb, db(-20) * (1 + (tb - 7.8) * 0.4))

    # impacts and risers
    place(dry, riser(1.8, 300, 8500), 7.8, db(-24), 0)
    place(dir_, boom(), 9.6, db(-10), 0)
    place(dry, boom(), 9.6, db(-24), 0)
    place(dry, riser(1.6, 400, 9500), 44.0, db(-25), 0)
    place(dir_, boom(), 45.6, db(-8), 0)
    place(dry, boom(), 45.6, db(-22), 0)
    # transition whooshes: window rise, web -> app, DNS state change
    place(dry, whoosh(1.4, True), 11.9, db(-35), 0)
    place(dry, whoosh(1.8, True), 15.2, db(-30), 0)
    place(dir_, boom(2.5), 16.5, db(-30), 0)
    place(dry, whoosh(0.9, False), 40.2, db(-34), 0)

    # UI ticks locked to text/caption entrances
    cues = [2.4, 3.6, 4.8, 7.0, 10.9, 12.4, 17.0, 17.3, 22.2, 24.6, 29.4, 30.65,
            34.2, 36.6, 41.0, 42.3, 47.0, 48.2, 49.0]
    for c in cues:
        place(dry, tick(2400 + 300 * (hash(c) % 3)), c, db(-33), 0.5 * np.sin(c))
    # ring pings for the "detection" rings
    for c in (6.5, 7.35, 8.2):
        place(dry, pluck(midi(93), 1.5), c, db(-30), 0.2)

    # hairline shimmer
    sh = fft_filter(rng.standard_normal(int(3 * SR)), "hp", 6000, 2) * np.sin(np.linspace(0, np.pi, int(3 * SR))) ** 2
    place(dry, sh, 0.8, db(-48), 0)

    # sidechain: duck pad/arp on each kick
    duck = np.ones(N)
    t0 = 16.8
    k = np.exp(-tt(0.35) / 0.09)
    while t0 < 40.8:
        i = int(t0 * SR)
        n = min(len(k), N - i)
        duck[i:i + n] -= 0.28 * k[:n]
        t0 += BEAT
    dry *= duck

    mix = reverb(dry, rt=3.4, mix=0.42) + dir_
    # fades
    f = np.ones(N)
    f[-int(2.2 * SR):] = np.linspace(1, 0, int(2.2 * SR)) ** 1.6
    f[: int(0.5 * SR)] = np.linspace(0, 1, int(0.5 * SR))
    mix *= f
    # glue: gentle tanh limiter, normalise to -1.5 dBFS peak
    mix = np.tanh(mix * 1.15) / np.tanh(1.15)
    mix /= np.max(np.abs(mix))
    mix *= db(-1.5)
    return mix


def write_wav(path, x):
    pcm = (np.clip(x, -1, 1) * 32767).astype("<i2").T.copy()
    with wave.open(path, "wb") as w:
        w.setnchannels(2); w.setsampwidth(2); w.setframerate(SR)
        w.writeframes(pcm.tobytes())


if __name__ == "__main__":
    out = sys.argv[1] if len(sys.argv) > 1 else "out/shield-score.wav"
    x = build()
    write_wav(out, x)
    rms = 20 * np.log10(np.sqrt(np.mean(x ** 2)))
    print(f"wrote {out}  {DUR}s  rms {rms:.1f} dBFS  peak {20*np.log10(np.max(np.abs(x))):.1f} dBFS")
