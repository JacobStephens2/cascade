#!/usr/bin/env bash
# Build, sign, and install CascadeiOS onto a connected iPhone, then launch it.
#
# Prerequisites (one-time):
#   1. iPhone connected + paired (tap "Trust This Computer" on the phone).
#   2. Developer Mode ON: iPhone Settings > Privacy & Security > Developer Mode.
#   3. An Apple ID added to Xcode (Settings > Accounts) so a development
#      signing certificate + team exist. A free Apple ID works (7-day apps).
#
# Usage (from apps/apple/):
#   ./scripts/install-device.sh [TEAM_ID] [DEVICE_UDID]
# Both args are auto-detected if omitted.
set -euo pipefail
cd "$(dirname "$0")/.."   # -> apps/apple

# --- resolve the device ---------------------------------------------------
UDID="${2:-$(xcrun devicectl list devices 2>/dev/null | awk '/available/{print $3; exit}')}"
[ -n "${UDID:-}" ] || { echo "No connected/paired device found. Plug in + Trust the phone."; exit 1; }
echo "→ device: $UDID"

# --- resolve the signing team ---------------------------------------------
TEAM="${1:-$(security find-certificate -a -c 'Apple Development' -p 2>/dev/null \
  | openssl x509 -noout -subject 2>/dev/null \
  | grep -oE 'OU *= *[A-Z0-9]{10}' | grep -oE '[A-Z0-9]{10}' | head -1)}"
[ -n "${TEAM:-}" ] || { echo "No Apple Development team found. Add your Apple ID in Xcode > Settings > Accounts, then pass the 10-char Team ID as arg 1."; exit 1; }
echo "→ team:   $TEAM"

# --- build + sign for the device ------------------------------------------
echo "→ building (automatic signing)…"
xcodebuild -project Cascade.xcodeproj -scheme CascadeiOS -configuration Debug \
  -destination "id=$UDID" -derivedDataPath build/dd-iosdev \
  DEVELOPMENT_TEAM="$TEAM" CODE_SIGN_STYLE=Automatic \
  -allowProvisioningUpdates -allowProvisioningDeviceRegistration build

APP="build/dd-iosdev/Build/Products/Debug-iphoneos/Cascade.app"
[ -d "$APP" ] || { echo "Build did not produce $APP"; exit 1; }

# --- install + launch ------------------------------------------------------
echo "→ installing onto device…"
xcrun devicectl device install app --device "$UDID" "$APP"
echo "→ launching…"
xcrun devicectl device process launch --device "$UDID" page.stephens.cascade || true
echo "✓ Done. If the phone shows 'Untrusted Developer', go to Settings > General"
echo "  > VPN & Device Management, trust your cert, then re-run this script."
