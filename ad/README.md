# Thomsen Shield — commercial (52.8 s, 16:9)

Rendered from the real website + application recordings. See [`CONCEPT.md`](CONCEPT.md) for the storyboard.

| File | What |
|---|---|
| `out/thomsen-shield-ad-1080p-web.mp4` | 1920×1080 · 30 fps · H.264 + AAC · ~15 MB — website / YouTube / social |
| `out/thomsen-shield-ad-1080p.mp4` | same, high-bitrate master (kept local, not in git) |
| `out/thomsen-shield-ad-4k.mp4` | 3840×2160 master (kept local, not in git) |
| `out/shield-score.wav` | the original score + sound design, stereo 48 kHz |

## Rebuild

```bash
mkdir -p source
cp <website recording> source/website-recording.mp4
cp <app recording>     source/app-recording.mp4

python3 audio.py out/shield-score.wav                          # ~15 s
python3 render.py --scale 1 --out out/silent-1080.mp4          # ~8 min on 4 cores
python3 render.py --scale 2 --out out/silent-4k.mp4 --crf 17   # 4K, ~45 min
ffmpeg -i out/silent-1080.mp4 -i out/shield-score.wav -map 0:v -map 1:a \
       -c:v copy -c:a aac -b:a 320k -shortest -movflags +faststart ad.mp4
```

Needs: Python 3 + numpy + Pillow (with Raqm), ffmpeg, Inter / Inter Display fonts.
`python3 render.py --stills 10.4,22.5` dumps QA frames to `out/stills/`.

## Swapping the soundtrack
The cut points sit on a 100 BPM grid (bar = 2.4 s; impacts at 9.6 s and 45.6 s). Any licensed track at 100 BPM
drops straight in with the last `ffmpeg` line above.

## Social cuts
Everything is a function of time in `render.py` — for 9:16 / 1:1, change `W, H` and re-anchor the `px, py`
arguments of `place_plate` and the caption `X` positions.
