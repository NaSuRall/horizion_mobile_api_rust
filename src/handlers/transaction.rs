use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;
use axum::response::IntoResponse;
use serde_json::json;
use crate::config::AppState;
use crate::errors::ApiError;
use crate::models::Transaction;
use crate::utils::extract_user_id;

pub async fn get_user_transactions(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = extract_user_id(&headers, &state.jwt_secret)?;

    let rows = sqlx::query_as::<_, Transaction>(
        "SELECT id, user_id, points, label, created_at FROM transactions WHERE user_id = ? ORDER BY created_at DESC"
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await?;

    let transactions: Vec<_> = rows.iter().map(|t| json!({
        "id":     t.id.to_string(),
        "points": t.points,
        "label":  t.label,
        "date":   t.created_at.format("%d/%m/%Y").to_string(),
    })).collect();

    Ok(Json(json!({ "transactions": transactions })))
}
