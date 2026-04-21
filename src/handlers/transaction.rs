use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use axum::response::IntoResponse;
use serde_json::json;
use crate::config::AppState;
use crate::models::Transaction;
use crate::utils::extract_user_id;

pub async fn get_user_transactions(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let user_id = match extract_user_id(&headers) {
        Ok(id) => id,
        Err(r) => return r,
    };

    let rows = match sqlx::query_as::<_, Transaction>(
        "SELECT id, user_id, points, label, created_at FROM transactions WHERE user_id = ? ORDER BY created_at DESC"
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await
    {
        Ok(rows) => rows,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Erreur base de données" }))).into_response(),
    };

    let transactions: Vec<_> = rows.iter().map(|t| json!({
        "id":     t.id.to_string(),
        "points": t.points,
        "label":  t.label,
        "date":   t.created_at.format("%d/%m/%Y").to_string(),
    })).collect();

    (StatusCode::OK, Json(json!({ "transactions": transactions }))).into_response()
}
