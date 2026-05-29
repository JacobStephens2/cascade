#!/usr/bin/env bash
# One-shot: convert the asset, cross-compile the Rust core for every Apple
# triple, regenerate the UniFFI Swift bindings, and (if xcodegen is installed)
# regenerate the Xcode project with both CascadeMac and CascadeiOS targets.
# After this, `open Cascade.xcodeproj` and pick a scheme.
#
# Run from `apps/apple/`:
#   ./scripts/build.sh           # build everything
#   ./scripts/build.sh macos     # macOS Rust only
#   ./scripts/build.sh ios       # iOS Rust only

set -euo pipefail
cd "$(dirname "$0")/.."

./scripts/build-asset.sh
./scripts/build-rust.sh "$@"

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
