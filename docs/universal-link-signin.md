# Universal Link sign-in (open the magic link in the app)

The account magic-link email contains
`https://cascade.stephens.page/auth?token=…`. By default that URL opens the
**website**. With Universal Links configured it instead opens the **Cascade
app** directly into `AppStore.completeSignIn(…)`, so the user never copies a
token by hand.

## Pieces (all in this repo)

| Piece | Where | Status |
|-------|-------|--------|
| `onOpenURL` → `handleOpenURL` → `completeSignIn` | `CascadeShared/App/AppStore.swift`, `Cascade*App.swift` | already wired |
| `associated-domains` entitlement (`applinks:cascade.stephens.page`) | `apps/apple/project.yml` (both targets, via `entitlements.properties` so XcodeGen doesn't clobber it) | added |
| AASA file | `apps/web/public/.well-known/apple-app-site-association` | added |
| Browser fallback for un-installed devices | `apps/web/public/auth/index.html` (shows the token to paste) | added |

The app already extracts the token from either a full link or a bare token, so
no Swift changes are needed — only the OS-level link registration above.

## Requirement: paid Apple Developer Program

The `associated-domains` capability is **not available under free
personal-team signing**. A *signed* build with this entitlement fails with:

```
"CascadeMac" has entitlements that require signing with a development certificate.
```

So to ship this you must:

1. Enrol in the paid Apple Developer Program.
2. Set `DEVELOPMENT_TEAM` in `project.yml` to your Team ID (`LHY8W725A8`, the
   paid program team) and build with development/distribution signing (not
   ad-hoc `-`). The AASA `appIDs` entry must match: `LHY8W725A8.page.stephens.cascade`.
3. Enable the **Associated Domains** capability for the App ID in the developer
   portal.

> CI is unaffected: `apple.yml` compiles with `CODE_SIGNING_ALLOWED=NO`, which
> skips entitlement validation. Only signed artifacts (the ad-hoc DMG and the
> free `install-device.sh` flow) require the paid cert — those will need the
> paid team once this is merged.

## Hosting the AASA file

Apple fetches `https://cascade.stephens.page/.well-known/apple-app-site-association`
(no redirects, `Content-Type: application/json`, **no** `.json` extension). The
file is plain static content under `apps/web/public/`, but verify after deploy:

```bash
curl -sI https://cascade.stephens.page/.well-known/apple-app-site-association
# 200, content-type application/json, served directly (no 30x)
```

Two gotchas:

- The web build must copy the dotfolder. Confirm `.well-known/` lands in the
  built `dist/` and the web server (Apache vhost) serves dotfolders and
  extensionless files as JSON.
- Apple caches the AASA via its CDN
  (`https://app-site-association.cdn-apple.com/a/v1/cascade.stephens.page`);
  changes can take time to propagate. On a dev device, reinstalling the app
  re-fetches it.

## Verify end-to-end (signed build, real device)

1. Install a properly-signed build on a device/Mac with the app.
2. Request a sign-in link from the app, open Mail, tap the link.
3. The app should foreground and land signed-in (no paste). If it opens the
   website instead, the AASA isn't being served correctly or the entitlement
   isn't in the signed build.

## Security note

The token rides in the URL, so keep it **single-use + short-lived** (the server
already expires it after 15 minutes / one use). Universal Links are safer than a
custom `cascade://` scheme because Apple verifies domain ownership via the AASA
file, so no other app can claim `cascade.stephens.page/auth`.
