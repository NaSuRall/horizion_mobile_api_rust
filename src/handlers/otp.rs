use axum::extract::State;
use axum::Json;
use axum::response::IntoResponse;
use chrono::{DateTime, Duration, Utc};
use rand::Rng;
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::MySqlPool;
use uuid::Uuid;
use validator::Validate;
use crate::config::AppState;
use crate::errors::ApiError;
use crate::handlers::login::{generate_access_token, issue_refresh_token};
use crate::models::user::User;
use crate::services::mail;

#[derive(Deserialize, Validate)]
pub struct VerifyEmailBody {
    #[validate(email(message = "Adresse email invalide"))]
    pub email: String,
    #[validate(length(equal = 6, message = "Le code doit contenir 6 chiffres"))]
    pub code: String,
}

#[derive(Deserialize, Validate)]
pub struct ResendCodeBody {
    #[validate(email(message = "Adresse email invalide"))]
    pub email: String,
}

const OTP_LIFETIME_MINUTES: i64 = 15;
const RESEND_COOLDOWN_SECONDS: i64 = 60;

fn generate_code() -> String {
    let n: u32 = rand::thread_rng().gen_range(0..1_000_000);
    format!("{:06}", n)
}

fn hash_code(code: &str) -> String {
    let digest = Sha256::digest(code.as_bytes());
    digest.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Helper réutilisable : génère un OTP de vérification email, le persiste,
/// invalide les anciens codes pending, et déclenche l'envoi mail.
pub async fn create_and_send_email_verification(
    db: &MySqlPool,
    user_id: Uuid,
    email: &str,
) -> Result<(), ApiError> {
    let code = generate_code();
    let code_hash = hash_code(&code);
    let expires_at = Utc::now()
        .checked_add_signed(Duration::minutes(OTP_LIFETIME_MINUTES))
        .ok_or(ApiError::Internal)?;

    // Un seul code actif à la fois → on consume les pending d'abord.
    sqlx::query(
        "UPDATE otp_codes SET used_at = NOW()
         WHERE user_id = ? AND purpose = 'verify_email' AND used_at IS NULL",
    )
    .bind(user_id)
    .execute(db)
    .await?;

    sqlx::query(
        "INSERT INTO otp_codes (id, user_id, purpose, code_hash, expires_at)
         VALUES (?, ?, 'verify_email', ?, ?)",
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .bind(&code_hash)
    .bind(expires_at)
    .execute(db)
    .await?;

    mail::send_email_verification(email.to_string(), code);
    Ok(())
}

pub async fn verify_email(
    State(state): State<AppState>,
    Json(body): Json<VerifyEmailBody>,
) -> Result<impl IntoResponse, ApiError> {
    body.validate()?;

    let code_hash = hash_code(&body.code);

    let row: Option<(Uuid, Uuid)> = sqlx::query_as(
        "SELECT otp.id, otp.user_id
         FROM otp_codes otp
         JOIN users u ON u.id = otp.user_id
         WHERE u.email = ?
           AND otp.purpose = 'verify_email'
           AND otp.code_hash = ?
           AND otp.used_at IS NULL
           AND otp.expires_at > NOW()",
    )
    .bind(&body.email)
    .bind(&code_hash)
    .fetch_optional(&state.db)
    .await?;

    let (otp_id, user_id) = row.ok_or_else(|| {
        tracing::warn!(email = %body.email, "OTP verify-email : code invalide ou expiré");
        ApiError::BadRequest("Code invalide ou expiré".into())
    })?;

    let mut tx = state.db.begin().await?;

    sqlx::query("UPDATE users SET email_verified = 1 WHERE id = ?")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query("UPDATE otp_codes SET used_at = NOW() WHERE id = ?")
        .bind(otp_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    // Charge le user complet et émet un JWT pour auto-login.
    let full_user = sqlx::query_as::<_, User>(
        "SELECT id, last_name, first_name, pseudo, email, password, birth_date, phone, pp, point, `rank`, `role`, email_verified
         FROM users WHERE id = ?",
    )
    .bind(user_id)
    .fetch_one(&state.db)
    .await?;

    let access  = generate_access_token(user_id.to_string(), full_user.role.clone(), &state.jwt_secret)?;
    let refresh = issue_refresh_token(&state.db, user_id).await?;

    tracing::info!(user_id = %user_id, email = %body.email, "Email vérifié — auto-login");

    Ok(Json(json!({
        "status":        "success",
        "access_token":  access,
        "refresh_token": refresh,
        "user":          full_user,
    })))
}

pub async fn resend_verification_code(
    State(state): State<AppState>,
    Json(body): Json<ResendCodeBody>,
) -> Result<impl IntoResponse, ApiError> {
    body.validate()?;

    // Cherche le user et son statut (sans révéler s'il existe).
    let row: Option<(Uuid, bool)> = sqlx::query_as(
        "SELECT id, email_verified FROM users WHERE email = ?",
    )
    .bind(&body.email)
    .fetch_optional(&state.db)
    .await?;

    if let Some((user_id, verified)) = row {
        if verified {
            // Email déjà vérifié — on ne renvoie pas de code mais on ne le dit pas
            // explicitement pour ne pas leaker le statut du compte.
            tracing::info!(email = %body.email, "Resend OTP demandé sur compte déjà vérifié");
        } else {
            // Cooldown : refuse si un code a été envoyé il y a moins de 60s.
            let last_sent: Option<DateTime<Utc>> = sqlx::query_scalar(
                "SELECT created_at FROM otp_codes
                 WHERE user_id = ? AND purpose = 'verify_email'
                 ORDER BY created_at DESC LIMIT 1",
            )
            .bind(user_id)
            .fetch_optional(&state.db)
            .await?;

            if let Some(last) = last_sent {
                let elapsed = (Utc::now() - last).num_seconds();
                if elapsed < RESEND_COOLDOWN_SECONDS {
                    let wait = RESEND_COOLDOWN_SECONDS - elapsed;
                    return Err(ApiError::BadRequest(format!(
                        "Attendez {}s avant de demander un nouveau code",
                        wait
                    )));
                }
            }

            create_and_send_email_verification(&state.db, user_id, &body.email).await?;
        }
    } else {
        tracing::info!(email = %body.email, "Resend OTP pour email inconnu (réponse silencieuse)");
    }

    Ok(Json(json!({
        "status":  "success",
        "message": "Si cet email existe et n'est pas vérifié, un nouveau code a été envoyé."
    })))
}
