# Getting Cascade notarized (Developer ID) — runbook

Goal: replace the ad-hoc/un-notarized macOS release with a **Developer ID-signed +
notarized** build so downloaders get no Gatekeeper warning. Split into **(A) web
steps for a browser agent** and **(B) local CLI steps** on this Mac.

Apple ID to use: **jacob.c.stephens@me.com** (already signed into Xcode; current
free dev team `G38J85UN6P`). Have a payment method ready ($99/yr).

---

## A. Browser-agent steps (web UI)

> Sign in with the Apple ID above. 2FA codes will go to the user's trusted
> devices — pause and ask the user to read them out when prompted.

### A1. Enroll in the Apple Developer Program ($99/yr)
1. Go to <https://developer.apple.com/programs/enroll/>.
2. Sign in with the Apple ID. Complete 2FA.
3. Choose **Individual / Sole Proprietor** entity type (simplest; the legal name
   becomes the seller name).
4. Review the agreement, accept, and pay the **$99 USD** annual fee.
5. Enrollment is usually approved in minutes to ~48 h. You'll get an email.
   Until "Welcome to the Apple Developer Program" arrives, B-step signing with a
   Developer ID cert will not work.

### A2. (For notarization auth) Create an App Store Connect API key
> Preferred over app-specific passwords for automation.
1. Go to <https://appstoreconnect.apple.com/access/integrations/api>
   (App Store Connect → **Users and Access → Integrations → App Store Connect API**).
2. Under **Team Keys**, click **Generate API Key** (or **+**).
3. Name it `cascade-notary`, set **Access: Developer**, click **Generate**.
4. **Download the `.p8` key file** (downloadable only once) and note the
   **Key ID** and the **Issuer ID** (shown on that page). Save all three —
   the user/agent must hand the `.p8` path, Key ID, and Issuer ID to step B2.

### A3. (Alternative to A2) App-specific password
If API keys are unavailable, instead:
1. Go to <https://account.apple.com/account/manage> → **Sign-In and Security →
   App-Specific Passwords → +**.
2. Name it `cascade-notary`, generate, and copy the `xxxx-xxxx-xxxx-xxxx` value.

### A4. (Optional) Developer ID cert via web
Easiest path is Xcode (step B1). If doing it on the web instead:
1. <https://developer.apple.com/account/resources/certificates/list> → **+**.
2. Choose **Developer ID Application** → Continue.
3. Upload a CSR (the user generates it via Keychain Access → Certificate
   Assistant → Request a Certificate from a CA → "Saved to disk"), download the
   `.cer`, and double-click to install into the login keychain.

---

## B. Local CLI steps (run on this Mac, after A is done)

Paths assume the repo at `~/cascade`. Ensure tools are on PATH:
`export PATH="/opt/homebrew/opt/rustup/bin:/opt/homebrew/bin:$PATH"`

### B1. Create the Developer ID Application certificate (via Xcode — easiest)
Once the program membership is active:
- Xcode → **Settings → Accounts** → select the Apple ID → **Manage Certificates…**
  → **+** → **Developer ID Application**. It lands in the login keychain.
- Verify: `security find-identity -v -p codesigning | grep "Developer ID Application"`
  → note the identity name, e.g. `Developer ID Application: Jacob Stephens (XXXXXXXXXX)`.

### B2. Store notarization credentials in a keychain profile
Using the API key from A2:
```
xcrun notarytool store-credentials cascade-notary \
  --key /path/to/AuthKey_<KEYID>.p8 \
  --key-id <KEYID> \
  --issuer <ISSUER-UUID>
```
(or with A3: `xcrun notarytool store-credentials cascade-notary \
  --apple-id jacob.c.stephens@me.com --team-id <TEAMID> --password <app-specific-pw>`)

### B3. Build, Developer ID-sign, notarize, staple, package
The project already sets `ENABLE_HARDENED_RUNTIME=YES` for the Mac target, which
notarization requires. Replace `DEVTEAM` with the **paid** team ID (it differs
from the current free `G38J85UN6P`).
```
cd ~/cascade/apps/apple
xcodegen generate

# Build signed with Developer ID (hardened runtime on, real team)
xcodebuild -project Cascade.xcodeproj -scheme CascadeMac -configuration Release \
  -destination 'platform=macOS' -derivedDataPath build/dd-mac-rel \
  ARCHS="arm64 x86_64" ONLY_ACTIVE_ARCH=NO \
  DEVELOPMENT_TEAM=DEVTEAM CODE_SIGN_STYLE=Manual \
  CODE_SIGN_IDENTITY="Developer ID Application" \
  OTHER_CODE_SIGN_FLAGS="--timestamp --options runtime" \
  build

APP=build/dd-mac-rel/Build/Products/Release/Cascade.app
codesign --verify --deep --strict --verbose=2 "$APP"

# Package a DMG
STAGING=$(mktemp -d)/Cascade; mkdir -p "$STAGING"
cp -R "$APP" "$STAGING/"; ln -s /Applications "$STAGING/Applications"
hdiutil create -volname "Cascade 0.1.0" -srcfolder "$STAGING" -ov -format UDZO \
  ~/Desktop/Cascade-0.1.0.dmg

# Notarize the DMG and staple the ticket
xcrun notarytool submit ~/Desktop/Cascade-0.1.0.dmg \
  --keychain-profile cascade-notary --wait
xcrun stapler staple ~/Desktop/Cascade-0.1.0.dmg
spctl -a -t open --context context:primary-signature -vv ~/Desktop/Cascade-0.1.0.dmg  # expect "accepted"
```

### B4. Replace the GitHub release asset
```
cd ~/cascade
gh release delete-asset v0.1.0 Cascade-0.1.0.dmg -y
gh release upload v0.1.0 ~/Desktop/Cascade-0.1.0.dmg
gh release edit v0.1.0 --notes "Cascade 0.1.0 (macOS) — Developer ID signed & notarized. Universal, macOS 14+."
```
After this, the download opens with no Gatekeeper bypass needed.

---

## Notes
- The **free** team (`G38J85UN6P`) cannot create Developer ID certs or notarize —
  those require the **paid** Developer Program (step A1).
- iOS distribution (TestFlight/App Store) is a separate flow; this runbook is
  macOS Developer ID only.
- Keep the `.p8` key and app-specific password secret; do not commit them.
