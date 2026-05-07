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

#[derive(Deserialize)]
pub struct AdminCustomerInfoRequest {
    pub qr_token: String,
}

pub async fn admin_customer_info(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<AdminCustomerInfoRequest>,
) -> impl IntoResponse {
    if let Err(r) = extract_admin_user_id(&headers, &state.jwt_secret) {
        return r;
    }

    let customer_id = match decode::<Claims>(
        &body.qr_token,
        &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
        &Validation::new(Algorithm::HS256),
    ) {
        Ok(data) => match Uuid::parse_str(&data.claims.sub) {
            Ok(id) => id,
            Err(_) => return (StatusCode::BAD_REQUEST, Json(json!({ "error": "Token QR invalide" }))).into_response(),
        },
        Err(_) => return (StatusCode::UNAUTHORIZED, Json(json!({ "error": "QR code invalide ou expiré" }))).into_response(),
    };

    let row = match sqlx::query(
        "SELECT first_name, last_name, email, point, `rank` FROM users WHERE id = ?",
    )
    .bind(customer_id)
    .fetch_one(&state.db)
    .await
    {
        Ok(r) => r,
        Err(_) => return (StatusCode::NOT_FOUND, Json(json!({ "error": "Utilisateur introuvable" }))).into_response(),
    };

    let first_name: String = row.try_get("first_name").unwrap_or_default();
    let last_name:  String = row.try_get("last_name").unwrap_or_default();
    let email:      String = row.try_get("email").unwrap_or_default();
    let point:      i32    = row.try_get::<Option<i32>, _>("point").ok().flatten().unwrap_or(0);
    let rank:       String = row.try_get("rank").unwrap_or_else(|_| "Bronze".to_string());

    (StatusCode::OK, Json(json!({
        "user_id":    customer_id.to_string(),
        "first_name": first_name,
        "last_name":  last_name,
        "email":      email,
        "points":     point,
        "rank":       rank,
    }))).into_response()
}

pub async fn admin_scan(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<AdminScanRequest>,
) -> impl IntoResponse {
    if let Err(r) = extract_admin_user_id(&headers, &state.jwt_secret) {
        return r;
    }

    if body.amount <= 0 || body.amount > 10_000 {
        return (StatusCode::BAD_REQUEST, Json(json!({
            "error": "Le montant doit être compris entre 1 et 10 000"
        }))).into_response();
    }

    let customer_id = match decode::<Claims>(
        &body.qr_token,
        &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
        &Validation::new(Algorithm::HS256),
    ) {
        Ok(data) => match Uuid::parse_str(&data.claims.sub) {
            Ok(id) => id,
            Err(_) => {
                tracing::warn!("admin_scan : UUID invalide dans le token QR");
                return (StatusCode::BAD_REQUEST, Json(json!({ "error": "Token QR invalide" }))).into_response();
            }
        },
        Err(e) => {
            tracing::warn!("admin_scan : QR token invalide ou expiré : {}", e);
            return (StatusCode::UNAUTHORIZED, Json(json!({ "error": "QR code invalide ou expiré" }))).into_response();
        }
    };

    match sqlx::query("UPDATE users SET point = point + ? WHERE id = ?")
        .bind(body.amount)
        .bind(customer_id)
        .execute(&state.db)
        .await
    {
        Ok(r) if r.rows_affected() == 0 => {
            tracing::warn!(customer_id = %customer_id, "admin_scan : client introuvable");
            return (StatusCode::NOT_FOUND, Json(json!({ "error": "Utilisateur introuvable" }))).into_response();
        }
        Err(e) => {
            tracing::error!(customer_id = %customer_id, "admin_scan : erreur DB update points : {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Erreur base de données" }))).into_response();
        }
        Ok(_) => {}
    }

    let row = match sqlx::query("SELECT point FROM users WHERE id = ?")
        .bind(customer_id)
        .fetch_one(&state.db)
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(customer_id = %customer_id, "admin_scan : erreur DB lecture points : {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Erreur base de données" }))).into_response();
        }
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

    tracing::info!(customer_id = %customer_id, amount = body.amount, total = new_total, rank = new_rank, "Admin scan : points ajoutés");

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
    if let Err(r) = extract_admin_user_id(&headers, &state.jwt_secret) {
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
                "used" => {
                    tracing::warn!(code = %body.code, "Validation code déjà utilisé");
                    (StatusCode::CONFLICT, Json(json!({ "error": "Code déjà utilisé" }))).into_response()
                }
                "expired" => {
                    tracing::warn!(code = %body.code, "Validation code expiré");
                    (StatusCode::GONE, Json(json!({ "error": "Code expiré" }))).into_response()
                }
                _ => {
                    let _ = sqlx::query(
                        "UPDATE redemptions SET status = 'used', used_at = NOW() WHERE code = ?",
                    )
                    .bind(&body.code)
                    .execute(&state.db)
                    .await;

                    tracing::info!(code = %body.code, reward = %reward_name, "Code de rédemption validé");

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