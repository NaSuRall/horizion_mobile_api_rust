use argon2::{Argon2, PasswordVerifier};
use argon2::password_hash::PasswordHash;
use crate::config::AppState;
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use chrono::{Duration, Utc};
use serde_json::json;
use jsonwebtoken::{encode, Header, EncodingKey};
use validator::Validate;
use crate::models::claim::Claims;
use crate::models::user::{AuthUser, LoginRequest, User};

pub async fn login(State(state): State<AppState>, Json(body): Json<LoginRequest>) -> impl IntoResponse {
    if let Err(e) = body.validate() {
        let first_msg = e.field_errors()
            .values()
            .flat_map(|v| v.iter())
            .next()
            .and_then(|err| err.message.as_ref())
            .map(|cow| cow.to_string())
            .unwrap_or_else(|| "Données invalides".to_string());
        tracing::warn!(email = %body.email, "Tentative de login invalide : {}", first_msg);
        return (StatusCode::BAD_REQUEST, Json(json!({ "error": first_msg }))).into_response();
    }

    let user = sqlx::query_as::<_, AuthUser>(
        "SELECT id, email, password FROM users WHERE email = ?"
    )
    .bind(&body.email)
    .fetch_optional(&state.db)
    .await;

    match user {
        Ok(Some(user)) => {
            let password_hash = match user.password.as_deref() {
                Some(p) => p.to_string(),
                None => {
                    tracing::warn!(email = %body.email, "Tentative login sur compte Google (pas de password)");
                    return (StatusCode::UNAUTHORIZED, Json(json!({ "error": "Email ou mot de passe incorrect" }))).into_response();
                }
            };
            let parsed_hash = match PasswordHash::new(&password_hash) {
                Ok(h) => h,
                Err(e) => {
                    tracing::error!(email = %body.email, "Hash argon2 invalide en DB : {}", e);
                    return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Erreur serveur" }))).into_response();
                }
            };

            if Argon2::default().verify_password(body.password.as_bytes(), &parsed_hash).is_err() {
                tracing::warn!(email = %body.email, "Mot de passe incorrect");
                return (StatusCode::UNAUTHORIZED, Json(json!({
                    "error": "Email ou mot de passe incorrect"
                }))).into_response();
            }

            let full_user = sqlx::query_as::<_, User>(
                "SELECT id, last_name, first_name, pseudo, email, password, birth_date, phone, pp, point, `rank`, `role` FROM users WHERE id = ?"
            )
            .bind(user.id)
            .fetch_one(&state.db)
            .await;

            match full_user {
                Ok(full_user) => {
                    tracing::info!(email = %body.email, user_id = %user.id, "Login réussi");
                    let token = generate_token(user.id.to_string(), full_user.role.clone(), &state.jwt_secret);
                    (StatusCode::OK, Json(json!({ "token": token, "user": full_user }))).into_response()
                }
                Err(e) => {
                    tracing::error!(user_id = %user.id, "Erreur récupération user après login : {}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({
                        "error": "Impossible de récupérer l'utilisateur"
                    }))).into_response()
                }
            }
        }

        Ok(None) => {
            tracing::warn!(email = %body.email, "Email introuvable");
            (StatusCode::UNAUTHORIZED, Json(json!({
                "error": "Email ou mot de passe incorrect"
            }))).into_response()
        }

        Err(e) => {
            tracing::error!("Erreur DB lors du login : {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({
                "error": "Une erreur est survenue"
            }))).into_response()
        }
    }
}

pub fn generate_token(user_id: String, role: String, secret: &str) -> String {
    let expiration = Utc::now()
        .checked_add_signed(Duration::days(30))
        .expect("Overflow date impossible")
        .timestamp() as usize;

    let claims = Claims { sub: user_id, exp: expiration, role: Some(role) };
    encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_bytes()))
        .expect("Erreur encodage JWT — clé invalide")
}