use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;
use axum::response::IntoResponse;
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::Deserialize;
use serde_json::json;
use sqlx::Row;
use uuid::Uuid;
use crate::config::AppState;
use crate::errors::ApiError;
use crate::models::Claims;
use crate::utils::{calculate_rank, extract_admin_user_id};

#[derive(Deserialize)]
pub struct AdminScanRequest {
    pub qr_token: String,
    pub amount: i32,
}

#[derive(Deserialize)]
pub struct ValidateRedemptionRequest {
    pub code: String,
}

#[derive(Deserialize)]
pub struct AdminCustomerInfoRequest {
    pub qr_token: String,
}

fn decode_qr_token(token: &str, secret: &str) -> Result<Uuid, ApiError> {
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::new(Algorithm::HS256),
    )
    .map_err(|e| {
        tracing::warn!("QR token invalide ou expiré : {}", e);
        ApiError::Unauthorized
    })?;

    Uuid::parse_str(&data.claims.sub).map_err(|_| ApiError::BadRequest("Token QR invalide".into()))
}

pub async fn admin_customer_info(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<AdminCustomerInfoRequest>,
) -> Result<impl IntoResponse, ApiError> {
    extract_admin_user_id(&headers, &state.jwt_secret)?;
    let customer_id = decode_qr_token(&body.qr_token, &state.jwt_secret)?;

    let row = sqlx::query(
        "SELECT first_name, last_name, email, point, `rank` FROM users WHERE id = ?",
    )
    .bind(customer_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::NotFound)?;

    let first_name: String = row.try_get("first_name").unwrap_or_default();
    let last_name:  String = row.try_get("last_name").unwrap_or_default();
    let email:      String = row.try_get("email").unwrap_or_default();
    let point:      i32    = row.try_get::<Option<i32>, _>("point").ok().flatten().unwrap_or(0);
    let rank:       String = row.try_get("rank").unwrap_or_else(|_| "Bronze".to_string());

    Ok(Json(json!({
        "user_id":    customer_id.to_string(),
        "first_name": first_name,
        "last_name":  last_name,
        "email":      email,
        "points":     point,
        "rank":       rank,
    })))
}

pub async fn admin_scan(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<AdminScanRequest>,
) -> Result<impl IntoResponse, ApiError> {
    extract_admin_user_id(&headers, &state.jwt_secret)?;

    if body.amount <= 0 || body.amount > 10_000 {
        return Err(ApiError::BadRequest(
            "Le montant doit être compris entre 1 et 10 000".into(),
        ));
    }

    let customer_id = decode_qr_token(&body.qr_token, &state.jwt_secret)?;

    let mut tx = state.db.begin().await?;

    let update = sqlx::query("UPDATE users SET point = point + ? WHERE id = ?")
        .bind(body.amount)
        .bind(customer_id)
        .execute(&mut *tx)
        .await?;

    if update.rows_affected() == 0 {
        tracing::warn!(customer_id = %customer_id, "admin_scan : client introuvable");
        return Err(ApiError::NotFound);
    }

    let row = sqlx::query("SELECT point FROM users WHERE id = ?")
        .bind(customer_id)
        .fetch_one(&mut *tx)
        .await?;

    let new_total: i32 = row.try_get::<Option<i32>, _>("point").ok().flatten().unwrap_or(0);
    let new_rank = calculate_rank(new_total);

    sqlx::query("UPDATE users SET `rank` = ? WHERE id = ?")
        .bind(new_rank)
        .bind(customer_id)
        .execute(&mut *tx)
        .await?;

    let tx_id = Uuid::new_v4();
    let label = format!("Achat en magasin : {}€", body.amount);
    sqlx::query("INSERT INTO transactions (id, user_id, points, label) VALUES (?, ?, ?, ?)")
        .bind(tx_id)
        .bind(customer_id)
        .bind(body.amount)
        .bind(&label)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    tracing::info!(customer_id = %customer_id, amount = body.amount, total = new_total, rank = new_rank, "Admin scan : points ajoutés");

    Ok(Json(json!({
        "status":  "success",
        "message": format!("{} points ajoutés", body.amount),
        "points":  new_total,
        "rank":    new_rank,
    })))
}

pub async fn admin_validate_redemption(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<ValidateRedemptionRequest>,
) -> Result<impl IntoResponse, ApiError> {
    extract_admin_user_id(&headers, &state.jwt_secret)?;

    let row = sqlx::query(
        "SELECT r.code, r.status, rw.name AS reward_name
         FROM redemptions r
         JOIN rewards rw ON r.reward_id = rw.id
         WHERE r.code = ?",
    )
    .bind(&body.code)
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::NotFound)?;

    let status: String      = row.try_get("status").unwrap_or_default();
    let reward_name: String = row.try_get("reward_name").unwrap_or_default();

    match status.as_str() {
        "used" => {
            tracing::warn!(code = %body.code, "Validation code déjà utilisé");
            Err(ApiError::Conflict("Code déjà utilisé".into()))
        }
        "expired" => {
            tracing::warn!(code = %body.code, "Validation code expiré");
            Err(ApiError::Conflict("Code expiré".into()))
        }
        _ => {
            sqlx::query(
                "UPDATE redemptions SET status = 'used', used_at = NOW() WHERE code = ?",
            )
            .bind(&body.code)
            .execute(&state.db)
            .await?;

            tracing::info!(code = %body.code, reward = %reward_name, "Code de rédemption validé");

            Ok(Json(json!({
                "status":  "success",
                "message": format!("Code validé : {}", reward_name),
                "reward":  reward_name,
            })))
        }
    }
}
