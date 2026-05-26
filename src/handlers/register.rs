use argon2::{Argon2, PasswordHasher};
use argon2::password_hash::{rand_core::OsRng, SaltString};
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde_json::json;
use uuid::Uuid;
use validator::Validate;
use crate::config::AppState;
use crate::errors::ApiError;
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

    tracing::info!(email = %body.email, user_id = %id, "Nouveau compte créé");
    Ok((
        StatusCode::CREATED,
        Json(json!({ "status": "success", "message": "Compte créé avec succès" })),
    ))
}
