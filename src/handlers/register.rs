use argon2::{Argon2, PasswordHasher};
use argon2::password_hash::{rand_core::OsRng, SaltString};
use axum::{Json, extract::State};
use serde_json::{json, Value};
use uuid::Uuid;
use crate::config::AppState;
use crate::models::RegisterUser;

pub async fn register(State(state): State<AppState>, Json(body): Json<RegisterUser>) -> Json<Value> {
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = match Argon2::default().hash_password(body.password.as_bytes(), &salt) {
        Ok(hash) => hash.to_string(),
        Err(_) => return Json(json!({ "status": "error", "message": "Erreur serveur" })),
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
        Ok(_) => Json(json!({ "status": "success", "message": "Compte créé avec succès" })),
        Err(e) => {
            if e.to_string().contains("Duplicate entry") {
                Json(json!({ "status": "error", "message": "Cet email est déjà utilisé" }))
            } else {
                Json(json!({ "status": "error", "message": "Erreur lors de la création du compte" }))
            }
        }
    }
}
