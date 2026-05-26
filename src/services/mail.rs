use std::env;
use std::time::Duration;
use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::AsyncSmtpTransport;
use lettre::{AsyncTransport, Message, Tokio1Executor};

pub struct MailerConfig {
    pub host:     String,
    pub port:     u16,
    pub user:     String,
    pub password: String,
    pub from:     String,
}

impl MailerConfig {
    pub fn from_env() -> Option<Self> {
        let host     = env::var("SMTP_HOST").ok()?;
        let user     = env::var("SMTP_USER").ok()?;
        let password = env::var("SMTP_PASSWORD").ok()?;
        let port     = env::var("SMTP_PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(587);
        let from     = env::var("SMTP_FROM").unwrap_or_else(|_| user.clone());

        if host.is_empty() || user.is_empty() || password.is_empty() {
            return None;
        }

        Some(Self { host, port, user, password, from })
    }
}

/// Envoie l'email en *fire-and-forget* (spawne une task tokio) pour ne pas
/// bloquer la réponse HTTP. Loggue le résultat asynchrone.
pub fn send_password_reset(to: String, code: String) {
    let Some(config) = MailerConfig::from_env() else {
        tracing::warn!(
            email = %to,
            code = %code,
            "Mailer mock — config SMTP absente. Code à utiliser : {}",
            code
        );
        return;
    };

    tokio::spawn(async move {
        match do_send(&config, &to, &code).await {
            Ok(()) => tracing::info!(to = %to, "Email reset password envoyé"),
            Err(e) => tracing::error!(to = %to, "Échec envoi email : {}", e),
        }
    });
}

async fn do_send(config: &MailerConfig, to: &str, code: &str) -> Result<(), String> {
    let body_text = format!(
        "Bonjour,\n\n\
        Vous avez demandé une réinitialisation de votre mot de passe Horizon Moto.\n\n\
        Votre code de réinitialisation : {}\n\n\
        Ce code expire dans 1 heure. Si vous n'êtes pas à l'origine de cette demande, \
        ignorez simplement cet email.\n\n\
        — L'équipe Horizon Moto",
        code
    );

    let body_html = format!(
        r#"<!DOCTYPE html>
<html><body style="font-family:-apple-system,BlinkMacSystemFont,sans-serif;max-width:480px;margin:auto;padding:24px;color:#111">
  <h2 style="color:#E3000F">Réinitialisation de mot de passe</h2>
  <p>Bonjour,</p>
  <p>Vous avez demandé une réinitialisation de votre mot de passe <strong>Horizon Moto</strong>.</p>
  <p style="font-size:14px;color:#666">Votre code de vérification :</p>
  <div style="font-size:36px;font-weight:700;letter-spacing:8px;text-align:center;background:#F5F5F5;padding:20px;border-radius:12px;color:#111">{}</div>
  <p style="margin-top:24px;color:#666;font-size:13px">Ce code expire dans 1 heure. Si vous n'êtes pas à l'origine de cette demande, ignorez simplement cet email.</p>
  <hr style="border:none;border-top:1px solid #E5E5E5;margin:32px 0" />
  <p style="font-size:12px;color:#999">— L'équipe Horizon Moto</p>
</body></html>"#,
        code
    );

    let from_address = config.from.parse().map_err(|e| format!("SMTP_FROM invalide : {}", e))?;
    let to_address   = to.parse().map_err(|e| format!("Destinataire invalide : {}", e))?;

    let email = Message::builder()
        .from(from_address)
        .to(to_address)
        .subject("Réinitialisation de votre mot de passe Horizon Moto")
        .multipart(
            lettre::message::MultiPart::alternative()
                .singlepart(
                    lettre::message::SinglePart::builder()
                        .header(ContentType::TEXT_PLAIN)
                        .body(body_text),
                )
                .singlepart(
                    lettre::message::SinglePart::builder()
                        .header(ContentType::TEXT_HTML)
                        .body(body_html),
                ),
        )
        .map_err(|e| format!("Construction email : {}", e))?;

    let creds = Credentials::new(config.user.clone(), config.password.clone());

    // Auto-detect du mode TLS selon le port :
    //   465 → SSL direct (implicit TLS)   ← recommandé par Hostinger
    //   587 → STARTTLS                    ← fallback standard
    //   autre → STARTTLS
    let builder = if config.port == 465 {
        AsyncSmtpTransport::<Tokio1Executor>::relay(&config.host)
    } else {
        AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.host)
    };

    let mailer: AsyncSmtpTransport<Tokio1Executor> = builder
        .map_err(|e| format!("Init SMTP transport : {}", e))?
        .port(config.port)
        .timeout(Some(Duration::from_secs(10)))
        .credentials(creds)
        .build();

    // Hard timeout total pour ne jamais traîner indéfiniment.
    tokio::time::timeout(Duration::from_secs(15), mailer.send(email))
        .await
        .map_err(|_| "timeout 15s (SMTP injoignable ou auth bloquée)".to_string())?
        .map_err(|e| format!("send : {}", e))?;

    Ok(())
}
