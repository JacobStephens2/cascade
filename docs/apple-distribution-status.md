# Cascade — Apple distribution status (2026-06-16)

Snapshot of the Apple Developer Program rollout: Universal-Link sign-in,
TestFlight, and the notarized macOS DMG. Universal-Link (macOS) and the
notarized DMG are **done**; iOS TestFlight is uploaded with external beta
review pending.

## Key facts / IDs

| Thing | Value |
|---|---|
| Paid team | **Stephens Page LLC** — Team ID `LHY8W725A8` (replaced the old free `G38J85UN6P`) |
| App record | **Cascade: White Noise & Focus** — Apple ID `6780721749`, bundle `page.stephens.cascade`, SKU `cascade-ios` |
| Bundle ID (portal) | `page.stephens.cascade` (UNIVERSAL), resource id `2PGZD32B5C`; watch `page.stephens.cascade.watchkitapp` |
| ASC API key | Key `JLFPG25C4J`, Issuer `67ee426c-dbe6-45a4-86e1-dc102fb781d1`, `~/.appstoreconnect/private_keys/AuthKey_JLFPG25C4J.p8` (App Manager role). GET helper `/tmp/asc.py` |
| Apple Distribution cert | auto-created on export; signs the `.ipa` |
| Developer ID cert | `Developer ID Application: Stephens Page LLC (LHY8W725A8)` — ASC id `HFF3XV4VG6`, keychain SHA `0009D8B6583D6939DE0E61B92587BB14C9D01E46` |
| Dev ID macOS profile | `MAC_APP_DIRECT` "Cascade macOS Developer ID" — ASC id `445ABBAXN6`, saved to `/tmp/cascade-devid.provisionprofile` (App ID `LHY8W725A8.page.stephens.cascade`, `associated-domains = *`) |
| Notary keychain profile | `cascade-notary` (validated, stored) |

## Workstream 1 — Universal-Link sign-in (macOS) ✅ DONE

Verified working 2026-06-15. The magic link `https://cascade.stephens.page/auth?token=…`
opens the Cascade macOS app (`swcutil openul` → SUCCESS, swcd association
`approved`). Details + gotchas in [universal-link-signin.md](universal-link-signin.md)
and the `cascade-apple-build` memory. Shipped:

- `project.yml` `DEVELOPMENT_TEAM: LHY8W725A8`; AASA `appIDs` → `LHY8W725A8.page.stephens.cascade`.
- AASA now served as `Content-Type: application/json` (was untyped — latent bug).
- Commits on `main`: `ebe972b` (team/AASA), `ead576a` (AASA content-type). Live + deployed.

## Workstream 2 — iOS/watch + TestFlight ✅ build uploaded, ⏳ beta setup

- **Archive + export + upload: DONE.** `Cascade.ipa` (universal watch embedded,
  `associated-domains` present) signed `Apple Distribution: Stephens Page LLC (LHY8W725A8)`,
  uploaded via `altool` + ASC key. **TestFlight build 1 = VALID** (Delivery UUID
  `166b0344-4181-4aff-8669-61f9885910f4`).
- Two real project bugs fixed to make `archive` work (see "Uncommitted" below):
  1. Watch shared `PRODUCT_NAME: Cascade` with the iOS app → archive collision
     ("Multiple commands produce Cascade.swiftmodule / .app.dSYM"). Fix: watch
     `PRODUCT_NAME: CascadeWatch` (display name stays "Cascade").
  2. `-sdk iphoneos` forced the watch to build for iphoneos → "AppIcon did not
     have applicable content". Fix: archive with `-destination 'generic/platform=iOS'`,
     no `-sdk`.
