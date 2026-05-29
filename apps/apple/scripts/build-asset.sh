#!/usr/bin/env bash
# Convert the canonical waterfall.ogg into a macOS/iOS-native format
# AVAudioFile can decode.
#
# AAC-in-M4A: smaller than WAV (~8 MB vs ~150 MB), Apple-native, and
# AVAudioEngine's PCM-buffer-with-`.loops` makes the seam sample-accurate
# regardless of any encoder padding.
#
# The same file is bundled into both the macOS and iOS targets — written
# into each Resources/ directory so xcodegen picks it up per-target.
#
# Run from `apps/apple/`:
#   ./scripts/build-asset.sh

set -euo pipefail

cd "$(dirname "$0")/.."           # → apps/apple
SOURCE="../web/public/sounds/waterfall.ogg"

OUTS=(
    "CascadeMac/Resources/waterfall.m4a"
    "CascadeiOS/Resources/waterfall.m4a"
)

if ! command -v ffmpeg >/dev/null 2>&1; then
    cat <<'EOF' >&2
ffmpeg not found. Install with: brew install ffmpeg
EOF
    exit 1
fi

# Decode once, then copy to each output. Keeps the two bundled assets
# bit-identical.
TMP="$(mktemp -t cascade.XXXXXX.m4a)"
trap 'rm -f "$TMP"' EXIT

echo "→ converting $SOURCE → AAC m4a"
ffmpeg -y -i "$SOURCE" -c:a aac -b:a 128k -movflags +faststart "$TMP"

for OUT in "${OUTS[@]}"; do
    mkdir -p "$(dirname "$OUT")"
    cp "$TMP" "$OUT"
    echo "✓ Wrote $OUT ($(du -h "$OUT" | cut -f1))"
done
