use axum::extract::State;
use axum::Json;
use axum::response::IntoResponse;
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;
use crate::config::AppState;
use crate::errors::ApiError;
use crate::models::user::User;
use super::login::generate_token;

#[derive(Deserialize)]
pub struct GoogleAuthRequest {
    pub access_token: String,
}

#[derive(Deserialize)]
struct GoogleUserInfo {
    sub: String,
    email: String,
    given_name: Option<String>,
    family_name: Option<String>,
}

pub async fn google_auth(
    State(state): State<AppState>,
    Json(body): Json<GoogleAuthRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let client = reqwest::Client::new();
    let google_res = client
        .get("https://www.googleapis.com/oauth2/v3/userinfo")
        .bearer_auth(&body.access_token)
        .send()
        .await
        .map_err(|e| {
            tracing::warn!("Erreur réseau Google userinfo : {}", e);
            ApiError::Unauthorized
        })?;

    if !google_res.status().is_success() {
        tracing::warn!(status = %google_res.status(), "Token Google refusé");
        return Err(ApiError::Unauthorized);
    }

    let google_user: GoogleUserInfo = google_res.json().await.map_err(|e| {
        tracing::warn!("Réponse Google userinfo invalide : {}", e);
        ApiError::Unauthorized
    })?;

    let existing: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM users WHERE google_id = ? OR email = ? LIMIT 1",
    )
    .bind(&google_user.sub)
    .bind(&google_user.email)
    .fetch_optional(&state.db)
    .await?;

    let user_id = if let Some(id) = existing {
        // Lien Google + email vérifié (validé par Google).
        sqlx::query(
            "UPDATE users SET google_id = COALESCE(google_id, ?), email_verified = 1 WHERE id = ?",
        )
        .bind(&google_user.sub)
        .bind(id)
        .execute(&state.db)
        .await?;
        id
    } else {
        let new_id = Uuid::new_v4();
        let first_name = google_user.given_name.as_deref().unwrap_or("Utilisateur");
        let last_name  = google_user.family_name.as_deref().unwrap_or("");
        let pseudo     = google_user.email.split('@').next().unwrap_or("user");

        sqlx::query(
            "INSERT INTO users (id, last_name, first_name, pseudo, email, google_id, point, `rank`, `role`, email_verified)
             VALUES (?, ?, ?, ?, ?, ?, 0, 'Bronze', 'user', 1)",
        )
        .bind(new_id)
        .bind(last_name)
        .bind(first_name)
        .bind(pseudo)
        .bind(&google_user.email)
        .bind(&google_user.sub)
        .execute(&state.db)
        .await?;

        new_id
    };

    let full_user = sqlx::query_as::<_, User>(
        "SELECT id, last_name, first_name, pseudo, email, password, birth_date, phone, pp, point, `rank`, `role`, email_verified
         FROM users WHERE id = ?",
    )
    .bind(user_id)
    .fetch_one(&state.db)
    .await?;

    let token = generate_token(user_id.to_string(), full_user.role.clone(), &state.jwt_secret)?;

    Ok(Json(json!({ "token": token, "user": full_user })))
}
