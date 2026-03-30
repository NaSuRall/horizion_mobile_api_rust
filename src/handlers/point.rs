use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use axum::response::IntoResponse;
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::FromRow;
use uuid::Uuid;
use crate::config::AppState;
use crate::models::{Claims, RegisterUser, User};

#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct PostPoint {
    pub id: Uuid,
    pub point: i32 ,
}

#[derive(Serialize)]
struct PointsResponse {
    points: i32,
}

pub async fn send_point(
    State(state): State<AppState>,
    Json(body): Json<PostPoint>
) -> Json<Value> {

    let result = sqlx::query_as::<_, User>(
        "SELECT *  FROM users WHERE id = ?"
    )
        .bind(body.id)
        .fetch_one(&state.db)
        .await;

    match result {
        Ok(_) => Json(json!({
            "message": "Points Ajouter avec Success",
            "status": "success"
        })),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

pub async fn get_user_points(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {

    let auth_header = match headers.get("Authorization") {
        Some(val) => val.to_str().unwrap_or(""),
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({ "error": "Token manquant" })),
            ).into_response();
        }
    };
    let token = match auth_header.strip_prefix("Bearer ") {
        Some(t) => t,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({ "error": "Format invalide" })),
            ).into_response();
        }
    };
    let secret = std::env::var("JWT_SECRET").expect("JWT_SECRET manquant dans .env");
    let claims = match decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::new(Algorithm::HS256),
    ) {
        Ok(data) => data.claims,
        Err(_) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({ "error": "Token invalide ou expiré" })),
            ).into_response();
        }
    };
    let user_id = match Uuid::parse_str(&claims.sub) {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "UUID invalide" })),
            ).into_response();
        }
    };
    let row = match sqlx::query!(
        "SELECT point FROM users WHERE id = ?",
        user_id
    )
        .fetch_one(&state.db)
        .await
    {
        Ok(row) => row,
        Err(sqlx::Error::RowNotFound) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "Utilisateur introuvable" })),
            ).into_response();
        }
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "Erreur base de données" })),
            ).into_response();
        }
    };

    (
        StatusCode::OK,
        Json(PointsResponse { points: row.point.expect("REASON") }),
    ).into_response()

}