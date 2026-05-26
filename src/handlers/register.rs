use argon2::{Argon2, PasswordHasher};
use argon2::password_hash::{rand_core::OsRng, SaltString};
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde_json::json;
use uuid::Uuid;
use validator::Validate;
use crate::config::AppState;
use crate::errors::ApiError;
use crate::handlers::otp::create_and_send_email_verification;
use crate::models::RegisterUser;

pub async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterUser>,
) -> Result<impl IntoResponse, ApiError> {
    body.validate()?;

    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(body.password.as_bytes(), &salt)
        .map_err(|e| {
            tracing::error!(email = %body.email, "Erreur hash argon2 : {}", e);
            ApiError::Internal
        })?
        .to_string();

    let id = Uuid::new_v4();

    sqlx::query(
        "INSERT INTO users (id, last_name, first_name, pseudo, email, password, phone) VALUES (?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(id)
    .bind(&body.last_name)
    .bind(&body.first_name)
    .bind(&body.pseudo)
    .bind(&body.email)
    .bind(&password_hash)
    .bind(&body.phone)
    .execute(&state.db)
    .await?;

    // Envoi du code de vérification email (fire-and-forget via le mailer).
    create_and_send_email_verification(&state.db, id, &body.email).await?;

    tracing::info!(email = %body.email, user_id = %id, "Nouveau compte créé — vérification email en attente");
    Ok((
        StatusCode::CREATED,
        Json(json!({
            "status":  "pending_verification",
            "email":   body.email,
            "message": "Compte créé. Vérifie ta boîte mail pour activer ton compte."
        })),
    ))
}
