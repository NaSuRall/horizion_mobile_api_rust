use axum::extract::State;
use axum::Json;
use axum::response::IntoResponse;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;
use crate::config::AppState;
use crate::errors::ApiError;
use crate::handlers::login::{generate_access_token, hash_refresh_token, issue_refresh_token};

#[derive(Deserialize)]
pub struct TokenBody {
    pub refresh_token: String,
}

/// Rotation : valide le refresh token, révoque l'ancien, émet une nouvelle paire.
/// Détecte le reuse → révoque toute la chaîne si un token déjà utilisé est rejoué.
pub async fn refresh_token(
    State(state): State<AppState>,
    Json(body): Json<TokenBody>,
) -> Result<impl IntoResponse, ApiError> {
    let token_hash = hash_refresh_token(&body.refresh_token);

    let row: Option<(Uuid, Uuid, DateTime<Utc>, Option<DateTime<Utc>>)> = sqlx::query_as(
        "SELECT id, user_id, expires_at, revoked_at
         FROM refresh_tokens
         WHERE token_hash = ?",
    )
    .bind(&token_hash)
    .fetch_optional(&state.db)
    .await?;

    let (token_id, user_id, expires_at, revoked_at) = row.ok_or_else(|| {
        tracing::warn!("Refresh token inconnu présenté");
        ApiError::Unauthorized
    })?;

    // Détection de reuse : si le token a déjà été révoqué (rotation antérieure),
    // c'est probablement un token volé. On invalide TOUTE la chaîne de cet user
    // pour forcer une reconnexion partout.
    if revoked_at.is_some() {
        tracing::warn!(user_id = %user_id, "Refresh token déjà utilisé → revoke all (suspicion de vol)");
        sqlx::query("UPDATE refresh_tokens SET revoked_at = NOW() WHERE user_id = ? AND revoked_at IS NULL")
            .bind(user_id)
            .execute(&state.db)
            .await?;
        return Err(ApiError::Unauthorized);
    }

    if expires_at < Utc::now() {
        tracing::info!(user_id = %user_id, "Refresh token expiré");
        return Err(ApiError::Unauthorized);
    }

    // Récupère le role pour le nouveau access token.
    let role: String = sqlx::query_scalar("SELECT `role` FROM users WHERE id = ?")
        .bind(user_id)
        .fetch_one(&state.db)
        .await?;

    let mut tx = state.db.begin().await?;

    sqlx::query("UPDATE refresh_tokens SET revoked_at = NOW() WHERE id = ?")
        .bind(token_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    // Émet la nouvelle paire après la révocation (pas dans la même tx pour ne
    // pas garder un INSERT pending si la génération échoue).
    let new_access  = generate_access_token(user_id.to_string(), role, &state.jwt_secret)?;
    let new_refresh = issue_refresh_token(&state.db, user_id).await?;

    tracing::info!(user_id = %user_id, "Refresh token rotated");

    Ok(Json(json!({
        "access_token":  new_access,
        "refresh_token": new_refresh,
    })))
}

/// Révoque le refresh token courant. Idempotent.
pub async fn logout(
    State(state): State<AppState>,
    Json(body): Json<TokenBody>,
) -> Result<impl IntoResponse, ApiError> {
    let token_hash = hash_refresh_token(&body.refresh_token);

    sqlx::query(
        "UPDATE refresh_tokens SET revoked_at = NOW()
         WHERE token_hash = ? AND revoked_at IS NULL",
    )
    .bind(&token_hash)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "status": "success" })))
}