- **TestFlight beta: DONE (2026-06-16).**
  - Export compliance cleared via API (`usesNonExemptEncryption=false`) → build is
    `READY_FOR_BETA_TESTING`. (TODO: add `ITSAppUsesNonExemptEncryption: false` to
    the iOS Info.plist in `project.yml` so future builds don't re-ask.)
  - **Internal group** "Internal" (`b74fe13c-df88-42ae-a7ca-cac3f26bcb40`), build
    attached, Jacob (`jacob@stephens.page`, Account Holder) added as tester →
    installable via TestFlight now.
  - **External group** "Friends & Family" (`08f85c72-5e0d-4ecf-ac3e-69f0c3ff75de`),
    **public link https://testflight.apple.com/join/ryhNMteg**, build attached,
    beta app description + "what to test" + review contact set. Submitted for Beta
    App Review → **WAITING_FOR_REVIEW** (~1 day; public link goes live once approved).
  - **Remaining: verify iOS/watch Universal Link** — install the TestFlight build on
    the iPhone, tap the magic link **in Mail.app** (native — a browser/Gmail-web
    tap never triggers Universal Links), confirm Cascade opens signed-in.
  - iPhone dev-install via `install-device.sh` is blocked on Xcode pairing
    ("device unpaired"); the TestFlight path sidesteps it.

## Workstream 3 — Notarized macOS DMG ✅ DONE (2026-06-16)

Goal: notarized Developer ID DMG with the account/Universal-Link feature →
publish **GitHub release v0.2.0**. **SHIPPED:**
**https://github.com/JacobStephens2/cascade/releases/tag/v0.2.0**
(`Cascade-0.2.0.dmg`, 15.8 MB, universal, macOS 14+, tag on `main@70ee859`).

What it took:
- Version bumped to **0.2.0** (matches Android account-feature line); committed
  on main in `70ee859` (also the watch `PRODUCT_NAME: CascadeWatch` archive fix).
- Universal (arm64+x86_64) Release archive: `build/Cascade-mac.xcarchive`.
- Developer ID cert created (Xcode), notary creds stored as keychain profile
  `cascade-notary`.
- The automatic `developer-id` export **hangs** on Apple's provisioning service,
  so we re-signed the archived app manually with the Developer ID cert + the
  API-created `MAC_APP_DIRECT` profile (keeps Universal Links). Staged app:
  `build/mac-devid/Cascade.app` (profile already embedded).

### The keychain blocker — RESOLVED
`codesign` with the Developer ID key was failing `errSecInternalComponent` (the
key's partition list / ACL didn't grant `codesign`). **Cleared this session** —
the first `codesign` after the GUI "Always Allow" approval succeeded and the ACL
now **persists** (subsequent re-signs run with no prompt). If it ever regresses:
Keychain Access → login → My Certificates → *Developer ID Application: Stephens
Page LLC* → private key → Access Control → "Allow all applications", or
`security set-key-partition-list -S apple-tool:,apple:,codesign: -k <pw> -s ~/Library/Keychains/login.keychain-db`.

### Gotchas hit while finishing
- **`/tmp/cascade-ent.plist` was wiped** (reboot clears `/tmp`). Regenerate from
  the archive app: `codesign -d --entitlements - --xml <archive>/.../Cascade.app > /tmp/cascade-ent.plist`
  (yields app-sandbox, network.client, associated-domains, application-identifier,
  team-identifier, **no get-task-allow** — correct).
- **Don't run two `codesign` passes concurrently on one bundle** → leaves a
  `Cascade.cstemp` and breaks the seal ("a sealed resource is missing or invalid").
  Wait for codesign to be idle, `find … -name '*.cstemp' -delete`, then one clean sign.
- **`spctl -a -t open` on the DMG reports "no usable signature"** — that's
  EXPECTED (the DMG container isn't codesigned). The authoritative check is the
  **app inside** the mounted DMG: `spctl -a -t exec -vv Cascade.app` →
  `accepted / source=Notarized Developer ID`. Stapling the DMG (not the inner
  app) is the correct DMG-distribution approach.
- **`notarytool --wait` can die on a transient network timeout** (NSURLError
  "request timed out") while Apple is still `In Progress` — NOT a rejection.
  Just `xcrun notarytool wait <id> --keychain-profile cascade-notary` to resume.
  This run sat `In Progress` ~30 min (Apple queue slow that day) before `Accepted`.
- **`gh release create --target`** rejects a short SHA (`target_commitish is
  invalid`) — pass a branch name (`--target main`) or full SHA.

### Recorded values
- Notary submission `6dee30d3-047d-4e29-a2ae-bd4fcf554ff6` → **Accepted**.
- DMG stapled + validated; app inside = `Notarized Developer ID`.

### Resume recipe (for the next version bump)
```bash
cd ~/cascade/apps/apple
APP=build/mac-devid/Cascade.app
codesign -d --entitlements - --xml build/Cascade-mac.xcarchive/Products/Applications/Cascade.app > /tmp/cascade-ent.plist
codesign --force --options runtime --timestamp \
  --sign "Developer ID Application: Stephens Page LLC (LHY8W725A8)" \
  --entitlements /tmp/cascade-ent.plist "$APP"
codesign --verify --strict --verbose=2 "$APP"     # expect valid on disk
STAGING=$(mktemp -d)/Cascade; mkdir -p "$STAGING"
cp -R "$APP" "$STAGING/"; ln -s /Applications "$STAGING/Applications"
hdiutil create -volname "Cascade <ver>" -srcfolder "$STAGING" -ov -format UDZO ~/Desktop/Cascade-<ver>.dmg
xcrun notarytool submit ~/Desktop/Cascade-<ver>.dmg --keychain-profile cascade-notary --wait
xcrun stapler staple ~/Desktop/Cascade-<ver>.dmg
# verify: mount, then  spctl -a -t exec -vv <mount>/Cascade.app  → accepted/Notarized Developer ID
cd ~/cascade
gh release create v<ver> ~/Desktop/Cascade-<ver>.dmg --target main \
  --title "Cascade <ver> (macOS)" --notes "Developer ID signed & notarized. …"
```

## Uncommitted local changes (commit WITHOUT a Claude co-author trailer)

`apps/apple/project.yml`:
- watch `PRODUCT_NAME: CascadeWatch` (archive-collision fix)
- `MARKETING_VERSION: "0.2.0"`

Also untracked: `docs/apple-developer-id-setup.md`, `docs/apple-distribution-status.md`
(this file), `santa-lockdown-restore.mobileconfig`. Regenerated UniFFI bindings
under `CascadeShared/Generated/` are intentionally left out of commits.

## Open task board
- [x] Universal Links (macOS) verified
- [x] App record created (`6780721749`)
- [x] iOS archive + Apple Distribution + TestFlight upload (build VALID)
- [ ] TestFlight beta testing setup
- [ ] iOS/watch Universal Link verify (via TestFlight, tap in Mail)
- [x] Notarized DMG → keychain ACL cleared → notarized → stapled → GitHub v0.2.0 published
- [x] Commit project.yml fixes (no co-author trailer) — landed in `70ee859`
