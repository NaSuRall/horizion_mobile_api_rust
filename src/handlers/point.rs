use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::FromRow;
use uuid::Uuid;
use crate::config::AppState;
use crate::models::{RegisterUser, User};

#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct PostPoint {
    pub id: Uuid,
    pub point: i32 ,
}

pub async fn send_point(
    State(state): State<AppState>,
    Json(body): Json<PostPoint>
) -> Json<Value> {

    let result = sqlx::query_as::<_, User>(
        "SELECT *  FROM users WHERE id = ?"
    )
        .bind(body.id)
        .fetch_one(&state.db)
        .await;

    match result {
        Ok(_) => Json(json!({
            "message": "Points Ajouter avec Success",
            "status": "success"
        })),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

pub async fn get_point(){
    println!("prendre les points");
}