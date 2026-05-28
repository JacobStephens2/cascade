#!/usr/bin/env bash
# Cross-compile `cascade-uniffi` for both macOS architectures, lipo them
# into a universal static lib, and regenerate the Swift bindings.
#
# Run from `apps/macos/`:
#   ./scripts/build-rust.sh
#
# Requires (on a Mac):
#   - Xcode command line tools (provides ld, lipo, codesign)
#   - rustup target add aarch64-apple-darwin x86_64-apple-darwin
#
# Idempotent. Cached cargo builds make subsequent runs fast.

set -euo pipefail

cd "$(dirname "$0")/.."          # → apps/macos
REPO_ROOT="$(cd ../.. && pwd)"
BUILD_DIR="$PWD/build/rust"
GEN_DIR="$PWD/CascadeMac/Generated"

mkdir -p "$BUILD_DIR"

echo "→ cargo build --target aarch64-apple-darwin"
( cd "$REPO_ROOT" && cargo build --release \
    --target aarch64-apple-darwin \
    -p cascade-uniffi )

echo "→ cargo build --target x86_64-apple-darwin"
( cd "$REPO_ROOT" && cargo build --release \
    --target x86_64-apple-darwin \
    -p cascade-uniffi )

echo "→ lipo (universal static lib)"
lipo -create \
    "$REPO_ROOT/target/aarch64-apple-darwin/release/libcascade_uniffi.a" \
    "$REPO_ROOT/target/x86_64-apple-darwin/release/libcascade_uniffi.a" \
    -output "$BUILD_DIR/libcascade_uniffi.a"

echo "→ uniffi-bindgen generate (Swift)"
( cd "$REPO_ROOT" && cargo run -p cascade-uniffi --bin uniffi-bindgen --release -- generate \
    --library "$REPO_ROOT/target/aarch64-apple-darwin/release/libcascade_uniffi.dylib" \
    --language swift \
    --out-dir "$GEN_DIR" )

# Some uniffi versions emit a generated `cascade_uniffiFFI.modulemap` that
# Xcode picks up automatically because the same directory is on the Swift
# include path. Sanity-check it's there.
test -f "$GEN_DIR/cascade_uniffi.swift"     || { echo "missing Swift binding"; exit 1; }
test -f "$GEN_DIR/cascade_uniffiFFI.h"      || { echo "missing C header"; exit 1; }
test -f "$GEN_DIR/cascade_uniffiFFI.modulemap" || { echo "missing modulemap"; exit 1; }

echo "✓ Done. Static lib: $BUILD_DIR/libcascade_uniffi.a"
