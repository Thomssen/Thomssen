#!/usr/bin/env bash
# Renders frames, synthesises the score, and muxes the final film.
set -euo pipefail
cd "$(dirname "$0")"
tmp="$(mktemp -d)"
python3 audio.py "$tmp/score.wav"
python3 render.py "$tmp/silent.mp4"
ffmpeg -y -loglevel error -i "$tmp/silent.mp4" -i "$tmp/score.wav" -c:v copy -c:a aac -b:a 256k -shortest -movflags +faststart thomsen-software-advert.mp4
echo "wrote advert/thomsen-software-advert.mp4"
