use argon2::{Argon2, PasswordVerifier};
use argon2::password_hash::PasswordHash;
use crate::config::AppState;
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use chrono::{Duration, Utc};
use rand::RngCore;
use serde_json::json;
use sha2::{Digest, Sha256};
use jsonwebtoken::{encode, Header, EncodingKey};
use sqlx::MySqlPool;
use uuid::Uuid;
use validator::Validate;
use crate::errors::ApiError;
use crate::handlers::otp::create_and_send_email_verification;
use crate::models::claim::Claims;
use crate::models::user::{AuthUser, LoginRequest, User};

pub const ACCESS_TOKEN_LIFETIME_MINUTES: i64 = 60;
pub const REFRESH_TOKEN_LIFETIME_DAYS: i64 = 30;

pub async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> Result<axum::response::Response, ApiError> {
    if let Err(e) = body.validate() {
        let err: ApiError = e.into();
        tracing::warn!(email = %body.email, "Tentative de login invalide");
        return Err(err);
    }

    let user = sqlx::query_as::<_, AuthUser>(
        "SELECT id, email, password, email_verified FROM users WHERE email = ?"
    )
    .bind(&body.email)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| {
        tracing::warn!(email = %body.email, "Email introuvable");
        ApiError::Unauthorized
    })?;

    let password_hash = user.password.as_deref().ok_or_else(|| {
        tracing::warn!(email = %body.email, "Tentative login sur compte Google (pas de password)");
        ApiError::Unauthorized
    })?;

    let parsed_hash = PasswordHash::new(password_hash).map_err(|e| {
        tracing::error!(email = %body.email, "Hash argon2 invalide en DB : {}", e);
        ApiError::Internal
    })?;

    if Argon2::default()
        .verify_password(body.password.as_bytes(), &parsed_hash)
        .is_err()
    {
        tracing::warn!(email = %body.email, "Mot de passe incorrect");
        return Err(ApiError::Unauthorized);
    }

    if !user.email_verified {
        tracing::info!(email = %body.email, "Login refusé : email non vérifié, envoi d'un nouveau code");
        let _ = create_and_send_email_verification(&state.db, user.id, &body.email).await;
        return Ok((
            StatusCode::FORBIDDEN,
            Json(json!({
                "error":              "Email non vérifié",
                "needs_verification": true,
                "email":              body.email,
            })),
        ).into_response());
    }

    let full_user = sqlx::query_as::<_, User>(
        "SELECT id, last_name, first_name, pseudo, email, password, birth_date, phone, pp, point, `rank`, `role`, email_verified FROM users WHERE id = ?"
    )
    .bind(user.id)
    .fetch_one(&state.db)
    .await?;

    let access = generate_access_token(user.id.to_string(), full_user.role.clone(), &state.jwt_secret)?;
    let refresh = issue_refresh_token(&state.db, user.id).await?;

    tracing::info!(email = %body.email, user_id = %user.id, "Login réussi");

    Ok(Json(json!({
        "access_token":  access,
        "refresh_token": refresh,
        "user":          full_user,
    })).into_response())
}

/// Token d'accès court (60 min). Stocké uniquement côté client en mémoire/
/// SecureStore, renouvelé via le refresh token.
pub fn generate_access_token(user_id: String, role: String, secret: &str) -> Result<String, ApiError> {
    let expiration = Utc::now()
        .checked_add_signed(Duration::minutes(ACCESS_TOKEN_LIFETIME_MINUTES))
        .ok_or(ApiError::Internal)?
        .timestamp() as usize;

    let claims = Claims { sub: user_id, exp: expiration, role: Some(role) };
    encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_bytes()))
        .map_err(|e| {
            tracing::error!("Erreur encodage JWT : {}", e);
            ApiError::Internal
        })
}

/// Génère un refresh token opaque (32 bytes hex), persiste son hash SHA-256
/// en base et retourne la valeur brute (à n'envoyer qu'une fois au client).
pub async fn issue_refresh_token(db: &MySqlPool, user_id: Uuid) -> Result<String, ApiError> {
    let mut buf = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut buf);
    let token: String = buf.iter().map(|b| format!("{:02x}", b)).collect();
    let token_hash = hash_refresh_token(&token);

    let expires_at = Utc::now()
        .checked_add_signed(Duration::days(REFRESH_TOKEN_LIFETIME_DAYS))
        .ok_or(ApiError::Internal)?;

    sqlx::query(
        "INSERT INTO refresh_tokens (id, user_id, token_hash, expires_at)
         VALUES (?, ?, ?, ?)",
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .bind(&token_hash)
    .bind(expires_at)
    .execute(db)
    .await?;

    Ok(token)
}

pub fn hash_refresh_token(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    digest.iter().map(|b| format!("{:02x}", b)).collect()
}
