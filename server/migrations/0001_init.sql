-- Cascade listening-time sync schema.
--
-- The whole privacy posture rests on what is NOT here: there is no session log,
-- no per-listen timestamps, no event stream. The only listening data stored is
-- one grow-only integer per (user, device) — a G-Counter slot. The server can
-- therefore report how much a user has listened in total, but can never
-- reconstruct when.

-- A user identity. Email is the only PII; there is no password (magic-link).
CREATE TABLE IF NOT EXISTS users (
    id         UUID PRIMARY KEY,
    email      TEXT NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- One-time magic-link tokens. We store only the SHA-256 hash of the token, so a
-- database leak does not hand out working sign-in links. Short-lived and
-- single-use.
CREATE TABLE IF NOT EXISTS login_tokens (
    token_hash BYTEA PRIMARY KEY,
    email      TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    used       BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS login_tokens_expires_idx ON login_tokens (expires_at);

-- Opaque server-side session tokens (again stored only as a hash). Opaque, not
-- JWT, so logout / delete-account / revoke-all is a single DELETE — no waiting
-- for a signed token to expire.
CREATE TABLE IF NOT EXISTS sessions (
    token_hash BYTEA PRIMARY KEY,
    user_id    UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL
);
CREATE INDEX IF NOT EXISTS sessions_user_idx ON sessions (user_id);

-- The G-Counter: one grow-only millisecond slot per device. Merge is
-- GREATEST(existing, incoming) on write; a user's lifetime total is SUM across
-- their devices on read. device_id is opaque and client-generated; rotating it
-- (on "delete my data") simply means the next write lands in a fresh slot.
CREATE TABLE IF NOT EXISTS device_counters (
    user_id    UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    device_id  TEXT NOT NULL,
    total_ms   BIGINT NOT NULL CHECK (total_ms >= 0),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, device_id)
);
CREATE INDEX IF NOT EXISTS device_counters_user_idx ON device_counters (user_id);
