#!/usr/bin/env bash
# Build + publish the web app. Run from the repo root.
#
#   ./deploy/deploy.sh
#
# Idempotent. Assumes the Apache vhost is already installed (see
# deploy/cascade.stephens.page.apache.conf).

set -euo pipefail

cd "$(dirname "$0")/.."

echo "→ wasm-pack build"
wasm-pack build crates/cascade-wasm --target web \
    --out-dir ../../apps/web/src/wasm --release

echo "→ vite build"
cd apps/web
npm install --silent
npm run build

echo "→ done. Apache already serves apps/web/dist."
echo "   Visit: https://cascade.stephens.page/"
