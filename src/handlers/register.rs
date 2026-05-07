use argon2::{Argon2, PasswordHasher};
use argon2::password_hash::{rand_core::OsRng, SaltString};
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde_json::json;
use uuid::Uuid;
use validator::Validate;
use crate::config::AppState;
use crate::models::RegisterUser;

pub async fn register(State(state): State<AppState>, Json(body): Json<RegisterUser>) -> impl IntoResponse {
    if let Err(e) = body.validate() {
        let first_msg = e.field_errors()
            .values()
            .flat_map(|v| v.iter())
            .next()
            .and_then(|err| err.message.as_ref())
            .map(|cow| cow.to_string())
            .unwrap_or_else(|| "Données invalides".to_string());
        tracing::warn!(email = %body.email, "Inscription invalide : {}", first_msg);
        return (StatusCode::BAD_REQUEST, Json(json!({ "error": first_msg }))).into_response();
    }

    let salt = SaltString::generate(&mut OsRng);
    let password_hash = match Argon2::default().hash_password(body.password.as_bytes(), &salt) {
        Ok(hash) => hash.to_string(),
        Err(e) => {
            tracing::error!(email = %body.email, "Erreur hash argon2 : {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Erreur serveur" }))).into_response();
        }
    };

    let id = Uuid::new_v4();

    let result = sqlx::query(
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
    .await;

    match result {
        Ok(_) => {
            tracing::info!(email = %body.email, user_id = %id, "Nouveau compte créé");
            (StatusCode::CREATED, Json(json!({ "status": "success", "message": "Compte créé avec succès" }))).into_response()
        }
        Err(e) => {
            if e.to_string().contains("Duplicate entry") {
                tracing::warn!(email = %body.email, "Tentative d'inscription avec email déjà utilisé");
                (StatusCode::CONFLICT, Json(json!({ "error": "Cet email est déjà utilisé" }))).into_response()
            } else {
                tracing::error!(email = %body.email, "Erreur DB inscription : {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Erreur lors de la création du compte" }))).into_response()
            }
        }
    }
}