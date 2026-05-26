use argon2::password_hash::{rand_core::OsRng, SaltString};
use argon2::{Argon2, PasswordHasher};
use axum::extract::State;
use axum::Json;
use axum::response::IntoResponse;
use chrono::{Duration, Utc};
use rand::Rng;
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use uuid::Uuid;
use validator::Validate;
use crate::config::AppState;
use crate::errors::ApiError;
use crate::services::mail;

#[derive(Deserialize, Validate)]
pub struct RequestResetBody {
    #[validate(email(message = "Adresse email invalide"))]
    pub email: String,
}

#[derive(Deserialize, Validate)]
pub struct VerifyCodeBody {
    #[validate(email(message = "Adresse email invalide"))]
    pub email: String,
    #[validate(length(equal = 6, message = "Le code doit contenir 6 chiffres"))]
    pub code: String,
}

#[derive(Deserialize, Validate)]
pub struct ConfirmResetBody {
    #[validate(email(message = "Adresse email invalide"))]
    pub email: String,
    #[validate(length(equal = 6, message = "Le code doit contenir 6 chiffres"))]
    pub code: String,
    #[validate(length(min = 8, max = 72, message = "Le mot de passe doit contenir entre 8 et 72 caractères"))]
    pub password: String,
}

const TOKEN_LIFETIME_MINUTES: i64 = 60;

/// Code 6 chiffres (000000 à 999999) — facile à taper depuis un email.
fn generate_code() -> String {
    let n: u32 = rand::thread_rng().gen_range(0..1_000_000);
    format!("{:06}", n)
}

/// SHA-256 hex du code, pour comparer sans stocker la valeur brute en DB.
fn hash_code(code: &str) -> String {
    let digest = Sha256::digest(code.as_bytes());
    digest.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Cherche un token actif (non utilisé et non expiré) pour un couple (email, code).
async fn find_active_token(
    db: &sqlx::Pool<sqlx::MySql>,
    email: &str,
    code: &str,
) -> Result<(Uuid, Uuid), ApiError> {
    let code_hash = hash_code(code);

    let row: Option<(Uuid, Uuid)> = sqlx::query_as(
        "SELECT prt.id, prt.user_id
         FROM password_reset_tokens prt
         JOIN users u ON u.id = prt.user_id
         WHERE u.email = ?
           AND prt.token_hash = ?
           AND prt.used_at IS NULL
           AND prt.expires_at > NOW()",
    )
    .bind(email)
    .bind(&code_hash)
    .fetch_optional(db)
    .await?;

    row.ok_or_else(|| {
        tracing::warn!(email = %email, "Code reset invalide / expiré / déjà utilisé");
        ApiError::BadRequest("Code invalide ou expiré".into())
    })
}

pub async fn request_password_reset(
    State(state): State<AppState>,
    Json(body): Json<RequestResetBody>,
) -> Result<impl IntoResponse, ApiError> {
    body.validate()?;

    let user_id: Option<Uuid> = sqlx::query_scalar("SELECT id FROM users WHERE email = ?")
        .bind(&body.email)
        .fetch_optional(&state.db)
        .await?;

    // Réponse identique que l'email existe ou non (pas de leak d'énumération).
    if let Some(uid) = user_id {
        let code = generate_code();
        let code_hash = hash_code(&code);
        let expires_at = Utc::now()
            .checked_add_signed(Duration::minutes(TOKEN_LIFETIME_MINUTES))
            .ok_or(ApiError::Internal)?;

        // Invalide les anciens codes pending — un seul code actif à la fois.
        sqlx::query(
            "UPDATE password_reset_tokens SET used_at = NOW()
             WHERE user_id = ? AND used_at IS NULL",
        )
        .bind(uid)
        .execute(&state.db)
        .await?;

        sqlx::query(
            "INSERT INTO password_reset_tokens (id, user_id, token_hash, expires_at)
             VALUES (?, ?, ?, ?)",
        )
        .bind(Uuid::new_v4())
        .bind(uid)
        .bind(&code_hash)
        .bind(expires_at)
        .execute(&state.db)
        .await?;

        // Envoi de l'email en fire-and-forget (la réponse HTTP n'attend pas le SMTP).
        // Le résultat est loggé via tracing.
        mail::send_password_reset(body.email.clone(), code);
    } else {
        tracing::info!(email = %body.email, "Demande reset password pour email inconnu (réponse silencieuse)");
    }

    Ok(Json(json!({
        "status":  "success",
        "message": "Si cet email existe, un code de réinitialisation a été envoyé."
    })))
}

/// Vérifie qu'un code est valide sans le consommer. Permet au front de gater
/// l'accès à l'écran de définition du nouveau mot de passe.
pub async fn verify_reset_code(
    State(state): State<AppState>,
    Json(body): Json<VerifyCodeBody>,
) -> Result<impl IntoResponse, ApiError> {
    body.validate()?;
    find_active_token(&state.db, &body.email, &body.code).await?;

    Ok(Json(json!({
        "status":  "success",
        "message": "Code valide"
    })))
}

pub async fn confirm_password_reset(
    State(state): State<AppState>,
    Json(body): Json<ConfirmResetBody>,
) -> Result<impl IntoResponse, ApiError> {
    body.validate()?;

    let (token_id, user_id) = find_active_token(&state.db, &body.email, &body.code).await?;

    let salt = SaltString::generate(&mut OsRng);
    let new_hash = Argon2::default()
        .hash_password(body.password.as_bytes(), &salt)
        .map_err(|e| {
            tracing::error!(user_id = %user_id, "Erreur hash argon2 : {}", e);
            ApiError::Internal
        })?
        .to_string();

    let mut tx = state.db.begin().await?;

    sqlx::query("UPDATE users SET password = ? WHERE id = ?")
        .bind(&new_hash)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query("UPDATE password_reset_tokens SET used_at = NOW() WHERE id = ?")
        .bind(token_id)
        .execute(&mut *tx)
        .await?;

    // Sécurité : invalide aussi tous les autres tokens pending pour cet utilisateur.
    sqlx::query(
        "UPDATE password_reset_tokens SET used_at = NOW()
         WHERE user_id = ? AND used_at IS NULL AND id != ?",
    )
    .bind(user_id)
    .bind(token_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    tracing::info!(user_id = %user_id, "Mot de passe réinitialisé");

    Ok(Json(json!({
        "status":  "success",
        "message": "Mot de passe mis à jour. Vous pouvez vous reconnecter."
    })))
}
