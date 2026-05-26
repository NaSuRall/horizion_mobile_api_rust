use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::{rand_core::OsRng, SaltString}};
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use axum::response::IntoResponse;
use serde::Deserialize;
use serde_json::json;
use sqlx::Row;
use uuid::Uuid;
use crate::config::AppState;
use crate::errors::ApiError;
use crate::utils::extract_user_id;

pub async fn get_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = extract_user_id(&headers, &state.jwt_secret)?;

    if user_id != id {
        return Err(ApiError::Forbidden);
    }

    let user = sqlx::query_as::<_, crate::models::User>(
        "SELECT id, last_name, first_name, pseudo, email, password, birth_date, phone, pp, point, `rank`, `role` FROM users WHERE id = ?"
    )
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(user))
}

#[derive(Deserialize)]
pub struct UpdateUserRequest {
    pub email:            Option<String>,
    pub password:         Option<String>,
    pub current_password: Option<String>,
}

pub async fn put_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateUserRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = extract_user_id(&headers, &state.jwt_secret)?;

    if user_id != id {
        return Err(ApiError::Forbidden);
    }

    if body.email.is_none() && body.password.is_none() {
        return Err(ApiError::BadRequest("Aucun champ à modifier".into()));
    }

    if let Some(email) = &body.email {
        if !email.contains('@') || email.len() > 255 {
            return Err(ApiError::BadRequest("Adresse email invalide".into()));
        }
    }

    if let Some(password) = &body.password {
        if password.len() < 8 || password.len() > 72 {
            return Err(ApiError::BadRequest(
                "Le mot de passe doit contenir entre 8 et 72 caractères".into(),
            ));
        }
    }

    let new_hash: Option<String> = if let Some(new_pw) = &body.password {
        let current_pw = body
            .current_password
            .as_ref()
            .ok_or_else(|| ApiError::BadRequest("Mot de passe actuel requis".into()))?;

        let row = sqlx::query("SELECT password FROM users WHERE id = ?")
            .bind(id)
            .fetch_one(&state.db)
            .await?;

        let current_hash: String = row.try_get("password").unwrap_or_default();
        let parsed = PasswordHash::new(&current_hash).map_err(|_| ApiError::Internal)?;

        if Argon2::default()
            .verify_password(current_pw.as_bytes(), &parsed)
            .is_err()
        {
            return Err(ApiError::Unauthorized);
        }

        let salt = SaltString::generate(&mut OsRng);
        Some(
            Argon2::default()
                .hash_password(new_pw.as_bytes(), &salt)
                .map_err(|_| ApiError::Internal)?
                .to_string(),
        )
    } else {
        None
    };

    let mut set_parts: Vec<&str> = Vec::new();
    if new_hash.is_some()   { set_parts.push("`password` = ?"); }
    if body.email.is_some() { set_parts.push("`email` = ?");    }

    let sql = format!("UPDATE users SET {} WHERE id = ?", set_parts.join(", "));
    let mut q = sqlx::query(&sql);
    if let Some(h) = &new_hash   { q = q.bind(h); }
    if let Some(e) = &body.email { q = q.bind(e); }
    q = q.bind(id);

    q.execute(&state.db).await?;

    Ok(Json(json!({ "status": "success", "message": "Profil mis à jour" })))
}
