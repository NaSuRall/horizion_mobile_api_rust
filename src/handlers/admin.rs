use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use axum::response::IntoResponse;
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::Deserialize;
use serde_json::json;
use sqlx::Row;
use uuid::Uuid;
use crate::config::AppState;
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

pub async fn admin_scan(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<AdminScanRequest>,
) -> impl IntoResponse {
    if let Err(r) = extract_admin_user_id(&headers) {
        return r;
    }

    if body.amount <= 0 {
        return (StatusCode::BAD_REQUEST, Json(json!({ "error": "Le montant doit être positif" }))).into_response();
    }

    let secret = std::env::var("JWT_SECRET").expect("JWT_SECRET manquant");
    let customer_id = match decode::<Claims>(
        &body.qr_token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::new(Algorithm::HS256),
    ) {
        Ok(data) => match Uuid::parse_str(&data.claims.sub) {
            Ok(id) => id,
            Err(_) => return (StatusCode::BAD_REQUEST, Json(json!({ "error": "Token QR invalide" }))).into_response(),
        },
        Err(_) => return (StatusCode::UNAUTHORIZED, Json(json!({ "error": "QR code invalide ou expiré" }))).into_response(),
    };

    match sqlx::query("UPDATE users SET point = point + ? WHERE id = ?")
        .bind(body.amount)
        .bind(customer_id)
        .execute(&state.db)
        .await
    {
        Ok(r) if r.rows_affected() == 0 => {
            return (StatusCode::NOT_FOUND, Json(json!({ "error": "Utilisateur introuvable" }))).into_response();
        }
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Erreur base de données" }))).into_response(),
        Ok(_) => {}
    }

    let row = match sqlx::query("SELECT point FROM users WHERE id = ?")
        .bind(customer_id)
        .fetch_one(&state.db)
        .await
    {
        Ok(r) => r,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Erreur base de données" }))).into_response(),
    };

    let new_total: i32 = row.try_get::<Option<i32>, _>("point").ok().flatten().unwrap_or(0);
    let new_rank = calculate_rank(new_total);

    let _ = sqlx::query("UPDATE users SET `rank` = ? WHERE id = ?")
        .bind(new_rank)
        .bind(customer_id)
        .execute(&state.db)
        .await;

    let tx_id = Uuid::new_v4();
    let label = format!("Achat en magasin : {}€", body.amount);
    let _ = sqlx::query("INSERT INTO transactions (id, user_id, points, label) VALUES (?, ?, ?, ?)")
        .bind(tx_id)
        .bind(customer_id)
        .bind(body.amount)
        .bind(&label)
        .execute(&state.db)
        .await;

    (StatusCode::OK, Json(json!({
        "status":  "success",
        "message": format!("{} points ajoutés", body.amount),
        "points":  new_total,
        "rank":    new_rank,
    }))).into_response()
}

pub async fn admin_validate_redemption(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<ValidateRedemptionRequest>,
) -> impl IntoResponse {
    if let Err(r) = extract_admin_user_id(&headers) {
        return r;
    }

    let result = sqlx::query(
        "SELECT r.code, r.status, rw.name AS reward_name
         FROM redemptions r
         JOIN rewards rw ON r.reward_id = rw.id
         WHERE r.code = ?",
    )
    .bind(&body.code)
    .fetch_optional(&state.db)
    .await;

    match result {
        Ok(None) => (StatusCode::NOT_FOUND, Json(json!({ "error": "Code introuvable" }))).into_response(),
        Err(_)   => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Erreur base de données" }))).into_response(),
        Ok(Some(row)) => {
            let status: String      = row.try_get("status").unwrap_or_default();
            let reward_name: String = row.try_get("reward_name").unwrap_or_default();

            match status.as_str() {
                "used"    => (StatusCode::CONFLICT, Json(json!({ "error": "Code déjà utilisé" }))).into_response(),
                "expired" => (StatusCode::GONE,     Json(json!({ "error": "Code expiré" }))).into_response(),
                _ => {
                    let _ = sqlx::query(
                        "UPDATE redemptions SET status = 'used', used_at = NOW() WHERE code = ?",
                    )
                    .bind(&body.code)
                    .execute(&state.db)
                    .await;

                    (StatusCode::OK, Json(json!({
                        "status":  "success",
                        "message": format!("Code validé : {}", reward_name),
                        "reward":  reward_name,
                    }))).into_response()
                }
            }
        }
    }
}
