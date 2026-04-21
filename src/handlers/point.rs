use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use axum::response::IntoResponse;
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::Row;
use uuid::Uuid;
use crate::config::AppState;
use crate::models::Claims;

#[derive(Debug, Serialize, Deserialize)]
pub struct PostPoint {
    pub id: Uuid,
    pub point: i32,
}


fn calculate_rank(points: i32) -> &'static str {
    match points {
        p if p >= 5000 => "Diamond",
        p if p >= 2500 => "Platine",
        p if p >= 1000 => "Gold",
        p if p >= 500  => "Silver",
        _              => "Bronze",
    }
}

fn extract_user_id(headers: &HeaderMap) -> Result<Uuid, axum::response::Response> {
    let auth_header = match headers.get("Authorization") {
        Some(val) => val.to_str().unwrap_or(""),
        None => return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Token manquant" })),
        ).into_response()),
    };

    let token = match auth_header.strip_prefix("Bearer ") {
        Some(t) => t,
        None => return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Format de token invalide" })),
        ).into_response()),
    };

    let secret = std::env::var("JWT_SECRET").expect("JWT_SECRET manquant dans .env");
    let claims = match decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::new(Algorithm::HS256),
    ) {
        Ok(data) => data.claims,
        Err(_) => return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Token invalide ou expiré" })),
        ).into_response()),
    };

    Uuid::parse_str(&claims.sub).map_err(|_| (
        StatusCode::BAD_REQUEST,
        Json(json!({ "error": "UUID invalide dans le token" })),
    ).into_response())
}

pub async fn send_point(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<PostPoint>,
) -> impl IntoResponse {
    if let Err(response) = extract_user_id(&headers) {
        return response;
    }

    let update_result = sqlx::query(
        "UPDATE users SET point = point + ? WHERE id = ?"
    )
    .bind(body.point)
    .bind(body.id)
    .execute(&state.db)
    .await;

    match update_result {
        Ok(r) if r.rows_affected() == 0 => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "Utilisateur introuvable" })),
            ).into_response();
        }
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Erreur base de données" })),
            ).into_response();
        }
        Ok(_) => {}
    }

    let row = match sqlx::query("SELECT point FROM users WHERE id = ?")
        .bind(body.id)
        .fetch_one(&state.db)
        .await
    {
        Ok(r) => r,
        Err(_) => return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "Erreur base de données" })),
        ).into_response(),
    };

    let new_total: i32 = row.try_get::<Option<i32>, _>("point").ok().flatten().unwrap_or(0);
    let new_rank = calculate_rank(new_total);

    let _ = sqlx::query("UPDATE users SET `rank` = ? WHERE id = ?")
        .bind(new_rank)
        .bind(body.id)
        .execute(&state.db)
        .await;

    (
        StatusCode::OK,
        Json(json!({
            "status": "success",
            "message": "Points ajoutés avec succès",
            "points": new_total,
            "rank": new_rank,
        })),
    ).into_response()
}

pub async fn get_user_points(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let user_id = match extract_user_id(&headers) {
        Ok(id) => id,
        Err(response) => return response,
    };

    let row = match sqlx::query("SELECT point FROM users WHERE id = ?")
        .bind(user_id)
        .fetch_one(&state.db)
        .await
    {
        Ok(r) => r,
        Err(sqlx::Error::RowNotFound) => return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Utilisateur introuvable" })),
        ).into_response(),
        Err(_) => return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "Erreur base de données" })),
        ).into_response(),
    };

    let points: i32 = row.try_get::<Option<i32>, _>("point").ok().flatten().unwrap_or(0);
    let rank = calculate_rank(points);

    (StatusCode::OK, Json(json!({ "points": points, "rank": rank }))).into_response()
}

pub async fn get_qrcode_token(
    State(_state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let user_id = match extract_user_id(&headers) {
        Ok(id) => id,
        Err(response) => return response,
    };

    let expiration = Utc::now()
        .checked_add_signed(Duration::minutes(10))
        .unwrap()
        .timestamp() as usize;

    let claims = Claims { sub: user_id.to_string(), exp: expiration };
    let secret = std::env::var("JWT_SECRET").expect("JWT_SECRET manquant dans .env");

    match encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_bytes())) {
        Ok(token) => (
            StatusCode::OK,
            Json(json!({ "token": token, "expires_in": 600 })),
        ).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "Erreur génération token" })),
        ).into_response(),
    }
}
