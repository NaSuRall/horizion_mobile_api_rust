use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use axum::response::IntoResponse;
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::Row;
use uuid::Uuid;
use crate::config::AppState;
use crate::models::Claims;
use crate::utils::{calculate_rank, extract_user_id};

#[derive(Debug, Serialize, Deserialize)]
pub struct PostPoint {
    pub id: Uuid,
    pub point: i32,
}

pub async fn send_point(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<PostPoint>,
) -> impl IntoResponse {
    if let Err(response) = extract_user_id(&headers, &state.jwt_secret) {
        return response;
    }

    if body.point <= 0 || body.point > 10_000 {
        return (StatusCode::BAD_REQUEST, Json(json!({
            "error": "Le montant doit être compris entre 1 et 10 000 points"
        }))).into_response();
    }

    let update_result = sqlx::query("UPDATE users SET point = point + ? WHERE id = ?")
        .bind(body.point)
        .bind(body.id)
        .execute(&state.db)
        .await;

    match update_result {
        Ok(r) if r.rows_affected() == 0 => {
            return (StatusCode::NOT_FOUND, Json(json!({ "error": "Utilisateur introuvable" }))).into_response();
        }
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Erreur base de données" }))).into_response();
        }
        Ok(_) => {}
    }

    let row = match sqlx::query("SELECT point FROM users WHERE id = ?")
        .bind(body.id)
        .fetch_one(&state.db)
        .await
    {
        Ok(r)  => r,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Erreur base de données" }))).into_response(),
    };

    let new_total: i32 = row.try_get::<Option<i32>, _>("point").ok().flatten().unwrap_or(0);
    let new_rank = calculate_rank(new_total);

    let _ = sqlx::query("UPDATE users SET `rank` = ? WHERE id = ?")
        .bind(new_rank)
        .bind(body.id)
        .execute(&state.db)
        .await;

    let tx_id = Uuid::new_v4();
    let label = format!("Ajout de {} points", body.point);
    let _ = sqlx::query("INSERT INTO transactions (id, user_id, points, label) VALUES (?, ?, ?, ?)")
        .bind(tx_id)
        .bind(body.id)
        .bind(body.point)
        .bind(&label)
        .execute(&state.db)
        .await;

    (StatusCode::OK, Json(json!({
        "status":  "success",
        "message": "Points ajoutés avec succès",
        "points":  new_total,
        "rank":    new_rank,
    }))).into_response()
}

pub async fn get_user_points(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let user_id = match extract_user_id(&headers, &state.jwt_secret) {
        Ok(id) => id,
        Err(r) => return r,
    };

    let row = match sqlx::query("SELECT point FROM users WHERE id = ?")
        .bind(user_id)
        .fetch_one(&state.db)
        .await
    {
        Ok(r)                         => r,
        Err(sqlx::Error::RowNotFound) => return (StatusCode::NOT_FOUND, Json(json!({ "error": "Utilisateur introuvable" }))).into_response(),
        Err(_)                        => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Erreur base de données" }))).into_response(),
    };

    let points: i32 = row.try_get::<Option<i32>, _>("point").ok().flatten().unwrap_or(0);
    let rank = calculate_rank(points);

    (StatusCode::OK, Json(json!({ "points": points, "rank": rank }))).into_response()
}

pub async fn get_qrcode_token(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let user_id = match extract_user_id(&headers, &state.jwt_secret) {
        Ok(id) => id,
        Err(r) => return r,
    };

    let expiration = Utc::now()
        .checked_add_signed(Duration::minutes(10))
        .expect("Overflow date impossible")
        .timestamp() as usize;

    let claims = Claims { sub: user_id.to_string(), exp: expiration, role: None };

    match encode(&Header::default(), &claims, &EncodingKey::from_secret(state.jwt_secret.as_bytes())) {
        Ok(token) => (StatusCode::OK, Json(json!({ "token": token, "expires_in": 600 }))).into_response(),
        Err(_)    => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Erreur génération token" }))).into_response(),
    }
}