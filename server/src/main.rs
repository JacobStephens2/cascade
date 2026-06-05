//! Cascade listening-time sync service.
//!
//! A small Axum + Postgres service that does exactly two things: sign a user in
//! by email (magic-link, opaque server-side sessions) and aggregate their
//! listening time across devices as a G-Counter (`GREATEST` on write, `SUM` on
//! read). It stores an email address and one integer per device — nothing else,
//! and in particular no record of *when* anyone listened.

mod error;
mod mail;

use std::net::SocketAddr;
use std::time::Duration as StdDuration;

use axum::extract::State;
use axum::http::{HeaderMap, Method};
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::{Duration, Utc};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::mail::Mailer;

const LOGIN_TOKEN_TTL_MINUTES: i64 = 15;
const SESSION_TTL_DAYS: i64 = 90;
const MAX_DEVICE_ID_LEN: usize = 128;

#[derive(Clone)]
struct AppState {
    pool: PgPool,
    mailer: Mailer,
    /// Where the magic link points (the web app), e.g. `https://cascade.stephens.page`.
    frontend_origin: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,sqlx=warn".into()),
        )
        .init();

    let database_url = env("DATABASE_URL")?;
    let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:3470".to_string());
    let frontend_origin =
        std::env::var("FRONTEND_ORIGIN").unwrap_or_else(|_| "https://cascade.stephens.page".into());

    let mailer = Mailer::from_settings(
        &std::env::var("SMTP_HOST").unwrap_or_default(),
        std::env::var("SMTP_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(587),
        &std::env::var("SMTP_USERNAME").unwrap_or_default(),
        &std::env::var("SMTP_PASSWORD").unwrap_or_default(),
        &std::env::var("SMTP_FROM").unwrap_or_else(|_| "Cascade <noreply@stephens.page>".into()),
    )?;

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(StdDuration::from_secs(5))
        .connect(&database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    let cors = build_cors(&frontend_origin);
    let state = AppState {
        pool,
        mailer,
        frontend_origin,
    };

    let app = Router::new()
        .route("/health", get(health))
        .route("/auth/request", post(auth_request))
        .route("/auth/verify", post(auth_verify))
        .route("/auth/logout", post(auth_logout))
        .route("/listening", put(listening_put))
        .route("/listening", get(listening_get))
        .route("/listening", delete(listening_delete))
        .route("/account", delete(account_delete))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr: SocketAddr = bind_addr.parse()?;
    tracing::info!(%addr, "cascade-sync-server listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

fn env(key: &str) -> anyhow::Result<String> {
    std::env::var(key).map_err(|_| anyhow::anyhow!("missing required env var {key}"))
}

fn build_cors(frontend_origin: &str) -> CorsLayer {
    // Allow the web origin(s) to call the API cross-subdomain. Comma-separated
    // CORS_ALLOWED_ORIGINS overrides; otherwise just the frontend origin.
    let origins: Vec<_> = std::env::var("CORS_ALLOWED_ORIGINS")
        .unwrap_or_else(|_| frontend_origin.to_string())
        .split(',')
        .filter_map(|o| o.trim().parse().ok())
        .collect();
    CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([
            axum::http::header::AUTHORIZATION,
            axum::http::header::CONTENT_TYPE,
        ])
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("shutting down");
}

// ---- token helpers -------------------------------------------------------

/// A fresh 256-bit URL-safe random token (the plaintext we hand the client).
fn generate_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

/// SHA-256 of a token — what we actually store, so a DB leak yields no usable
/// tokens.
fn hash_token(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}

fn normalize_email(raw: &str) -> Option<String> {
    let e = raw.trim().to_lowercase();
    // Deliberately minimal: one '@' with something on each side. Real delivery
    // is the actual validity check.
    let parts: Vec<&str> = e.split('@').collect();
    if parts.len() == 2 && !parts[0].is_empty() && parts[1].contains('.') {
        Some(e)
    } else {
        None
    }
}

/// Resolve the bearer session token to a user id, or `Unauthorized`.
async fn authenticate(pool: &PgPool, headers: &HeaderMap) -> AppResult<Uuid> {
    let token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
        .ok_or(AppError::Unauthorized)?;
    let hash = hash_token(token);
    let user_id: Option<Uuid> =
        sqlx::query_scalar("SELECT user_id FROM sessions WHERE token_hash = $1 AND expires_at > now()")
            .bind(hash)
            .fetch_optional(pool)
            .await?;
    user_id.ok_or(AppError::Unauthorized)
}

async fn aggregate_total_ms(pool: &PgPool, user_id: Uuid) -> AppResult<i64> {
    let total: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(total_ms), 0)::BIGINT FROM device_counters WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    Ok(total)
}

// ---- request / response shapes ------------------------------------------

#[derive(Deserialize)]
struct AuthRequest {
    email: String,
}

#[derive(Deserialize)]
struct VerifyRequest {
    token: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct VerifyResponse {
    session_token: String,
    email: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListeningPut {
    device_id: String,
    device_total_ms: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ListeningResponse {
    server_total_ms: i64,
    synced_through_ms: i64,
}

#[derive(Serialize)]
struct Ok {
    ok: bool,
}

fn ok() -> Json<Ok> {
    Json(Ok { ok: true })
}

// ---- handlers ------------------------------------------------------------

async fn health() -> Json<Ok> {
    ok()
}

/// Start sign-in: mint a one-time link and email it. Always 200, even for an
/// unknown or malformed address, so the endpoint never reveals who has an
/// account.
async fn auth_request(
    State(state): State<AppState>,
    Json(body): Json<AuthRequest>,
) -> AppResult<Json<Ok>> {
    let Some(email) = normalize_email(&body.email) else {
        return Ok(ok());
    };
    let token = generate_token();
    let expires_at = Utc::now() + Duration::minutes(LOGIN_TOKEN_TTL_MINUTES);
    sqlx::query("INSERT INTO login_tokens (token_hash, email, expires_at) VALUES ($1, $2, $3)")
        .bind(hash_token(&token))
        .bind(&email)
        .bind(expires_at)
        .execute(&state.pool)
        .await?;
    let link = format!("{}/auth?token={}", state.frontend_origin, token);
    if let Err(e) = state.mailer.send_login_link(&email, &link).await {
        // Don't fail the request — log and still return 200. A retry just mints
        // another link.
        tracing::error!(error = ?e, "failed to send magic link");
    }
    Ok(ok())
}

/// Complete sign-in: consume the one-time token, upsert the user, mint an opaque
/// session token.
async fn auth_verify(
    State(state): State<AppState>,
    Json(body): Json<VerifyRequest>,
) -> AppResult<Json<VerifyResponse>> {
    // Atomically consume the token: single-use is enforced by `used = FALSE` in
    // the UPDATE, so two concurrent verifies can't both succeed.
    let email: Option<String> = sqlx::query_scalar(
        "UPDATE login_tokens SET used = TRUE \
         WHERE token_hash = $1 AND used = FALSE AND expires_at > now() \
         RETURNING email",
    )
    .bind(hash_token(&body.token))
    .fetch_optional(&state.pool)
    .await?;
    let Some(email) = email else {
        return Err(AppError::BadRequest("invalid or expired link".into()));
    };

    let user_id: Uuid = sqlx::query_scalar(
        "INSERT INTO users (id, email) VALUES ($1, $2) \
         ON CONFLICT (email) DO UPDATE SET email = EXCLUDED.email \
         RETURNING id",
    )
    .bind(Uuid::new_v4())
    .bind(&email)
    .fetch_one(&state.pool)
    .await?;

    let session_token = generate_token();
    let expires_at = Utc::now() + Duration::days(SESSION_TTL_DAYS);
    sqlx::query("INSERT INTO sessions (token_hash, user_id, expires_at) VALUES ($1, $2, $3)")
        .bind(hash_token(&session_token))
        .bind(user_id)
        .bind(expires_at)
        .execute(&state.pool)
        .await?;

    Ok(Json(VerifyResponse {
        session_token,
        email,
    }))
}

async fn auth_logout(State(state): State<AppState>, headers: HeaderMap) -> AppResult<Json<Ok>> {
    // Best-effort: delete whatever session the bearer token names.
    if let Some(token) = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|v| v.trim())
    {
        sqlx::query("DELETE FROM sessions WHERE token_hash = $1")
            .bind(hash_token(token))
            .execute(&state.pool)
            .await?;
    }
    Ok(ok())
}

/// Merge this device's grow-only counter and return the user's cross-device
/// aggregate.
async fn listening_put(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<ListeningPut>,
) -> AppResult<Json<ListeningResponse>> {
    let user_id = authenticate(&state.pool, &headers).await?;
    let device_id = body.device_id.trim();
    if device_id.is_empty() || device_id.len() > MAX_DEVICE_ID_LEN {
        return Err(AppError::BadRequest("invalid deviceId".into()));
    }
    if body.device_total_ms < 0 {
        return Err(AppError::BadRequest("deviceTotalMs must be >= 0".into()));
    }

    sqlx::query(
        "INSERT INTO device_counters (user_id, device_id, total_ms) VALUES ($1, $2, $3) \
         ON CONFLICT (user_id, device_id) \
         DO UPDATE SET total_ms = GREATEST(device_counters.total_ms, EXCLUDED.total_ms), \
                       updated_at = now()",
    )
    .bind(user_id)
    .bind(device_id)
    .bind(body.device_total_ms)
    .execute(&state.pool)
    .await?;

    let server_total_ms = aggregate_total_ms(&state.pool, user_id).await?;
    Ok(Json(ListeningResponse {
        server_total_ms,
        // Echo back what we accepted so the client can advance its synced
        // high-water mark to exactly this.
        synced_through_ms: body.device_total_ms,
    }))
}

async fn listening_get(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<ListeningResponse>> {
    let user_id = authenticate(&state.pool, &headers).await?;
    let server_total_ms = aggregate_total_ms(&state.pool, user_id).await?;
    Ok(Json(ListeningResponse {
        server_total_ms,
        synced_through_ms: 0,
    }))
}

/// Delete all of this user's listening counters (but keep the account). The
/// client rotates its `device_id` alongside this so a stale offline write can't
/// resurrect the deleted total.
async fn listening_delete(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Ok>> {
    let user_id = authenticate(&state.pool, &headers).await?;
    sqlx::query("DELETE FROM device_counters WHERE user_id = $1")
        .bind(user_id)
        .execute(&state.pool)
        .await?;
    Ok(ok())
}

/// Delete the account entirely: the cascade removes sessions and counters too.
async fn account_delete(State(state): State<AppState>, headers: HeaderMap) -> AppResult<Json<Ok>> {
    let user_id = authenticate(&state.pool, &headers).await?;
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user_id)
        .execute(&state.pool)
        .await?;
    Ok(ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn email_normalization() {
        assert_eq!(normalize_email("  Foo@Bar.COM "), Some("foo@bar.com".into()));
        assert_eq!(normalize_email("no-at-sign"), None);
        assert_eq!(normalize_email("a@b"), None); // no dot in domain
        assert_eq!(normalize_email("@bar.com"), None);
    }

    #[test]
    fn token_hash_is_stable_and_opaque() {
        let t = generate_token();
        assert_eq!(hash_token(&t), hash_token(&t));
        assert_ne!(hash_token(&t), t.as_bytes().to_vec());
        assert_eq!(hash_token(&t).len(), 32);
    }

    #[test]
    fn tokens_are_unique() {
        assert_ne!(generate_token(), generate_token());
    }
}
