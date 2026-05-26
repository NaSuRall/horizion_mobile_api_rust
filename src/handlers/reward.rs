use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use axum::response::IntoResponse;
use serde_json::json;
use sqlx::Row;
use uuid::Uuid;
use crate::config::AppState;
use crate::errors::ApiError;
use crate::models::Reward;
use crate::utils::{calculate_rank, extract_user_id};

pub async fn get_rewards(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    extract_user_id(&headers, &state.jwt_secret)?;

    let rewards = sqlx::query_as::<_, Reward>(
        "SELECT id, name, description, image_url, point_cost, stock, active
         FROM rewards WHERE active = 1 ORDER BY point_cost ASC",
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!(rewards)))
}

pub async fn redeem_reward(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(reward_id): Path<i32>,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = extract_user_id(&headers, &state.jwt_secret)?;

    let mut tx = state.db.begin().await?;

    let reward = sqlx::query_as::<_, Reward>(
        "SELECT id, name, description, image_url, point_cost, stock, active
         FROM rewards WHERE id = ? AND active = 1",
    )
    .bind(reward_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(ApiError::NotFound)?;

    // 1. Décrément atomique des points utilisateur (rejet si solde insuffisant)
    let points_update = sqlx::query(
        "UPDATE users SET point = point - ? WHERE id = ? AND point >= ?",
    )
    .bind(reward.point_cost)
    .bind(user_id)
    .bind(reward.point_cost)
    .execute(&mut *tx)
    .await?;

    if points_update.rows_affected() == 0 {
        // Désambiguïsation : utilisateur introuvable vs points insuffisants
        let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE id = ?)")
            .bind(user_id)
            .fetch_one(&mut *tx)
            .await?;
        if !exists {
            return Err(ApiError::NotFound);
        }
        // Lecture des points actuels uniquement pour le message d'erreur
        let current: i32 = sqlx::query_scalar("SELECT point FROM users WHERE id = ?")
            .bind(user_id)
            .fetch_one(&mut *tx)
            .await?;
        return Err(ApiError::BadRequest(format!(
            "Points insuffisants ({} requis, {} disponibles)",
            reward.point_cost, current
        )));
    }

    // 2. Décrément atomique du stock (rejet si épuisé entre-temps).
    //    stock = -1 signifie illimité — on ne décrémente pas mais on ne rejette pas.
    if reward.stock > 0 {
        let stock_update = sqlx::query(
            "UPDATE rewards SET stock = stock - 1 WHERE id = ? AND stock > 0",
        )
        .bind(reward_id)
        .execute(&mut *tx)
        .await?;

        if stock_update.rows_affected() == 0 {
            return Err(ApiError::Conflict("Récompense épuisée".into()));
        }
    }

    // 3. Calcul du nouveau rang à partir du solde post-décrément
    let new_points: i32 = sqlx::query_scalar("SELECT point FROM users WHERE id = ?")
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await?;
    let new_rank = calculate_rank(new_points);

    sqlx::query("UPDATE users SET `rank` = ? WHERE id = ?")
        .bind(new_rank)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    // 4. Génération du code + insertion redemption + transaction
    let code = generate_code();

    let redemption_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO redemptions (id, user_id, reward_id, code) VALUES (?, ?, ?, ?)",
    )
    .bind(redemption_id)
    .bind(user_id)
    .bind(reward_id)
    .bind(&code)
    .execute(&mut *tx)
    .await?;

    let tx_id = Uuid::new_v4();
    let label = format!("Échange : {}", reward.name);
    sqlx::query(
        "INSERT INTO transactions (id, user_id, points, label) VALUES (?, ?, ?, ?)",
    )
    .bind(tx_id)
    .bind(user_id)
    .bind(-reward.point_cost)
    .bind(&label)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(json!({
        "code":             code,
        "reward_name":      reward.name,
        "points_spent":     reward.point_cost,
        "points_remaining": new_points,
        "rank":             new_rank,
    })))
}

pub async fn get_user_redemptions(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = extract_user_id(&headers, &state.jwt_secret)?;

    let rows = sqlx::query(
        "SELECT r.code, r.status, rw.name AS reward_name,
                DATE_FORMAT(r.created_at, '%d/%m/%Y') AS created_at
         FROM redemptions r
         JOIN rewards rw ON r.reward_id = rw.id
         WHERE r.user_id = ?
         ORDER BY r.created_at DESC",
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await?;

    let data: Vec<serde_json::Value> = rows
        .iter()
        .map(|row| json!({
            "code":        row.try_get::<String, _>("code").unwrap_or_default(),
            "status":      row.try_get::<String, _>("status").unwrap_or_default(),
            "reward_name": row.try_get::<String, _>("reward_name").unwrap_or_default(),
            "created_at":  row.try_get::<String, _>("created_at").unwrap_or_default(),
        }))
        .collect();

    Ok(Json(json!(data)))
}

fn generate_code() -> String {
    let id = Uuid::new_v4();
    let bytes = id.as_bytes();
    let chars: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let part1: String = bytes[0..4].iter().map(|b| chars[(*b as usize) % chars.len()] as char).collect();
    let part2: String = bytes[4..8].iter().map(|b| chars[(*b as usize) % chars.len()] as char).collect();
    format!("HRZ-{}-{}", part1, part2)
}
