//! Magic-link delivery.
//!
//! In production this sends through SMTP (credentials from the environment). In
//! development, or when SMTP isn't configured, it falls back to logging the
//! link so the flow is exercisable without a mail server.

use anyhow::Context;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

#[derive(Clone)]
pub enum Mailer {
    Smtp {
        transport: AsyncSmtpTransport<Tokio1Executor>,
        from: String,
    },
    /// No SMTP configured — log the link instead of sending it.
    LogOnly,
}

impl Mailer {
    /// Build an SMTP mailer from explicit settings, or a log-only mailer if
    /// `host` is empty.
    pub fn from_settings(
        host: &str,
        port: u16,
        username: &str,
        password: &str,
        from: &str,
    ) -> anyhow::Result<Self> {
        if host.is_empty() {
            tracing::warn!("SMTP_HOST not set — magic links will be logged, not emailed");
            return Ok(Mailer::LogOnly);
        }
        let creds = Credentials::new(username.to_string(), password.to_string());
        // Port 465 is implicit TLS (SMTPS); 587 (submission) and anything else
        // negotiate TLS via STARTTLS after a plaintext greeting. Using the wrong
        // one yields "received corrupt message of type InvalidContentType".
        let builder = if port == 465 {
            AsyncSmtpTransport::<Tokio1Executor>::relay(host).context("invalid SMTP host")?
        } else {
            AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(host)
                .context("invalid SMTP host")?
        };
        let transport = builder.port(port).credentials(creds).build();
        Ok(Mailer::Smtp {
            transport,
            from: from.to_string(),
        })
    }

    /// Send a sign-in link to `to`. The link is already a full URL.
    pub async fn send_login_link(&self, to: &str, link: &str) -> anyhow::Result<()> {
        match self {
            Mailer::LogOnly => {
                tracing::info!(%to, %link, "magic link (log-only mailer)");
                Ok(())
            }
            Mailer::Smtp { transport, from } => {
                let body = format!(
                    "Hi,\n\nClick to sign in to Cascade and sync your listening time:\n\n{link}\n\n\
                     This link expires in 15 minutes and can only be used once. \
                     If you didn't request it, you can ignore this email.\n\n— Cascade"
                );
                let email = Message::builder()
                    .from(from.parse().context("invalid From address")?)
                    .to(to.parse().context("invalid recipient address")?)
                    .subject("Your Cascade sign-in link")
                    .body(body)
                    .context("could not build email")?;
                transport.send(email).await.context("SMTP send failed")?;
                Ok(())
            }
        }
    }
}
