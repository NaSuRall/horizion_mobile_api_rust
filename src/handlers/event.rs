use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use axum::response::IntoResponse;
use chrono::Datelike;
use serde_json::json;
use crate::config::AppState;
use crate::models::Event;

pub async fn get_events(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let events = match sqlx::query_as::<_, Event>(
        "SELECT id, title, location, badge, badge_type, event_date FROM events ORDER BY event_date ASC"
    )
    .fetch_all(&state.db)
    .await
    {
        Ok(e)  => e,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Erreur base de données" }))).into_response(),
    };

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

    (StatusCode::OK, Json(json!({ "events": result }))).into_response()
}
