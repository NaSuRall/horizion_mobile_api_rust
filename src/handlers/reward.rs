use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use axum::response::IntoResponse;
use serde_json::json;
use sqlx::Row;
use uuid::Uuid;
use crate::config::AppState;
use crate::models::Reward;
use crate::utils::{calculate_rank, extract_user_id};

pub async fn get_rewards(State(state): State<AppState>) -> impl IntoResponse {
    match sqlx::query_as::<_, Reward>(
        "SELECT id, name, description, image_url, point_cost, stock, active
         FROM rewards WHERE active = 1 ORDER BY point_cost ASC",
    )
    .fetch_all(&state.db)
    .await
    {
        Ok(rewards) => (StatusCode::OK, Json(json!(rewards))).into_response(),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Erreur base de données" }))).into_response(),
    }
}

pub async fn redeem_reward(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(reward_id): Path<i32>,
) -> impl IntoResponse {
    let user_id = match extract_user_id(&headers) {
        Ok(id) => id,
        Err(r) => return r,
    };

    let reward = match sqlx::query_as::<_, Reward>(
        "SELECT id, name, description, image_url, point_cost, stock, active
         FROM rewards WHERE id = ? AND active = 1",
    )
    .bind(reward_id)
    .fetch_optional(&state.db)
    .await
    {
        Ok(Some(r)) => r,
        Ok(None)    => return (StatusCode::NOT_FOUND,    Json(json!({ "error": "Récompense introuvable" }))).into_response(),
        Err(_)      => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Erreur base de données" }))).into_response(),
    };

    if reward.stock == 0 {
        return (StatusCode::CONFLICT, Json(json!({ "error": "Récompense épuisée" }))).into_response();
    }

    let user_points: i32 = match sqlx::query("SELECT point FROM users WHERE id = ?")
        .bind(user_id)
        .fetch_optional(&state.db)
        .await
    {
        Ok(Some(row)) => row.try_get::<Option<i32>, _>("point").ok().flatten().unwrap_or(0),
        _ => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Erreur base de données" }))).into_response(),
    };

    if user_points < reward.point_cost {
        return (StatusCode::PAYMENT_REQUIRED, Json(json!({
            "error":     "Points insuffisants",
            "required":  reward.point_cost,
            "available": user_points,
        }))).into_response();
    }

    let code = generate_code();
    let new_points = user_points - reward.point_cost;
    let new_rank = calculate_rank(new_points);

    let _ = sqlx::query("UPDATE users SET point = point - ?, `rank` = ? WHERE id = ?")
        .bind(reward.point_cost)
        .bind(new_rank)
        .bind(user_id)
        .execute(&state.db)
        .await;

    let redemption_id = Uuid::new_v4();
    let _ = sqlx::query(
        "INSERT INTO redemptions (id, user_id, reward_id, code) VALUES (?, ?, ?, ?)",
    )
    .bind(redemption_id)
    .bind(user_id)
    .bind(reward_id)
    .bind(&code)
    .execute(&state.db)
    .await;

    if reward.stock > 0 {
        let _ = sqlx::query("UPDATE rewards SET stock = stock - 1 WHERE id = ?")
            .bind(reward_id)
            .execute(&state.db)
            .await;
    }

    let tx_id = Uuid::new_v4();
    let label = format!("Échange : {}", reward.name);
    let _ = sqlx::query(
        "INSERT INTO transactions (id, user_id, points, label) VALUES (?, ?, ?, ?)",
    )
    .bind(tx_id)
    .bind(user_id)
    .bind(-reward.point_cost)
    .bind(&label)
    .execute(&state.db)
    .await;

    (StatusCode::OK, Json(json!({
        "code":             code,
        "reward_name":      reward.name,
        "points_spent":     reward.point_cost,
        "points_remaining": new_points,
        "rank":             new_rank,
    }))).into_response()
}

pub async fn get_user_redemptions(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let user_id = match extract_user_id(&headers) {
        Ok(id) => id,
        Err(r) => return r,
    };

    match sqlx::query(
        "SELECT r.code, r.status, rw.name AS reward_name,
                DATE_FORMAT(r.created_at, '%d/%m/%Y') AS created_at
         FROM redemptions r
         JOIN rewards rw ON r.reward_id = rw.id
         WHERE r.user_id = ?
         ORDER BY r.created_at DESC",
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await
    {
        Ok(rows) => {
            let data: Vec<serde_json::Value> = rows
                .iter()
                .map(|row| json!({
                    "code":        row.try_get::<String, _>("code").unwrap_or_default(),
                    "status":      row.try_get::<String, _>("status").unwrap_or_default(),
                    "reward_name": row.try_get::<String, _>("reward_name").unwrap_or_default(),
                    "created_at":  row.try_get::<String, _>("created_at").unwrap_or_default(),
                }))
                .collect();
            (StatusCode::OK, Json(json!(data))).into_response()
        }
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Erreur base de données" }))).into_response(),
    }
}

fn generate_code() -> String {
    let id = Uuid::new_v4();
    let bytes = id.as_bytes();
    let chars: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let part1: String = bytes[0..4].iter().map(|b| chars[(*b as usize) % chars.len()] as char).collect();
    let part2: String = bytes[4..8].iter().map(|b| chars[(*b as usize) % chars.len()] as char).collect();
    format!("HRZ-{}-{}", part1, part2)
}
