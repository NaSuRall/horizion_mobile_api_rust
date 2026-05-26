use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;
use axum::response::IntoResponse;
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::Row;
use uuid::Uuid;
use crate::config::AppState;
use crate::errors::ApiError;
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
) -> Result<impl IntoResponse, ApiError> {
    extract_user_id(&headers, &state.jwt_secret)?;

    if body.point <= 0 || body.point > 10_000 {
        tracing::warn!(user_id = %body.id, points = body.point, "Montant de points hors limites");
        return Err(ApiError::BadRequest(
            "Le montant doit être compris entre 1 et 10 000 points".into(),
        ));
    }

    let update = sqlx::query("UPDATE users SET point = point + ? WHERE id = ?")
        .bind(body.point)
        .bind(body.id)
        .execute(&state.db)
        .await?;

    if update.rows_affected() == 0 {
        tracing::warn!(user_id = %body.id, "send_point : utilisateur introuvable");
        return Err(ApiError::NotFound);
    }

    let row = sqlx::query("SELECT point FROM users WHERE id = ?")
        .bind(body.id)
        .fetch_one(&state.db)
        .await?;

    let new_total: i32 = row.try_get::<Option<i32>, _>("point").ok().flatten().unwrap_or(0);
    let new_rank = calculate_rank(new_total);

    sqlx::query("UPDATE users SET `rank` = ? WHERE id = ?")
        .bind(new_rank)
        .bind(body.id)
        .execute(&state.db)
        .await?;

    let tx_id = Uuid::new_v4();
    let label = format!("Ajout de {} points", body.point);
    sqlx::query("INSERT INTO transactions (id, user_id, points, label) VALUES (?, ?, ?, ?)")
        .bind(tx_id)
        .bind(body.id)
        .bind(body.point)
        .bind(&label)
        .execute(&state.db)
        .await?;

    tracing::info!(user_id = %body.id, points = body.point, total = new_total, rank = new_rank, "Points ajoutés");

    Ok(Json(json!({
        "status":  "success",
        "message": "Points ajoutés avec succès",
        "points":  new_total,
        "rank":    new_rank,
    })))
}

pub async fn get_user_points(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = extract_user_id(&headers, &state.jwt_secret)?;

    let row = sqlx::query("SELECT point FROM users WHERE id = ?")
        .bind(user_id)
        .fetch_one(&state.db)
        .await?;

    let points: i32 = row.try_get::<Option<i32>, _>("point").ok().flatten().unwrap_or(0);
    let rank = calculate_rank(points);

    Ok(Json(json!({ "points": points, "rank": rank })))
}

pub async fn get_qrcode_token(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = extract_user_id(&headers, &state.jwt_secret)?;

    let expiration = Utc::now()
        .checked_add_signed(Duration::minutes(10))
        .ok_or(ApiError::Internal)?
        .timestamp() as usize;

    let claims = Claims { sub: user_id.to_string(), exp: expiration, role: None };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.jwt_secret.as_bytes()),
    )
    .map_err(|e| {
        tracing::error!("Erreur génération QR token : {}", e);
        ApiError::Internal
    })?;

    Ok(Json(json!({ "token": token, "expires_in": 600 })))
}
