#!/usr/bin/env bash
# Renders frames, synthesises the voice-over and score, and muxes the final film.
#   MODEL_DIR must hold kokoro-v1.0.onnx and voices-v1.0.bin
#   (github.com/thewh1teagle/kokoro-onnx/releases, tag model-files-v1.0)
#   pip install kokoro-onnx soundfile pillow numpy
set -euo pipefail
cd "$(dirname "$0")"
: "${MODEL_DIR:?set MODEL_DIR to the folder with the Kokoro model files}"
tmp="$(mktemp -d)"
python3 voiceover.py "$MODEL_DIR" "$tmp/vo.wav"
python3 audio.py "$tmp/mix.wav" "$tmp/vo.wav"
python3 render.py "$tmp/silent.mp4"
ffmpeg -y -loglevel error -i "$tmp/silent.mp4" -i "$tmp/mix.wav" -c:v libx264 -preset medium -crf 22 \
  -pix_fmt yuv420p -c:a aac -b:a 256k -shortest -movflags +faststart thomsen-software-advert.mp4
echo "wrote advert/thomsen-software-advert.mp4"
