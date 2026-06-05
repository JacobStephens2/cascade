# cascade-sync-server

The optional backend for Cascade's listening-time feature: magic-link sign-in
and cross-device aggregation of how long you've listened. It is a standalone
Rust/Axum service (its own cargo workspace) backed by Postgres.

It is deliberately tiny in what it stores: an **email address** and **one
grow-only integer per device**. There is no session log and no per-listen
timestamps, so the data can answer "how much have I listened?" but never "when
did I listen?".

## Data model

Four tables (`migrations/0001_init.sql`):

- `users` — id + email (the only PII; no password).
- `login_tokens` — single-use, short-lived magic-link tokens (stored hashed).
- `sessions` — opaque server-side session tokens (stored hashed). Opaque, not
  JWT, so logout / delete-account revokes instantly with one `DELETE`.
- `device_counters` — the **G-Counter**: one `total_ms` per `(user, device)`.
  Merge is `GREATEST(existing, incoming)` on write; a user's lifetime total is
  `SUM(total_ms)` on read. Concurrent listening on two devices adds, never
  overwrites.

## API

All bodies are JSON; camelCase fields.

| Method | Path             | Auth | Body                              | Returns |
|--------|------------------|------|-----------------------------------|---------|
| GET    | `/health`        | —    | —                                 | `{ok}`  |
| POST   | `/auth/request`  | —    | `{email}`                         | `{ok}` (always — never leaks who has an account) |
| POST   | `/auth/verify`   | —    | `{token}`                         | `{sessionToken, email}` |
| POST   | `/auth/logout`   | Bearer | —                               | `{ok}`  |
| PUT    | `/listening`     | Bearer | `{deviceId, deviceTotalMs}`     | `{serverTotalMs, syncedThroughMs}` |
| GET    | `/listening`     | Bearer | —                               | `{serverTotalMs}` |
| DELETE | `/listening`     | Bearer | —                               | `{ok}` (clears counters; client rotates `deviceId`) |
| DELETE | `/account`       | Bearer | —                               | `{ok}` (cascades to sessions + counters) |

Auth is `Authorization: Bearer <sessionToken>`.

## Run locally

```bash
cp .env.example .env        # set DATABASE_URL; leave SMTP_HOST empty to log links
createdb cascade_sync       # or use the existing Postgres-on-volume instance
cargo run                   # migrations run automatically on boot
```

With `SMTP_HOST` empty the magic link is written to the log instead of emailed,
so you can complete the flow without a mail server.

## Deploy (to do — coordinated with the feature release)

This is intentionally **not deployed yet**; it must go live in the same release
as the shells and the privacy-policy update. When we cut that release:

1. **Postgres** — create a `cascade_sync` database + role on the existing
   Postgres-on-volume instance.
2. **Subdomain** — e.g. `sync.cascade.stephens.page`, Apache vhost reverse-proxying
   to `BIND_ADDR`, plus certbot for TLS (see the new-subdomain pattern in the
   infra notes).
3. **systemd** — a `cascade-sync` unit running the release binary with the env
   file; `EnvironmentFile=` for `.env`.
4. **SMTP** — real `SMTP_*` creds so magic links actually send.
5. Point the shells' `VITE_SYNC_API` (web) / equivalent at the subdomain.
