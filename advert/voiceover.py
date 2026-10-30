"""Thomsen Software advertisement: voice-over.

Synthesises the narration with Kokoro (open neural text-to-speech) and places each
line on the film's timeline. Usage: voiceover.py <model_dir> <out.wav>
"""
import json
import subprocess
import sys
import wave

import numpy as np
import soundfile as sf
from kokoro_onnx import Kokoro

sys.path.insert(0, __file__.rsplit("/", 1)[0])
from render import DURATION, SCENES  # noqa: E402

VOICE = "af_heart"
SPEED = 0.97
SR = 48000
SC = {n: s for n, s, _ in SCENES}

# (scene, seconds into scene, text)
LINES = [
    ("hook", 0.20, "Right now, your PC is talking to the internet."),
    ("hook", 3.25, "Do you know who to?"),
    ("logo", 2.00, "Meet Thomsen Software."),
    ("hero", 0.25, "Software you can actually trust."),
    ("shield", 0.40, "Thomsen Shield. See every connection. Stop the ones you do not trust."),
    ("vpn", 0.40, "Thomsen VPN. Your traffic, encrypted. One click to connect. Coming soon."),
    ("osint", 0.40, "Thomsen OSINT. Public information, organized into one clear picture."),
    ("vm", 0.40, "Thomsen VM. A full machine, running inside your desktop. Free."),
    ("unified", 0.30, "Quiet on your system. Clear in your results."),
    ("end", 0.50, "Software. Designed with purpose."),
    ("end", 3.70, "Thomsen Software dot com."),
]


def main(model_dir, out):
    k = Kokoro(f"{model_dir}/kokoro-v1.0.onnx", f"{model_dir}/voices-v1.0.bin")
    track = np.zeros(int(DURATION * SR) + SR, np.float32)
    ends = {s: s_end for s, st, ln in SCENES for s_end in [st + ln]}
    report = []
    for scene, off, text in LINES:
        samples, sr = k.create(text, voice=VOICE, speed=SPEED, lang="en-us")
        sf.write("/tmp/_vo_in.wav", samples, sr)
        subprocess.run(["ffmpeg", "-y", "-loglevel", "error", "-i", "/tmp/_vo_in.wav", "-ar", str(SR),
                        "-ac", "1", "/tmp/_vo_out.wav"], check=True)
        with wave.open("/tmp/_vo_out.wav") as w:
            x = np.frombuffer(w.readframes(w.getnframes()), "<i2").astype(np.float32) / 32768
        t0 = SC[scene] + off
        i0 = int(t0 * SR)
        track[i0:i0 + len(x)] += x
        report.append((scene, round(t0, 2), round(t0 + len(x) / SR, 2), round(ends[scene], 2), text))
    for r in report:
        flag = "  <-- overruns scene" if r[2] > r[3] + 0.05 else ""
        print(r, flag)
    track = track[: int(DURATION * SR)]
    sf.write("/tmp/_vo_track.wav", track, SR)
    # broadcast-style polish: rumble filter, gentle compression, safety limiter
    subprocess.run(["ffmpeg", "-y", "-loglevel", "error", "-i", "/tmp/_vo_track.wav", "-af",
                    "highpass=f=85,acompressor=threshold=0.06:ratio=3:attack=8:release=140:makeup=2.2,"
                    "alimiter=limit=0.9", out], check=True)


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
