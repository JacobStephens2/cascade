# Threat model — cascade-sync-server

The sync service exists to do one small thing — let a user see their total
Cascade listening time across devices — and the design goal is that doing it
should *create as little risk as possible*. This document states what it
protects, against whom, and where the residual risk is.

## What it stores (assets)

| Asset | Sensitivity | Why it's small |
|---|---|---|
| Email address | PII (low) | The only personal data. Used solely to sign in. |
| Magic-link tokens | Secret (short-lived) | Stored **hashed** (SHA-256); single-use; 15-min TTL. |
| Session tokens | Secret | Stored **hashed**; opaque (not JWT), so revocable instantly. |
| Listening counters | Low | One integer per `(user, device)`. **No timestamps, no session log** — a breach reveals *how much*, never *when*. |

The single most important control is **data minimization by construction**: the
schema cannot express a listening timeline, so no amount of access reconstructs
one. This bounds the blast radius of every other threat below.

## Trust boundaries

```
[ browser / native app ]
        | HTTPS (TLS, certbot)            ← public internet boundary
[ Apache reverse proxy ]  -- blocks /metrics
        | localhost:3470
[ cascade-sync-server (hardened systemd unit) ]
        | localhost:5432                  ← DB boundary
[ Postgres (cascade_sync role, least priv) ]

[ cascade-sync-server ] --STARTTLS--> [ Resend SMTP ]   ← email channel
```

## Threats and mitigations

| # | Threat | Mitigation |
|---|---|---|
| T1 | **Account enumeration** via the sign-in endpoint | `POST /auth/request` always returns 200 — it never reveals whether an address has an account. |
| T2 | **DB leak yields working tokens** | Magic-link and session tokens are stored only as SHA-256 hashes; the plaintext exists only in the email / the client. |
| T3 | **Magic-link replay / interception** | 256-bit random token, single-use (atomic `UPDATE ... WHERE used=false RETURNING`), 15-min expiry, HTTPS-only delivery. |
| T4 | **Session theft → can't revoke** | Opaque server-side sessions; logout / delete-account is a single `DELETE`, taking effect immediately (no wait for a signed token to expire). |
| T5 | **CSRF** | API authenticates with a `Bearer` header, not cookies, so cross-site requests can't ride ambient credentials. |
| T6 | **SQL injection** | All queries are parameterized (`sqlx` bind); no string interpolation into SQL. |
| T7 | **Cross-origin abuse** | CORS allow-list is exactly the web origin; other origins get no `Access-Control-Allow-Origin`. |
| T8 | **Counter resurrection after deletion** | "Delete my data" rotates the client `device_id`, so a stale offline write lands in a fresh slot instead of restoring a deleted total. |
| T9 | **Metrics disclosure** | `/metrics` is localhost-bound and blocked at Apache; it carries only operational counters — no PII, no per-user data. |
| T10 | **Host compromise via the service** | systemd hardening: unprivileged user, `ProtectSystem=strict`, empty `CapabilityBoundingSet`, `MemoryDenyWriteExecute`, restricted address families, `PrivateTmp`. The service writes nothing to disk. |
| T11 | **Secret exposure** | DB password + SMTP creds live in a `0640` env file owned by the service user (vault-encrypted in the repo); the DO token is env-only. Nothing secret is committed. |
| T12 | **Counter inflation** | A user can only inflate *their own* total (the G-Counter slot is per-device, summed per-user) — it never affects another user. The core also clamps per-tick deltas client-side. Accepted: a vanity number has no integrity value worth defending server-side. |

## Residual risks / known gaps

- **No rate limiting (R1).** `POST /auth/request` will email a link to any
  address on every call, so it can be abused to send unsolicited mail (bounded
  by Resend's own quotas/abuse controls) or to brute-force nothing useful (tokens
  are 256-bit). *Recommended next step:* per-IP + per-email rate limiting on the
  auth endpoints.
- **Web tokens live in `localStorage` (R2),** so a successful XSS on
  cascade.stephens.page could exfiltrate a session. Mitigated by the app shipping
  no third-party scripts; a strict CSP would harden it further.
- **No audit log (R3).** Intentional — it's the flip side of data minimization.
  Acceptable for a listening-time toy; revisit if scope grows.

## Out of scope

DDoS absorption (handled at the proxy/network layer), Resend's own security,
and physical/host security of the droplet beyond the service's own sandboxing.
