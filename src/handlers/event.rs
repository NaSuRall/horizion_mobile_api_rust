use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;
use axum::response::IntoResponse;
use chrono::Datelike;
use serde_json::json;
use crate::config::AppState;
use crate::errors::ApiError;
use crate::models::Event;
use crate::utils::extract_user_id;

pub async fn get_events(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    extract_user_id(&headers, &state.jwt_secret)?;

    let events = sqlx::query_as::<_, Event>(
        "SELECT id, title, location, badge, badge_type, event_date FROM events ORDER BY event_date ASC"
    )
    .fetch_all(&state.db)
    .await?;

    let result: Vec<_> = events.iter().map(|e| json!({
        "id":        e.id,
        "title":     e.title,
        "location":  e.location,
        "badge":     e.badge,
        "badgeType": e.badge_type,
        "day":   e.event_date.day(),
        "month": e.event_date.month(),
        "year":  e.event_date.year(),
    })).collect();

    Ok(Json(json!({ "events": result })))
}
