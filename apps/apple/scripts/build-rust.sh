#!/usr/bin/env bash
# Cross-compile `cascade-uniffi` for every Apple target the project needs and
# regenerate the Swift bindings.
#
# Output layout (relative to apps/apple/):
#   build/rust/macos/libcascade_uniffi.a       # universal macOS
#   build/rust/ios-device/libcascade_uniffi.a  # arm64 iPhone
#   build/rust/ios-sim/libcascade_uniffi.a     # universal iOS simulator
#   CascadeShared/Generated/cascade_uniffi*    # Swift code + header + modulemap
#
# Run from `apps/apple/`:
#   ./scripts/build-rust.sh           # build everything
#   ./scripts/build-rust.sh macos     # macOS only (fastest if iterating on Mac)
#   ./scripts/build-rust.sh ios       # iOS only
#
# Requires (on a Mac):
#   - Xcode command line tools (ld, lipo, codesign)
#   - rustup target add \
#       aarch64-apple-darwin x86_64-apple-darwin \
#       aarch64-apple-ios \
#       aarch64-apple-ios-sim x86_64-apple-ios

set -euo pipefail

cd "$(dirname "$0")/.."          # → apps/apple
REPO_ROOT="$(cd ../.. && pwd)"
GEN_DIR="$PWD/CascadeShared/Generated"

WHAT="${1:-all}"

build_target() {
    local triple="$1"
    echo "→ cargo build --target $triple"
    ( cd "$REPO_ROOT" && cargo build --release --target "$triple" -p cascade-uniffi )
}

lib() { echo "$REPO_ROOT/target/$1/release/libcascade_uniffi.a"; }

build_macos() {
    build_target aarch64-apple-darwin
    build_target x86_64-apple-darwin
    mkdir -p build/rust/macos
    echo "→ lipo macOS universal"
    lipo -create "$(lib aarch64-apple-darwin)" "$(lib x86_64-apple-darwin)" \
        -output build/rust/macos/libcascade_uniffi.a
}

build_ios() {
    build_target aarch64-apple-ios
    build_target aarch64-apple-ios-sim
    build_target x86_64-apple-ios
    mkdir -p build/rust/ios-device build/rust/ios-sim
    cp "$(lib aarch64-apple-ios)" build/rust/ios-device/libcascade_uniffi.a
    echo "→ lipo iOS simulator universal"
    lipo -create "$(lib aarch64-apple-ios-sim)" "$(lib x86_64-apple-ios)" \
        -output build/rust/ios-sim/libcascade_uniffi.a
}

case "$WHAT" in
    all)   build_macos; build_ios ;;
    macos) build_macos ;;
    ios)   build_ios ;;
    *) echo "usage: $0 [all|macos|ios]" >&2; exit 2 ;;
esac

# UniFFI Swift bindings are platform-independent (they read the .dylib's
# metadata sections, not its instructions), so regenerate from whichever
# macOS dylib we have on hand. The header + modulemap are static text.
DYLIB="$REPO_ROOT/target/aarch64-apple-darwin/release/libcascade_uniffi.dylib"
if [ ! -f "$DYLIB" ]; then
    # Fall back to a host build if the macOS triple wasn't requested.
    ( cd "$REPO_ROOT" && cargo build --release -p cascade-uniffi )
    DYLIB="$REPO_ROOT/target/release/libcascade_uniffi.dylib"
fi

echo "→ uniffi-bindgen generate (Swift)"
( cd "$REPO_ROOT" && cargo run -p cascade-uniffi --bin uniffi-bindgen --release -- generate \
    --library "$DYLIB" \
    --language swift \
    --out-dir "$GEN_DIR" )

test -f "$GEN_DIR/cascade_uniffi.swift"        || { echo "missing Swift binding"; exit 1; }
test -f "$GEN_DIR/cascade_uniffiFFI.h"         || { echo "missing C header"; exit 1; }
test -f "$GEN_DIR/cascade_uniffiFFI.modulemap" || { echo "missing modulemap"; exit 1; }

echo "✓ Done."
