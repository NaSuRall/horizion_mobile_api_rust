use axum::extract::State;
use axum::Json;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;
use crate::config::AppState;
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
) -> impl IntoResponse {
    // 1. Verify access_token with Google userinfo endpoint
    let client = reqwest::Client::new();
    let google_res = client
        .get("https://www.googleapis.com/oauth2/v3/userinfo")
        .bearer_auth(&body.access_token)
        .send()
        .await;

    let google_user = match google_res {
        Ok(r) if r.status().is_success() => match r.json::<GoogleUserInfo>().await {
            Ok(u) => u,
            Err(_) => return (StatusCode::UNAUTHORIZED, Json(json!({ "error": "Token Google invalide" }))).into_response(),
        },
        _ => return (StatusCode::UNAUTHORIZED, Json(json!({ "error": "Token Google invalide" }))).into_response(),
    };

    // 2. Find existing user by google_id or email
    let existing: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM users WHERE google_id = ? OR email = ? LIMIT 1",
    )
    .bind(&google_user.sub)
    .bind(&google_user.email)
    .fetch_optional(&state.db)
    .await
    .unwrap_or(None);

    let user_id = if let Some(id) = existing {
        // Link google_id if the account existed via email/password
        let _ = sqlx::query("UPDATE users SET google_id = ? WHERE id = ? AND google_id IS NULL")
            .bind(&google_user.sub)
            .bind(id)
            .execute(&state.db)
            .await;
        id
    } else {
        // Create new account from Google profile
        let new_id = Uuid::new_v4();
        let first_name = google_user.given_name.as_deref().unwrap_or("Utilisateur");
        let last_name  = google_user.family_name.as_deref().unwrap_or("");
        let pseudo     = google_user.email.split('@').next().unwrap_or("user");

        let insert = sqlx::query(
            "INSERT INTO users (id, last_name, first_name, pseudo, email, google_id, point, `rank`, `role`)
             VALUES (?, ?, ?, ?, ?, ?, 0, 'Bronze', 'user')",
        )
        .bind(new_id)
        .bind(last_name)
        .bind(first_name)
        .bind(pseudo)
        .bind(&google_user.email)
        .bind(&google_user.sub)
        .execute(&state.db)
        .await;

        match insert {
            Ok(_) => new_id,
            Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Erreur création du compte" }))).into_response(),
        }
    };

    // 3. Fetch full user + generate JWT
    let full_user = sqlx::query_as::<_, User>(
        "SELECT id, last_name, first_name, pseudo, email, password, birth_date, phone, pp, point, `rank`, `role`
         FROM users WHERE id = ?",
    )
    .bind(user_id)
    .fetch_one(&state.db)
    .await;

    match full_user {
        Ok(u) => {
            let token = generate_token(user_id.to_string(), u.role.clone());
            (StatusCode::OK, Json(json!({ "token": token, "user": u }))).into_response()
        }
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Erreur récupération utilisateur" }))).into_response(),
    }
}
