#!/usr/bin/env bash
# Convert the canonical waterfall.ogg into a macOS-native format AVAudioFile
# can decode.
#
# We use AAC-in-M4A: smaller than WAV (~8 MB vs ~150 MB), Apple-native, and
# AVAudioEngine's PCM-buffer-with-`.loops` makes the seam sample-accurate
# regardless of any encoder padding.
#
# Run from `apps/macos/`:
#   ./scripts/build-asset.sh
#
# Requires (on a Mac):
#   - ffmpeg  (brew install ffmpeg)   — OR afconvert if you'd rather use Apple's tools
#
# The output lives at CascadeMac/Resources/waterfall.m4a and is gitignored.

set -euo pipefail

cd "$(dirname "$0")/.."           # → apps/macos
SOURCE="../web/public/sounds/waterfall.ogg"
OUT="CascadeMac/Resources/waterfall.m4a"

mkdir -p CascadeMac/Resources

if [ -f "$OUT" ] && [ "$OUT" -nt "$SOURCE" ]; then
    echo "✓ $OUT is up to date"
    exit 0
fi

if ! command -v ffmpeg >/dev/null 2>&1; then
    cat <<'EOF' >&2
ffmpeg not found. Install with: brew install ffmpeg

Alternative: use macOS's afconvert (no install needed) to produce a CAF file:

    afconvert -f caff -d aac -b 128000 ../web/public/sounds/waterfall.ogg \
        CascadeMac/Resources/waterfall.caf

Note: afconvert can't read .ogg directly; you'd need to decode to .wav first.
ffmpeg is the path of least resistance.
EOF
    exit 1
fi

echo "→ converting $SOURCE → $OUT"
ffmpeg -y -i "$SOURCE" -c:a aac -b:a 128k -movflags +faststart "$OUT"

echo "✓ Wrote $OUT ($(du -h "$OUT" | cut -f1))"
