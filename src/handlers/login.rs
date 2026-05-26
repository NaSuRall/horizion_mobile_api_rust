use argon2::{Argon2, PasswordVerifier};
use argon2::password_hash::PasswordHash;
use crate::config::AppState;
use axum::Json;
use axum::extract::State;
use axum::response::IntoResponse;
use chrono::{Duration, Utc};
use serde_json::json;
use jsonwebtoken::{encode, Header, EncodingKey};
use validator::Validate;
use crate::errors::ApiError;
use crate::models::claim::Claims;
use crate::models::user::{AuthUser, LoginRequest, User};

pub async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> Result<impl IntoResponse, ApiError> {
    if let Err(e) = body.validate() {
        let err: ApiError = e.into();
        tracing::warn!(email = %body.email, "Tentative de login invalide");
        return Err(err);
    }

    let user = sqlx::query_as::<_, AuthUser>(
        "SELECT id, email, password FROM users WHERE email = ?"
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

    let full_user = sqlx::query_as::<_, User>(
        "SELECT id, last_name, first_name, pseudo, email, password, birth_date, phone, pp, point, `rank`, `role` FROM users WHERE id = ?"
    )
    .bind(user.id)
    .fetch_one(&state.db)
    .await?;

    let token = generate_token(user.id.to_string(), full_user.role.clone(), &state.jwt_secret)?;
    tracing::info!(email = %body.email, user_id = %user.id, "Login réussi");

    Ok(Json(json!({ "token": token, "user": full_user })))
}

pub fn generate_token(user_id: String, role: String, secret: &str) -> Result<String, ApiError> {
    let expiration = Utc::now()
        .checked_add_signed(Duration::days(30))
        .ok_or(ApiError::Internal)?
        .timestamp() as usize;

    let claims = Claims { sub: user_id, exp: expiration, role: Some(role) };
    encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_bytes()))
        .map_err(|e| {
            tracing::error!("Erreur encodage JWT : {}", e);
            ApiError::Internal
        })
}
