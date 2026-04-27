use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::{rand_core::OsRng, SaltString}};
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use axum::response::IntoResponse;
use serde::Deserialize;
use serde_json::json;
use sqlx::Row;
use uuid::Uuid;
use crate::config::AppState;
use crate::utils::extract_user_id;

pub async fn get_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let user_id = match extract_user_id(&headers, &state.jwt_secret) {
        Ok(uid) => uid,
        Err(r)  => return r,
    };

    if user_id != id {
        return (StatusCode::FORBIDDEN, Json(json!({ "error": "Accès interdit" }))).into_response();
    }

    match sqlx::query_as::<_, crate::models::User>(
        "SELECT id, last_name, first_name, pseudo, email, password, birth_date, phone, pp, point, `rank`, `role` FROM users WHERE id = ?"
    )
    .bind(id)
    .fetch_one(&state.db)
    .await
    {
        Ok(user) => (StatusCode::OK, Json(user)).into_response(),
        Err(sqlx::Error::RowNotFound) => (StatusCode::NOT_FOUND, Json(json!({ "error": "Utilisateur introuvable" }))).into_response(),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Erreur base de données" }))).into_response(),
    }
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
) -> impl IntoResponse {
    let user_id = match extract_user_id(&headers, &state.jwt_secret) {
        Ok(uid) => uid,
        Err(r)  => return r,
    };

    if user_id != id {
        return (StatusCode::FORBIDDEN, Json(json!({ "error": "Accès interdit" }))).into_response();
    }

    if body.email.is_none() && body.password.is_none() {
        return (StatusCode::BAD_REQUEST, Json(json!({ "error": "Aucun champ à modifier" }))).into_response();
    }

    if let Some(email) = &body.email {
        if !email.contains('@') || email.len() > 255 {
            return (StatusCode::BAD_REQUEST, Json(json!({ "error": "Adresse email invalide" }))).into_response();
        }
    }

    if let Some(password) = &body.password {
        if password.len() < 8 || password.len() > 72 {
            return (StatusCode::BAD_REQUEST, Json(json!({ "error": "Le mot de passe doit contenir entre 8 et 72 caractères" }))).into_response();
        }
    }

    let new_hash: Option<String> = if let Some(new_pw) = &body.password {
        let current_pw = match &body.current_password {
            Some(p) => p,
            None => return (StatusCode::BAD_REQUEST, Json(json!({ "error": "Mot de passe actuel requis" }))).into_response(),
        };

        let row = match sqlx::query("SELECT password FROM users WHERE id = ?")
            .bind(id)
            .fetch_one(&state.db)
            .await
        {
            Ok(r)  => r,
            Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Erreur base de données" }))).into_response(),
        };

        let current_hash: String = row.try_get("password").unwrap_or_default();
        let parsed = match PasswordHash::new(&current_hash) {
            Ok(h)  => h,
            Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Erreur interne" }))).into_response(),
        };

        if Argon2::default().verify_password(current_pw.as_bytes(), &parsed).is_err() {
            return (StatusCode::UNAUTHORIZED, Json(json!({ "error": "Mot de passe actuel incorrect" }))).into_response();
        }

        let salt = SaltString::generate(&mut OsRng);
        match Argon2::default().hash_password(new_pw.as_bytes(), &salt) {
            Ok(h)  => Some(h.to_string()),
            Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Erreur hash" }))).into_response(),
        }
    } else {
        None
    };

    let mut set_parts: Vec<&str> = Vec::new();
    if new_hash.is_some()     { set_parts.push("`password` = ?"); }
    if body.email.is_some()   { set_parts.push("`email` = ?");    }

    let sql = format!("UPDATE users SET {} WHERE id = ?", set_parts.join(", "));
    let mut q = sqlx::query(&sql);
    if let Some(h) = &new_hash   { q = q.bind(h); }
    if let Some(e) = &body.email { q = q.bind(e); }
    q = q.bind(id);

    match q.execute(&state.db).await {
        Ok(_) => (StatusCode::OK, Json(json!({ "status": "success", "message": "Profil mis à jour" }))).into_response(),
        Err(e) if e.to_string().contains("Duplicate entry") => {
            (StatusCode::CONFLICT, Json(json!({ "error": "Cet email est déjà utilisé" }))).into_response()
        }
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Erreur base de données" }))).into_response(),
    }
}