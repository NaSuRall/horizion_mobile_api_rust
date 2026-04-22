use argon2::{Argon2, PasswordVerifier};
use argon2::password_hash::PasswordHash;
use crate::config::AppState;
use axum::Json;
use axum::extract::State;
use chrono::{Duration, Utc};
use serde_json::{Value, json};
use jsonwebtoken::{encode, Header, EncodingKey};
use crate::models::claim::Claims;
use crate::models::user::{AuthUser, LoginRequest, User};

pub async fn login(State(state): State<AppState>, Json(body): Json<LoginRequest>) -> Json<Value> {

    let user = sqlx::query_as::<_, AuthUser>(
        "SELECT id, email, password FROM users WHERE email = ?"
    )
    .bind(&body.email)
    .fetch_optional(&state.db)
    .await;

    match user {
        Ok(Some(user)) => {
            let parsed_hash = match PasswordHash::new(&user.password) {
                Ok(h) => h,
                Err(_) => return Json(json!({ "status": "error", "message": "Erreur serveur" })),
            };

            if Argon2::default().verify_password(body.password.as_bytes(), &parsed_hash).is_err() {
                return Json(json!({
                    "status": "error",
                    "message": "Email ou mot de passe incorrect"
                }));
            }

            let full_user = sqlx::query_as::<_, User>(
                "SELECT id, last_name, first_name, pseudo, email, password, birth_date, phone, pp, point, `rank` FROM users WHERE id = ?"
            )
            .bind(user.id)
            .fetch_one(&state.db)
            .await;

            match full_user {
                Ok(full_user) => {
                    let role: String = sqlx::query_scalar("SELECT `role` FROM users WHERE id = ?")
                        .bind(user.id)
                        .fetch_one(&state.db)
                        .await
                        .unwrap_or_else(|_| "user".to_string());
                    let token = generate_token(user.id.to_string(), role);
                    Json(json!({ "token": token, "user": full_user }))
                }
                Err(_) => Json(json!({
                    "status": "error",
                    "message": "Impossible de récupérer l'utilisateur"
                })),
            }
        }

        Ok(None) => Json(json!({
            "status": "error",
            "message": "Email ou mot de passe incorrect"
        })),

        Err(_) => Json(json!({
            "status": "error",
            "message": "Une erreur est survenue"
        })),
    }
}

pub fn generate_token(user_id: String, role: String) -> String {
    let expiration = Utc::now()
        .checked_add_signed(Duration::days(30))
        .unwrap()
        .timestamp() as usize;

    let claims = Claims { sub: user_id, exp: expiration, role: Some(role) };
    let secret = std::env::var("JWT_SECRET").expect("JWT_SECRET manquant dans .env");

    encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_bytes())).unwrap()
}
