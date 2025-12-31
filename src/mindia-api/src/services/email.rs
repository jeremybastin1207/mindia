//! Email service for sending usage alert notifications via SMTP.

use lettre::message::header::ContentType;
use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use std::sync::Arc;
use tracing::info;

use mindia_core::Config;

/// Email service for sending alert notifications.
/// No-op if email alerts are disabled or SMTP is not configured.
#[allow(dead_code)]
#[derive(Clone)]
pub struct EmailService {
    mailer: Arc<AsyncSmtpTransport<Tokio1Executor>>,
    from: String,
}

impl EmailService {
    /// Create email service from config. Returns `None` if disabled or SMTP not configured.
    #[allow(dead_code)]
    pub fn from_config(config: &Config) -> Option<Self> {
        if !config.email_alerts_enabled() {
            tracing::debug!("Email alerts disabled (EMAIL_ALERTS_ENABLED=false)");
            return None;
        }
        let host = config.smtp_host()?;
        let from = config.smtp_from()?.to_string();
        let port = config.smtp_port().unwrap_or(587);

        let mailer = if config.smtp_tls() {
            let b = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(host).ok()?;
            let b = b.port(port);
            let b = if let (Some(u), Some(p)) = (config.smtp_user(), config.smtp_password()) {
                b.credentials(Credentials::new(u.to_string(), p.to_string()))
            } else {
                b
            };
            tracing::info!(
                host = %host,
                port = port,
                "Email service initialized (SMTP with STARTTLS)"
            );
            b.build()
        } else {
            let b = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(host).port(port);
            let b = if let (Some(u), Some(p)) = (config.smtp_user(), config.smtp_password()) {
                b.credentials(Credentials::new(u.to_string(), p.to_string()))
            } else {
                b
            };
            tracing::info!(host = %host, port = port, "Email service initialized (SMTP)");
            b.build()
        };

        Some(Self {
            mailer: Arc::new(mailer),
            from,
        })
    }

    /// Send a plain-text email to the given recipients.
    #[allow(dead_code)]
    pub async fn send(&self, to: &[String], subject: &str, body_plain: &str) -> Result<(), String> {
        if to.is_empty() {
            return Ok(());
        }
        let to_addrs: Vec<Mailbox> = to.iter().filter_map(|s| s.parse().ok()).collect::<Vec<_>>();
        if to_addrs.is_empty() {
            return Err("No valid recipient addresses".to_string());
        }
        let from_addr: Mailbox = self
            .from
            .parse()
            .map_err(|e| format!("Invalid SMTP_FROM: {}", e))?;

        let mut builder = Message::builder().from(from_addr).subject(subject);
        for mb in &to_addrs {
            builder = builder.to(mb.clone());
        }
        let email = builder
            .header(ContentType::TEXT_PLAIN)
            .body(body_plain.to_string())
            .map_err(|e| e.to_string())?;

        self.mailer.send(email).await.map_err(|e| e.to_string())?;
        info!(count = to.len(), "Usage alert email sent");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// EmailService::from_config returns None when email alerts are disabled.
    #[test]
    fn from_config_returns_none_when_email_disabled() {
        std::env::set_var("ENVIRONMENT", "development");
        std::env::set_var("DATABASE_URL", "postgresql://localhost/test");
        std::env::set_var("JWT_SECRET", "test-secret-key-min-32-characters-long");
        std::env::set_var("STORAGE_BACKEND", "local");
        std::env::set_var("LOCAL_STORAGE_PATH", "/tmp/mindia-test");
        std::env::set_var("LOCAL_STORAGE_BASE_URL", "http://localhost:3000");
        std::env::set_var("EMAIL_ALERTS_ENABLED", "false");
        let config = mindia_core::Config::from_env().expect("test config from env");
        assert!(
            EmailService::from_config(&config).is_none(),
            "When EMAIL_ALERTS_ENABLED=false, from_config should return None"
        );
    }
}
