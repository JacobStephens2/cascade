#!/usr/bin/env bash
# Build + publish the web app. Run from the repo root on the droplet:
#
#   ./deploy/deploy.sh
#
# Automatic deploys happen from GitHub Actions on every push to main
# (`.github/workflows/deploy-web.yml`): CI builds, then rsyncs `apps/web/dist`
# here. This script is the on-box equivalent (git pull, then this).
#
# Idempotent. Assumes the Apache vhost is already installed (see
# deploy/cascade.stephens.page.apache.conf).

set -euo pipefail

# rustup installs to ~/.cargo/bin, which is not on the non-login PATH.
export PATH="${HOME}/.cargo/bin:${PATH}"

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
