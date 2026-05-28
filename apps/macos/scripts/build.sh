#!/usr/bin/env bash
# One-shot: convert the asset, cross-compile the Rust core, regenerate the
# UniFFI Swift bindings, and (if xcodegen is installed) regenerate the
# Xcode project. After this, `open Cascade.xcodeproj` and ⌘B.
#
# Run from `apps/macos/`:
#   ./scripts/build.sh

set -euo pipefail
cd "$(dirname "$0")/.."

./scripts/build-asset.sh
./scripts/build-rust.sh

if command -v xcodegen >/dev/null 2>&1; then
    echo "→ xcodegen"
    xcodegen generate
else
    cat <<'EOF'
xcodegen not found — skipping project regeneration.

Install with:  brew install xcodegen
Then run:      xcodegen generate
EOF
fi

echo
echo "Next: open Cascade.xcodeproj in Xcode, set your Team in Signing & "
echo "      Capabilities, then ⌘B / ⌘R."
